#!/bin/bash

PROFILE="release"

if [ -z "$1" ]; then
  PROFILE="debug"
fi

SCRIPT_DIR=$(dirname "$(realpath "$0")")
BIN_DIR="$SCRIPT_DIR/bin"
TARGET_DIR="$SCRIPT_DIR/target/release"
CORAL_PROXY="$TARGET_DIR/coral-proxy"
CORAL_SERVER="$TARGET_DIR/coral-server"

if [ ! -d "$BIN_DIR" ]; then
  mkdir -p $BIN_DIR
fi

cd $SCRIPT_DIR;

cargo build $1

cp $CORAL_PROXY $BIN_DIR
cp $CORAL_SERVER $BIN_DIR
