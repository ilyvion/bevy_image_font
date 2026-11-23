#!/usr/bin/env bash
# Measures test coverage with the tarpaulin Docker image the CI coverage job
# uses, with the CI job's tarpaulin arguments and `--cfg ci`, which ignores the
# visual tests because the container has no GPU.
#
# Usage: ./coverage.sh [extra tarpaulin arguments...]
#
# Reports are written to target/tarpaulin/: cobertura.xml (what CI uploads to
# Codecov) and tarpaulin-report.html. Extra arguments are passed to tarpaulin
# after the CI ones, e.g. `./coverage.sh --skip-clean` to reuse the previous
# build.
set -euo pipefail

cd "$(dirname "$0")"

image=xd009642/tarpaulin:develop-nightly
# Named volumes keep the downloaded crates and the nightly toolchain that
# `cargo +nightly` installs between runs.
volume_prefix=bevy_image_font-tarpaulin

# `label=disable` lets the container read the project directory on SELinux
# hosts without relabeling it.
docker run --rm \
    --security-opt seccomp=unconfined \
    --security-opt label=disable \
    --volume "$PWD:/volume" \
    --volume "$volume_prefix-registry:/usr/local/cargo/registry" \
    --volume "$volume_prefix-rustup:/usr/local/rustup" \
    --workdir /volume \
    --env CARGO_TARGET_DIR=/volume/target/tarpaulin \
    --env RUSTFLAGS="--cfg ci" \
    --env HOST_UID="$(id -u)" \
    --env HOST_GID="$(id -g)" \
    "$image" \
    bash -c '
        status=0
        cargo +nightly tarpaulin --verbose --all-features --workspace --timeout 120 --engine llvm \
            --out xml --out html --output-dir target/tarpaulin "$@" || status=$?
        # The container runs as root; give the build output back to the host user.
        chown -R "$HOST_UID:$HOST_GID" target/tarpaulin
        exit "$status"
    ' tarpaulin "$@"
