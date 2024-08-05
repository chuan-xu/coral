#!/bin/bash

SCRIPT_DIR=$(dirname "$(realpath "$0")")
BIN_DIR="$SCRIPT_DIR/bin"
TARGET_DIR="$SCRIPT_DIR/target/release"
CORAL_PROXY="$TARGET_DIR/coral-proxy"

if [ ! -d "$BIN_DIR" ]; then
  mkdir -p $BIN_DIR
fi

cd $SCRIPT_DIR;

cargo build --release

cp $CORAL_PROXY $BIN_DIR