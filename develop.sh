#!/bin/bash

set -euo pipefail
current_path="$(realpath $0)"
current_dir="$(dirname $current_path)"

function test() {
	cargo test --workspace --all-targets --all-features -- --nocapture
}

function build() {
	# RUSTFLAGS="-C lto -C opt-level=3" cargo build
}

function help() {
	echo "Usage: $(basename "$0") [OPTIONS]

Commands:
  test           Run all tests
  build          Build the project
  help           Show help
"
}

if [[ $1 =~ ^(test|build|help)$ ]]; then
	"$@"
else
	help
	exit 1
fi
