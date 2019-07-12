#!/bin/sh

finish() {
	rm "$temp"
}

cargo build

temp="$(mktemp /tmp/rwsh.XXXX)"
trap finish EXIT
for example in *.rwsh; do
    echo $example
    ../target/debug/rwsh "$example" > "$temp"
    if ! diff "$temp" "${example%.rwsh}.out" > /dev/null; then
        echo "[Wrong] $example"
        diff "$temp" "${example%.rwsh}.out"
    fi
done
