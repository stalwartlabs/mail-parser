#!/bin/sh

perf record -F max  --call-graph dwarf target/release/mail-parser
perf script | inferno-collapse-perf | inferno-flamegraph > flamegraph.svg
