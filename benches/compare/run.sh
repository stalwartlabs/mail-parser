#!/bin/sh
set -eu

cd "$(dirname "$0")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$(pwd)/target}"
TARGET=$CARGO_TARGET_DIR
RESULTS=${RESULTS_DIR:-$(pwd)/results}
RUNS=${RUNS:-3}
BUILD_TIME_LIMIT=${BUILD_TIME_LIMIT:-10800}

step() {
    if [ -n "${BUILD_WRAP:-}" ]; then
        "$BUILD_WRAP" perl -e 'alarm shift; exec @ARGV' "$BUILD_TIME_LIMIT" "$@"
    else
        "$@"
    fi
}

excludes=
for implementation in ${EXCLUDE:-}; do
    excludes="$excludes --exclude $implementation"
done

for archive in gmime/lib/libgmime-3.0.a vmime/lib/libvmime.a libetpan/lib/libetpan-mime.a \
    dovecot/lib/libdovecot-mail.a; do
    if [ ! -f "vendor/prefix/$archive" ]; then
        vendor/build.sh
        break
    fi
done

mkdir -p "$TARGET" "$RESULTS"
step cargo build --release --bins
step cargo bench --no-run > "$TARGET/bench-build.log" 2>&1
step "$TARGET/release/check" --examples 5 $excludes --out "$RESULTS/check.md" > /dev/null

runs=
run=1
while [ "$run" -le "$RUNS" ]; do
    ./measure.sh "$run"
    runs="$runs ${RUNS_DIR:-$TARGET/compare-runs}/r$run/criterion"
    run=$((run + 1))
done
"$TARGET/release/summary" $excludes $runs --out "$RESULTS/summary.md"
