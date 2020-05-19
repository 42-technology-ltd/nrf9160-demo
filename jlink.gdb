target extended-remote :2331

# print demangled symbols
set print asm-demangle on

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32

# detect unhandled exceptions, hard faults and panics
break DefaultHandler
break HardFault
break rust_begin_unwind

# Flash the CPU. The resets appear to be required.
monitor reset
monitor reset
load
monitor reset

# start the process but immediately halt the processor
stepi

