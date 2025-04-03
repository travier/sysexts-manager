all: build-run

build-run *args:
    cargo build
    ./target/debug/sysexts-manager symlinks {{args}}

build-run-local *args:
    cargo build
    flatpak-spawn --watch-bus --host ./target/debug/sysexts-manager symlinks {{args}}
