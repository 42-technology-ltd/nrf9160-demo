#!/bin/bash
set -euo pipefail

OUTPUT_DIR=./target/bin/release
INPUT=./target/thumbv8m.main-none-eabi/release/nrf9160-demo
mkdir -p $OUTPUT_DIR
cargo build --release --features=nrf9160dk
arm-none-eabi-objcopy -O ihex $INPUT $OUTPUT_DIR/nrf9160dk.hex
arm-none-eabi-objdump -t $INPUT > $OUTPUT_DIR/nrf9160dk.sym
arm-none-eabi-objdump -dC $INPUT > $OUTPUT_DIR/nrf9160dk.S
arm-none-eabi-objcopy -O binary $INPUT $OUTPUT_DIR/nrf9160dk.bin
cargo build --release --features=icarus
arm-none-eabi-objcopy -O ihex $INPUT $OUTPUT_DIR/icarus.hex
arm-none-eabi-objdump -t $INPUT > $OUTPUT_DIR/icarus.sym
arm-none-eabi-objdump -dC $INPUT > $OUTPUT_DIR/icarus.S
arm-none-eabi-objcopy -O binary $INPUT $OUTPUT_DIR/icarus.bin
