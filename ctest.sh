#!/usr/bin/env bash
set -u

cargo build -q || exit 1

fail=0

for t in x86 arm; do

    sed -i "1s/.*/#target $t/" prog.src
    rm -f out

    ./target/debug/skye prog.src || { echo "$t -> refused"; fail=1; continue; }

    case $t in
        x86) text=$(./out);              code=$? ;;
        arm) text=$(qemu-aarch64 ./out); code=$? ;;
    esac

    if [ "$text" = "hello" ] && [ "$code" = 42 ]; then
        echo "$t -> ok"
    else
        echo "$t -> '$text' / $code"
        fail=1
    fi

done

[ $fail = 0 ] && echo "both targets agree"
exit $fail
