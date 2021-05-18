#!/bin/bash
set -euo pipefail

OUTPUT_DIR=./target/bin/release
INPUT=./target/thumbv8m.main-none-eabihf/release/nrf9160-demo

function process {
    local build_type=$1
    echo "Creating $1.elf..."
    cargo build --release --features=$1
    cp $INPUT $OUTPUT_DIR/$1.elf
    echo "Creating $1.hex..."
    arm-none-eabi-objcopy -O ihex $INPUT $OUTPUT_DIR/$1.hex
    echo "Creating $1.sym..."
    arm-none-eabi-objdump -t $INPUT > $OUTPUT_DIR/$1.sym
    echo "Creating $1.S..."
    arm-none-eabi-objdump -dC $INPUT > $OUTPUT_DIR/$1.S
    echo "Creating $1.bin..."
    arm-none-eabi-objcopy -O binary $INPUT $OUTPUT_DIR/$1.bin
}

mkdir -p $OUTPUT_DIR
process nrf9160dk
process icarus
ls -lh $OUTPUT_DIR
