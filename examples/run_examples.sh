#!/bin/sh

cargo build

temp="$(mktemp /tmp/rwsh.XXXX)"
for example in *.rwsh; do 
    echo $example
    ../target/debug/rwsh "$example" > "$temp"
    if ! diff "$temp" "${example%.rwsh}.out" > /dev/null; then
        echo "[Wrong] $example"
        diff "$temp" "${example%.rwsh}.out"
        exit 1
    fi
done