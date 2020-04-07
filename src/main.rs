//! nRF9160 Rust Demo
//!
//! This is a demo application for the nRF9160-DK. See the [README.md] file
//! for more details.
//!
//! Copyright (c) 42 Technology Ltd 2019
//! Licensed under the MIT or Apache-2.0 licences, at your option.

#![no_std]
#![no_main]
#![allow(deprecated)]

// ==========================================================================
//
// Modules and Crates
//
// ==========================================================================

extern crate cortex_m_rt as rt;
extern crate tinyrlibc;

#[cfg(not(any(feature = "nrf9160dk", feature = "icarus")))]
compile_error!("Must enable nrf9160dk or icarus features to select a board.");

#[cfg(feature = "nrf9160dk")]
extern crate nrf9160_dk_bsp as bsp;

#[cfg(feature = "icarus")]
extern crate actinius_icarus_bsp as bsp;

mod secrets;

// ==========================================================================
//
// Imports
//
// ==========================================================================

use core::fmt::Write;
use core::panic::PanicInfo;

use bsp::pac::interrupt;
use bsp::prelude::*;
use rt::entry;

// ==========================================================================
//
// Private Types
//
// ==========================================================================

/// Our menu system holds a context object for us, of this type.
struct Context {
	timer: bsp::hal::Timer<bsp::pac::TIMER0_NS>,
}

#[derive(Debug)]
enum Error {
	Nrfxlib(nrfxlib::Error),
	WriteError,
	ReadError,
}

// ==========================================================================
//
// Private Global Data
//
// ==========================================================================

/// This is the main menu
static ROOT_MENU: menu::Menu<Context> = menu::Menu {
	label: "root",
	items: &[
		&menu::Item {
			item_type: menu::ItemType::Callback(command_on),
			command: "on",
			help: Some("Power on modem"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_mode),
			command: "mode",
			help: Some("Get/set XSYSTEMMODE (NB-IOT, LTE-M, and/or GPS)"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_flight),
			command: "flight",
			help: Some("Enter flight mode"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_off),
			command: "off",
			help: Some("Power off modem"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_wait),
			command: "wait",
			help: Some("Wait for signal"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_stat),
			command: "stat",
			help: Some("Show registration and general modem status"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_get),
			command: "get",
			help: Some("Do an HTTP GET"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_store),
			command: "store",
			help: Some("Write the TLS keys to Flash"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_panic),
			command: "panic",
			help: Some("Deliberately crash"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_fix),
			command: "fix",
			help: Some("Get a GPS fix"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_go_at),
			command: "go_at",
			help: Some("Enter AT over UART mode"),
		},
		&menu::Item {
			item_type: menu::ItemType::Callback(command_go_at_fun),
			command: "AT+CFUN?",
			help: Some("Enter AT mode if an AT command is entered..."),
		},
	],
	entry: None,
	exit: None,
};

/// A UART we can access from anywhere (with run-time lock checking).
static GLOBAL_UART: spin::Mutex<Option<bsp::hal::uarte::Uarte<bsp::pac::UARTE0_NS>>> =
	spin::Mutex::new(None);

/// The tag we use for our crypto keys (which we pass when saving the keys to
/// flash and then also when opening a TLS socket).
const SECURITY_TAG: u32 = 0;

// ==========================================================================
//
// Macros
//
// ==========================================================================

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write as _;
            if let Some(ref mut uart) = *crate::GLOBAL_UART.lock() {
                let _err = write!(*uart, $($arg)*);
            }
        }
    };
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => {
        {
            use core::fmt::Write as _;
            if let Some(ref mut uart) = *crate::GLOBAL_UART.lock() {
                let _err = writeln!(*uart, $($arg)*);
            }
        }
    };
}

// ==========================================================================
//
// Public Functions and Impls
//
// ==========================================================================

#[entry]
fn main() -> ! {
	let mut board = bsp::Board::take().unwrap();

	#[cfg(feature = "nrf9160dk")]
	let (mut led1, mut led2) = (board.leds.led_1, board.leds.led_2);

	#[cfg(feature = "icarus")]
	let (mut led1, mut led2) = (board.leds.red, board.leds.green);

	board.NVIC.enable(bsp::pac::Interrupt::EGU1);
	board.NVIC.enable(bsp::pac::Interrupt::EGU2);
	// Enabled by bsd_init();
	// board.NVIC.enable(bsp::pac::Interrupt::IPC);
	// Only use top three bits, so shift by up by 8 - 3 = 5 bits
	unsafe {
		board.NVIC.set_priority(bsp::pac::Interrupt::EGU2, 4 << 5);
		board.NVIC.set_priority(bsp::pac::Interrupt::EGU1, 4 << 5);
		board.NVIC.set_priority(bsp::pac::Interrupt::IPC, 0 << 5);
	}

	*GLOBAL_UART.lock() = Some(board.cdc_uart);

	// Set one LED on so we know we're running
	led1.enable();

	println!("This is Rust on the nRF9160 LTE SiP");
	println!("Copyright (c) 42 Technology Ltd, 2019.");

	// Work around https://www.nordicsemi.com/DocLib/Content/Errata/nRF9160_EngA/latest/ERR/nRF9160/EngineeringA/latest/anomaly_160_17
	// *(volatile uint32_t *)0x40005C04 = 0x02ul;
	unsafe {
		core::ptr::write_volatile(0x4000_5C04 as *mut u32, 0x02);
	}

	// Start the Nordic library
	println!("Calling nrfxlib::init()...");
	nrfxlib::init();

	// Set another LED to we know the library has initialised
	led2.enable();

	// Start the menu system
	let mut buffer = [0u8; 64];
	let mut context = Context {
		timer: bsp::hal::timer::Timer::new(board.TIMER0_NS),
	};

	let mut runner = menu::Runner::new(&ROOT_MENU, &mut buffer, &mut context);

	loop {
		// Grab the UART and maybe get a character
		let maybe_c = if let Some(ref mut uart) = *crate::GLOBAL_UART.lock() {
			let mut uart_rx_buf = [0u8; 1];
			if uart.read(&mut uart_rx_buf).is_ok() {
				Some(uart_rx_buf[0])
			} else {
				None
			}
		} else {
			None
		};
		// If we did, give it to the menu (without holding the UART lock)
		if let Some(c) = maybe_c {
			runner.input_byte(c);
		}
	}
}

/// Interrupt Handler for LTE related hardware. Defer straight to the library.
#[interrupt]
fn EGU1() {
	nrfxlib::application_irq_handler();
	cortex_m::asm::sev();
}

/// Interrupt Handler for LTE related hardware. Defer straight to the library.
#[interrupt]
fn EGU2() {
	nrfxlib::trace_irq_handler();
	cortex_m::asm::sev();
}

/// Interrupt Handler for LTE related hardware. Defer straight to the library.
#[interrupt]
fn IPC() {
	nrfxlib::ipc_irq_handler();
	cortex_m::asm::sev();
}

/// Debug function our C code can use to print messages to the UART.
#[no_mangle]
unsafe extern "C" fn rust_print(data: *const u8) {
	extern "C" {
		fn strlen(s: *const u8) -> isize;
	}
	let len = strlen(data);
	let slice = core::slice::from_raw_parts(data, len as usize);
	let string = core::str::from_utf8_unchecked(slice);
	print!("{}", &string);
}

/// Called when our code panics.
#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	println!("{:?}", info);
	loop {
		cortex_m::asm::nop();
	}
}

// ==========================================================================
//
// Private Functions and Impls
//
// ==========================================================================

/// The modem starts up in the powered-off state. This turns it on.
fn command_on(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	println!("Configure GNSS antenna...");
	match nrfxlib::modem::configure_gnss_on_pca10090ns() {
		Ok(_) => {
			println!("GNSS antenna enabled.");
		}
		Err(e) => {
			println!("Error turning GNSS antenna on: {:?}", e);
		}
	}
	print!("Turning modem on...");
	match nrfxlib::modem::on() {
		Ok(_) => {
			println!("Modem now on.");
		}
		Err(e) => {
			println!("Error turning modem on: {:?}", e);
		}
	}
	println!("Opening socket...");
	let gnss = nrfxlib::gnss::GnssSocket::new().expect("GnssSocket::new");
	// Same as the Nordic demo app
	println!("Set fix interval to 1...");
	if let Err(e) = gnss.set_fix_interval(1) {
		println!(
			"Failed to set fix interval. GPS may be disabled - see 'mode'. Error {:?}",
			e
		);
		return;
	}
	println!("Set fix retry to 0...");
	if let Err(e) = gnss.set_fix_retry(0) {
		println!(
			"Failed to set fix retry. GPS may be disabled - see 'mode'. Error {:?}",
			e
		);
		return;
	}
	let mask = nrfxlib::gnss::NmeaMask::new();
	println!("Setting NMEA mask to {:?}", mask);
	if let Err(e) = gnss.set_nmea_mask(mask) {
		println!(
			"Failed to set NMEA mask. GPS may be disabled - see 'mode'. Error {:?}",
			e
		);
		return;
	}
	println!("Starting gnss...");
	if let Err(e) = gnss.start(nrfxlib::gnss::DeleteMask::new()) {
		println!(
			"Failed to start GPS. GPS may be disabled - see 'mode'. Error {:?}",
			e
		);
		return;
	}
	println!("GPS started OK.");
}

/// The modem starts up in the powered-off state. This turns it on.
fn command_mode(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	args: &str,
	_context: &mut Context,
) {
	let mut args_iter = args.split_whitespace();
	let _command = args_iter.next();
	let mut modes = (0, 0, 0);
	for arg in args_iter {
		if arg.eq_ignore_ascii_case("gps") {
			println!("Enabling GPS.");
			modes.2 = 1;
		} else if arg.eq_ignore_ascii_case("nbiot") || arg.eq_ignore_ascii_case("nb-iot") {
			println!("Enabling NB-IoT.");
			modes.1 = 1;
		} else if arg.eq_ignore_ascii_case("ltem") || arg.eq_ignore_ascii_case("lte-m") {
			println!("Enabling LTE-M.");
			modes.0 = 1;
		} else {
			println!("Don't understand argument {:?}.", arg);
			println!("Try 'nbiot', 'ltem' and/or 'gps'");
			return;
		}
	}
	// They've enabled something
	if modes != (0, 0, 0) {
		let mut command: heapless::String<heapless::consts::U32> = heapless::String::new();
		write!(
			command,
			"AT%XSYSTEMMODE={},{},{},0",
			modes.0, modes.1, modes.2
		)
		.unwrap();
		if let Err(e) = nrfxlib::at::send_at_command(&command, |_| {}) {
			println!("Err running {}: {:?}", command, e);
		}
	}

	if let Err(e) = nrfxlib::at::send_at_command("AT%XSYSTEMMODE?", |s| {
		println!("> {}", s);
		// TODO parse result here
	}) {
		println!("Err running AT%XSYSTEMMODE: {:?}", e);
	}
}

/// This puts the modem into flight mode. Needed if you want to fiddle with
/// some particular parameters that can't be set when it's on.
fn command_flight(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	print!("Taking modem offline...");
	match nrfxlib::modem::flight_mode() {
		Ok(_) => {
			println!("Modem now in flight mode.");
		}
		Err(e) => {
			println!("Error taking modem offline: {:?}", e);
		}
	}
}

/// Powers the modem right off.
fn command_off(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	print!("Turning modem off...");
	match nrfxlib::modem::off() {
		Ok(_) => {
			println!("Modem now off.");
		}
		Err(e) => {
			println!("Error turning modem off: {:?}", e);
		}
	}
}

/// Wait for signal
fn command_wait(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	print!("Waiting for signal...");
	match nrfxlib::modem::wait_for_lte() {
		Ok(_) => {
			println!("Modem now registered.");
		}
		Err(e) => {
			println!("Error getting registration: {:?}", e);
		}
	}
}

/// Show registration and general modem status.
fn command_stat(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	for cmd in &[
		"AT+CFUN?",
		"AT+CEREG?",
		"AT%XSNRSQ?",
		"AT+CESQ",
		"AT%XTEMP?",
		"AT+CGCONTRDP=0",
		"AT+CCLK?",
		"AT%XMONITOR",
		"AT+CGDCONT?",
		"AT+CGPADDR",
		"AT%XCONNSTAT?",
	] {
		print_at_results(cmd);
	}
}

fn print_at_results(cmd: &str) {
	if let Err(e) = nrfxlib::at::send_at_command(cmd, |s| {
		println!("> {}", s);
	}) {
		println!("Err running {:?}: {:?}", cmd, e);
	}
}

/// Do an HTTP GET
fn command_get(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	context: &mut Context,
) {
	let f = || -> Result<(), Error> {
		let host = "jsonplaceholder.typicode.com";
		let port = 443;
		let url = "/todos/1";

		// We make a secure connection here, using our pre-saved certs
		println!("Making socket..");
		let mut skt = nrfxlib::tls::TlsSocket::new(
			nrfxlib::tls::PeerVerification::Disabled,
			&[SECURITY_TAG],
			nrfxlib::tls::Version::Tls1v3,
		)?;
		println!("Connecting to {}..", host);
		skt.connect(host, port)?;
		println!("Writing...");
		write!(
			skt,
			"GET {url} HTTP/1.1\r\n\
			 Host: {host}:{port}\r\n\
			 Connection: close\r\n\
			 User-Agent: rust/nrf\r\n\
			 \r\n",
			url = url,
			host = host,
			port = port,
		)
		.map_err(|_e| Error::WriteError)?;
		loop {
			let mut buf = [0u8; 32];
			let x = skt.recv_wait(&mut buf)?;
			if let Ok(s) = core::str::from_utf8(&buf[0..x]) {
				print!("{}", s);
			} else {
				print!("{:?}", &buf[0..x]);
			}
			if x < buf.len() {
				break;
			}
		}
		Ok(())
	};

	// Start a timer
	context.timer.start(0xFFFF_FFFFu32);
	// Run the function
	let result = f();
	// Print the result
	let now = context.timer.read();
	println!(
		"Got {:?} after {} seconds",
		result,
		(now as f32) / 1_000_000.0
	);
}

/// Store the certificates into nrfxlib, so the TLS sockets can use them.
fn command_store(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	let f = || -> Result<(), Error> {
		nrfxlib::tls::provision_certificates(SECURITY_TAG, Some(secrets::CA_CHAIN), None, None)?;
		Ok(())
	};
	let result = f();
	println!("Got {:?}", result);
}

/// Deliberately crash
fn command_panic(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	panic!("command_panic was called!")
}

/// Get a GPS fix
fn command_fix(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	_context: &mut Context,
) {
	let gps = nrfxlib::gnss::GnssSocket::new().expect("GnssSocket::new");
	match gps.get_fix() {
		Ok(None) => println!("No data available"),
		Ok(Some(fix)) => {
			println!("{:#?}", fix);
		}
		Err(e) => println!("Error: {:?}", e),
	}
}

/// Enter AT over UART mode
fn command_go_at(
	_menu: &menu::Menu<Context>,
	_item: &menu::Item<Context>,
	_args: &str,
	context: &mut Context,
) {
	let mut f = || -> Result<(), Error> {
		let at_socket = nrfxlib::at::AtSocket::new()?;
		let mut input_buffer: heapless::Vec<u8, heapless::consts::U256> = heapless::Vec::new();
		loop {
			let mut temp_buf = [0u8; 1];
			// Read from console UART
			let res = if let Some(ref mut uart) = *crate::GLOBAL_UART.lock() {
				Some(uart.read_timeout(&mut temp_buf, &mut context.timer, 100_000))
			} else {
				None
			};
			match res {
				Some(Err(bsp::hal::uarte::Error::Timeout(_n))) => {
					// Do nothing. N must be 0 because we only pass a 1 byte buffer to read into
				}
				Some(Err(_)) => {
					return Err(Error::ReadError);
				}
				Some(Ok(_)) => {
					// Send character to modem, unless it's Ctrl+C (0x03), in which
					// case exit.
					print!("{}", temp_buf[0] as char);
					if temp_buf == [0x03] {
						break;
					} else if temp_buf == [b'\n'] || temp_buf == [b'\r'] {
						println!();
						input_buffer.extend(b"\r\n");
						at_socket.write(&input_buffer)?;
						input_buffer.clear();
					} else {
						input_buffer.extend(&temp_buf);
					}
				}
				None => {
					println!("Failed to grab UART lock!");
				}
			}
			let mut buffer = [0u8; 128];
			// Now check the AT socket for data
			match at_socket.recv(&mut buffer)? {
				Some(n) => {
					if let Some(ref mut uart) = *crate::GLOBAL_UART.lock() {
						// Subtract 1 to avoid printing final NUL byte
						uart.write(&buffer[0..n - 1])
							.map_err(|_| Error::WriteError)?;
					}
				}
				None => {
					// Do nothing
				}
			}
		}
		Ok(())
	};
	println!("OK\r\n");
	if let Err(e) = f() {
		println!("Error: {:?}", e);
	}
}

/// Handle the first AT command the link monitor sends, then enter AT mode.
fn command_go_at_fun(
	menu: &menu::Menu<Context>,
	item: &menu::Item<Context>,
	args: &str,
	context: &mut Context,
) {
	match nrfxlib::at::send_at_command("AT+CFUN?", |s| {
		println!("{}", s);
	}) {
		Ok(_) => {
			// Jump to the normal AT handler (which prints OK when it starts)
			command_go_at(menu, item, args, context);
		}
		Err(_) => {
			// Quit with an error.
			println!("ERROR");
		}
	}
}

impl core::fmt::Write for Context {
	fn write_str(&mut self, message: &str) -> core::fmt::Result {
		if let Some(ref mut uart) = *crate::GLOBAL_UART.lock() {
			write!(uart, "{}", message)?;
		}
		Ok(())
	}
}

impl From<nrfxlib::Error> for Error {
	fn from(err: nrfxlib::Error) -> Error {
		Error::Nrfxlib(err)
	}
}

// ==========================================================================
//
// End of file
//
// ==========================================================================
