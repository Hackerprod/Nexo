#!/usr/bin/env sh
set -eu
export PATH="$HOME/.cargo/bin:$PATH"
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/nexo-compare-head.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
head_dir="$tmp/head"
head_target="$tmp/head-target"
work_target="$tmp/work-target"
inputs="$tmp/inputs"
results="$tmp/results"
mkdir -p "$head_dir" "$inputs" "$results"
git -C "$root" archive HEAD | tar -x -C "$head_dir"
cargo build --locked --offline --manifest-path "$head_dir/Cargo.toml" --target-dir "$head_target"
cargo build --locked --offline --manifest-path "$root/Cargo.toml" --target-dir "$work_target"
head_bin="$head_target/debug/nexo"
work_bin="$work_target/debug/nexo"
[ -x "$head_bin" ] || head_bin="$head_bin.exe"
[ -x "$work_bin" ] || work_bin="$work_bin.exe"

printf 'a\377b' >"$inputs/invalid-utf8.nx"
printf 'a\360\237' >"$inputs/invalid-truncated-end-2prefix.nx"
printf 'a\360\237\231' >"$inputs/invalid-truncated-end-3prefix.nx"
printf 'a\342' >"$inputs/invalid-truncated-end-1prefix.nx"
printf 'a\360\237b' >"$inputs/invalid-truncated-middle-2prefix.nx"
printf 'a\360\237\231b' >"$inputs/invalid-truncated-middle-3prefix.nx"
printf 'a\342b' >"$inputs/invalid-truncated-middle-1prefix.nx"
printf '\377' >"$inputs/invalid-single-byte.nx"
printf 'x\300\200x' >"$inputs/invalid-overlong-2.nx"
printf 'x\340\200\200x' >"$inputs/invalid-overlong-3.nx"
printf 'x\355\240\200x' >"$inputs/invalid-surrogate-3.nx"
printf 'x\364\220\200\200x' >"$inputs/invalid-above-unicode-4.nx"
printf 'x\360\200\200\200x' >"$inputs/invalid-four-byte-prefix.nx"
printf 'x\365\200\200\200x' >"$inputs/invalid-leading-5.nx"
: >"$inputs/empty.nx"
dd if=/dev/zero bs=1048576 count=1 2>/dev/null | tr '\000' 'a' >"$inputs/exact-1mib.nx"
dd if=/dev/zero bs=1048576 count=1 2>/dev/null | tr '\000' 'a' >"$inputs/over-1mib.nx"
printf 'a' >>"$inputs/over-1mib.nx"
dd if=/dev/zero bs=1048575 count=1 2>/dev/null | tr '\000' 'a' >"$inputs/emoji-split-at-limit.nx"
printf '\360\237\231\202tail' >>"$inputs/emoji-split-at-limit.nx"
for spec in '1 \342' '2 \360\237' '3 \360\237\231' '4 \360\200\200\200'; do
    set -- $spec
    prefix=$1; octets=$2
    for position in limit limit-plus-one; do
        if [ "$position" = limit ]; then fill=$((1048576 - prefix)); else fill=$((1048576 - prefix + 1)); fi
        name="boundary-${position}-truncated-${prefix}"
        dd if=/dev/zero bs=1 count="$fill" 2>/dev/null | tr '\000' 'a' >"$inputs/$name.nx"
        printf "$octets" >>"$inputs/$name.nx"
    done
done
for position in limit limit-plus-one; do
    if [ "$position" = limit ]; then fill=1048575; else fill=1048576; fi
    name="boundary-${position}-invalid-one-byte"
    dd if=/dev/zero bs=1 count="$fill" 2>/dev/null | tr '\000' 'a' >"$inputs/$name.nx"
    printf '\377' >>"$inputs/$name.nx"
done

differences=0
compare() {
    label=$1; shift
    set +e
    "$head_bin" "$@" >"$results/head.out" 2>"$results/head.err"
    head_status=$?
    "$work_bin" "$@" >"$results/work.out" 2>"$results/work.err"
    work_status=$?
    set -e
    if [ "$head_status" -ne "$work_status" ] || ! cmp -s "$results/head.out" "$results/work.out" || ! cmp -s "$results/head.err" "$results/work.err"; then
        differences=$((differences + 1))
        printf 'DIFF %s (exit %s vs %s)\n' "$label" "$head_status" "$work_status"
        diff -u "$results/head.out" "$results/work.out" || true
        diff -u "$results/head.err" "$results/work.err" || true
    fi
}
for mode in check run; do
    for file in "$root"/corpus/pass/*.nx "$root"/corpus/fail/*.nx; do
        [ -f "$file" ] || continue
        compare "$mode corpus/$(basename "$(dirname "$file")")/$(basename "$file")" "$mode" "${file#"$root/"}"
    done
    for file in "$inputs"/*.nx; do
        compare "$mode $(basename "$file")" "$mode" "$file"
    done
    compare "$mode nonexistent" "$mode" "$tmp/missing-file.nx"
done
compare "no arguments"
compare "invalid command" invalid-command
compare "invalid argument count" check a.nx extra
if [ "$differences" -eq 0 ]; then
    printf '0 differences\n'
else
    printf '%s differences\n' "$differences"
    exit 1
fi
