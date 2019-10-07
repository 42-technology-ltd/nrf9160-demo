# nrf9160-blink

> An example Rust project for the nRF9160 LTE SiP

This project is developed and maintained by 42 Technology (www.42technology.com)

## Features

* Is started by the Nordic 'secure bootloader' and operates in ‘insecure’ mode
* Uses just 140 KiB out of 768 KiB of Application Flash
* Uses just 8 KiB out of the 128 KiB Application RAM (plus stack)
* Uses the pre-compiled Nordic `libbsd` static library for mbedTLS, the Nordic
  socket API, access to the LTE modem, etc.
* Demonstrates embedded-hal compliant drivers for the nRF9160's Timer, GPIO and
  UARTE peripherals
* Has a menu-driven interface over UARTE0 (which appears as a USB Serial device)
* Demonstrates access to HTTP services over TLS

## Dependencies

To build embedded programs using this template you'll need:

- Rust `rustc 1.36.0 (a53f9df32 2019-07-03)` or newer

- `rust-std` components (pre-compiled `core` crate) for armv8m.main targets.

- GCC for bare-metal ARM (`arm-none-eabi-gcc`), with the newlib C library

- clang

To get these things on Ubuntu 18.04, run:

``` console
$ apt-get update && apt-get install -y curl llvm-dev libclang-dev clang git
$ curl -Lq https://developer.arm.com/-/media/Files/downloads/gnu-rm/8-2018q4/gcc-arm-none-eabi-8-2018-q4-major-linux.tar.bz2 | tar xjf - -C ~
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s - '-y'
$ source $HOME/.cargo/env
$ rustup target add thumbv8m.main-none-eabi
$ export PATH=$PATH:~/gcc-arm-none-eabi-8-2018-q4-major/bin
$ export NEWLIB_PATH=~/gcc-arm-none-eabi-8-2018-q4-major/arm-none-eabi/include
```

To build, just run:

```console
$ ./release.sh
```

The outputs are placed in `target/bin/release`. You can also run `./debug.sh`
if you want an unoptimised version.

To flash, load up J-Link Commander, and run:

```
J-Link> usb
J-Link> connect
# select nRF9160
J-Link> r # for reset
J-Link> loadfile ~/nrf9160-blink/target/bin/release/nrf9160-demo.hex # to flash the board
J-Link> r # for reset
J-Link> g # for go
```

*NOTE*: The Nordic secure bootloader will not boot if the USB-Serial is
disconnected. Connect your serial terminal to the lowest numbered COM port on
the nRF9160-DK then press the 'Reset' button to reboot the chip.

# Upstream

This project is based on
[cortex-m-quickstart](https://github.com/rust-embedded/cortex-m-quickstart) by
the Rust Embedded team. We are grateful for their work.

# License

This example is licensed to your under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option. Please note that any dependencies, including the Nordic libbsd library,
may be under different licensing terms.

