#!/usr/bin/env bash
set -u

cargo build -q || exit 1

fail=0

for src in tests/*.src; do

    want=$(grep -oP '^// expect:\s*\K\d+' "$src" | head -1)

    if [ -z "$want" ]; then
        echo "$src -> no expectation"
        fail=1
        continue
    fi

    for t in x86 arm; do

        rm -f out build.src
        { echo "#target $t"; cat "$src"; } > build.src

        if ! ./target/debug/skye build.src 2>/dev/null; then
            echo "$(basename $src) [$t] -> refused"
            fail=1
            continue
        fi

        case $t in
            x86) ./out >/dev/null 2>&1;              got=$? ;;
            arm) qemu-aarch64 ./out >/dev/null 2>&1; got=$? ;;
        esac

        if [ "$got" != "$want" ]; then
            echo "$(basename $src) [$t] -> $got, want $want"
            fail=1
        fi

    done

done

rm -f build.src

[ $fail = 0 ] && echo "all fixtures pass on both targets"
exit $fail
