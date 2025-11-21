# SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
# SPDX-License-Identifier: CC0-1.0

# Build and run
all: build-run

# Auto re-format and run clippy
lint:
    cargo fmt && cargo build && cargo clippy

# Direct run, i.e. like cargo run
build-run *args:
    cargo build
    ./target/debug/sysexts-manager {{args}}

# Build sysext
sysext:
    cargo build --release
    mkdir -p sysexts-manager/usr/bin
    cp target/release/sysexts-manager sysexts-manager/usr/bin
    cd sysexts-manager && just build quay.io/fedora-ostree-desktops/base-atomic:42

# Build and deploy sysext in remote host
sysext-remote: sysext
    scp ./tests/setup.sh ./sysexts-manager/sysexts-manager-*.raw fcos-next:
    ssh fcos-next ./setup.sh

# Run local tests
test:
    cargo test

# Basic functionality test on a remote host
test-remote *args: sysext
    scp ./tests/{setup,basic-test}.sh ./sysexts-manager/sysexts-manager-*.raw fcos-next:
    ssh fcos-next ./basic-test.sh {{args}}

# Stress test download of a larger set of sysexts on a remote host
test-remote-stress *args: sysext
    scp ./tests/{setup,parallel-downloads}.sh ./sysexts-manager/sysexts-manager-*.raw fcos-next:
    ssh fcos-next ./parallel-downloads.sh {{args}}

# Run on a remote host
build-run-remote *args:
    cargo build
    scp target/debug/sysexts-manager fcos-next:
    ssh -t fcos-next sudo ./sysexts-manager {{args}}

# Run outside of toolbox
build-run-local *args:
    cargo build
    flatpak-spawn --watch-bus --host ./target/debug/sysexts-manager {{args}}
