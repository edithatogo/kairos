#!/bin/sh
set -eu

archive=$1
compacted_source=$2
output=$3
probe=$(dirname "$0")/probe.rs

mkdir "$output"
rustc -vV >&2
for variant in baseline compacted; do
    source_dir="$output/$variant/source"
    project="$output/$variant/probe"
    mkdir -p "$source_dir" "$project/src"
    tar -xzf "$archive" -C "$source_dir"
    if test "$variant" = compacted; then
        cp "$compacted_source" "$source_dir/crates/kairo-ecs-core/src/lib.rs"
    fi
    cp "$probe" "$project/src/main.rs"
    cat > "$project/Cargo.toml" <<'TOML'
[package]
name = "scheduler-pressure"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
kairo-ecs-core = { path = "../source/crates/kairo-ecs-core" }
kairo-ecs-types = { path = "../source/crates/kairo-ecs-types" }
TOML
    (
        cd "$project"
        CARGO_TARGET_DIR=target cargo generate-lockfile --offline
        CARGO_TARGET_DIR=target cargo build --release --locked --offline
    ) >&2
done

printf '%s\n' 'workload,pair,profile,count,retained_before,retained_after,loop_ns,drain_ns,dispatched'
for workload in pressure burst mixed; do
    count=1000000
    if test "$workload" = burst; then
        count=250000
    fi
    for variant in baseline compacted; do
        "$output/$variant/probe/target/release/scheduler-pressure" "$workload" "$count" > /dev/null
    done
    for pair in 1 2 3 4 5 6 7 8 9 10; do
        order='baseline compacted'
        if test "$((pair % 2))" = 0; then
            order='compacted baseline'
        fi
        for variant in $order; do
            row=$("$output/$variant/probe/target/release/scheduler-pressure" "$workload" "$count")
            printf '%s,%s,%s,%s\n' "$workload" "$pair" "$variant" "$row"
        done
    done
done
