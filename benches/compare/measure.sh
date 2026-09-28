#!/bin/sh
set -eu

cd "$(dirname "$0")"
TARGET=${CARGO_TARGET_DIR:-$(pwd)/target}
RUNS_DIR=${RUNS_DIR:-$TARGET/compare-runs}
LABEL_PREFIX=${LABEL_PREFIX:-compare}
ATTEMPTS=${ATTEMPTS:-5}
PAUSE=${PAUSE:-240}
BUILD_TIME_LIMIT=${BUILD_TIME_LIMIT:-10800}
MEASURE_TIME_LIMIT=${MEASURE_TIME_LIMIT:-1800}
PART_A="testsuite-lf testsuite-crlf enron-crlf attachments-crlf newsletter-crlf"
PART_B="modern-headers-crlf forwarded-crlf dashes-crlf plain-large-crlf attachments-lf newsletter-lf"
STOP="$RUNS_DIR/STOP"

if [ $# -lt 1 ]; then
    echo "usage: $0 <run> [session...]   (sessions: <structure|headers|full>-<a|b>)" >&2
    exit 2
fi
run=$1
shift
sessions=${*:-"structure-a structure-b headers-a headers-b full-a full-b"}

step() {
    if [ -n "${BUILD_WRAP:-}" ]; then
        "$BUILD_WRAP" perl -e 'alarm shift; exec @ARGV' "$BUILD_TIME_LIMIT" "$@"
    else
        "$@"
    fi
}

stopped() {
    if [ -e "$STOP" ]; then
        echo "$STOP exists: stopping before the next session" >&2
        exit 3
    fi
}

pause() {
    waited=0
    while [ "$waited" -lt "$PAUSE" ]; do
        stopped
        sleep 10
        waited=$((waited + 10))
    done
}

top_processes() {
    case "$(uname -s)" in
        Darwin) ps -Arco pid,pcpu,comm | head -n 8 ;;
        *) ps -eo pid,pcpu,comm --sort=-pcpu | head -n 8 ;;
    esac
}

BENCH=$(sed -n 's/.*Executable benches\/compare\.rs (\(.*\))$/\1/p' "$TARGET/bench-build.log" 2>/dev/null | tail -n 1)
case "$BENCH" in
    "")
        echo "no compare bench executable in $TARGET/bench-build.log: run cargo bench --no-run > $TARGET/bench-build.log 2>&1 first" >&2
        exit 1
        ;;
    /*) ;;
    *) BENCH="$(pwd)/$BENCH" ;;
esac

NOISY=
if [ -n "${BENCH_LOCK:-}" ]; then
    NOISY=${BENCH_NOISY_LOG:-${BENCH_LOCK_STATE:-$(dirname "$BENCH_LOCK")/state}/noisy.log}
    export BENCH_IDLE_MIN="${BENCH_IDLE_MIN:-92}"
    mkdir -p "$(dirname "$NOISY")"
    touch "$NOISY"
fi

for session in $sessions; do
    case "$session" in
        structure-a | structure-b | headers-a | headers-b | full-a | full-b) ;;
        *) echo "unknown session $session" >&2; exit 2 ;;
    esac
done

out="$RUNS_DIR/r$run"
mkdir -p "$out/criterion"
stopped
step "$BENCH" --list > /dev/null

for session in $sessions; do
    workload=${session%-*}
    case "${session##*-}" in
        a) corpora=$PART_A ;;
        *) corpora=$PART_B ;;
    esac
    filter="^$workload/($(echo $corpora | tr ' ' '|'))/"
    attempt=1
    while :; do
        stopped
        label="$LABEL_PREFIX-$session-r$run-a$attempt"
        (
            while :; do
                date '+%H:%M:%S'
                top_processes
                sleep 10
            done
        ) > "$out/ps-$label.log" 2>&1 &
        sampler=$!
        status=0
        if [ -n "$NOISY" ]; then
            before=$(wc -l < "$NOISY")
            "$BENCH_LOCK" "$label" perl -e 'alarm shift; exec @ARGV' "$MEASURE_TIME_LIMIT" \
                "$BENCH" --bench "$filter" > "$out/$label.log" 2>&1 || status=$?
        else
            "$BENCH" --bench "$filter" > "$out/$label.log" 2>&1 || status=$?
        fi
        kill "$sampler" 2> /dev/null || true
        wait "$sampler" 2> /dev/null || true
        if [ "$status" -ne 0 ]; then
            echo "$label: exit status $status (log: $out/$label.log)" >&2
            exit "$status"
        fi
        if [ -n "$NOISY" ] && tail -n +"$((before + 1))" "$NOISY" | grep -q " $label "; then
            echo "$label: noisy ($(grep " $label " "$NOISY" | tail -n 1))"
            attempt=$((attempt + 1))
            if [ "$attempt" -gt "$ATTEMPTS" ]; then
                echo "$session: still noisy after $ATTEMPTS attempts" >&2
                exit 1
            fi
            pause
            continue
        fi
        echo "$label: clean"
        for corpus in $corpora; do
            if [ -d "$TARGET/criterion/${workload}_$corpus" ]; then
                rm -rf "$out/criterion/${workload}_$corpus"
                cp -R "$TARGET/criterion/${workload}_$corpus" "$out/criterion/"
            fi
        done
        break
    done
done
echo "run $run done: $out/criterion"
