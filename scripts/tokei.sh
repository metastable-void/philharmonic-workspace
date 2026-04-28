#!/bin/sh

set -eu

. "$(dirname -- "$0")/lib/workspace-cd.sh"

if ! command -v cargo >/dev/null 2>&1 ; then
    echo "Cargo required" >&2
    exit 1
fi

cargo install tokei --features=all
tokei
