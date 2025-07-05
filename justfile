all: build-run

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
    scp ./sysexts-manager/sysexts-manager-*.raw fcos-next:
    ssh fcos-next "sudo mv sysexts-manager-*.raw /var/lib/extensions/sysexts-manager.raw && sudo systemctl restart systemd-sysext"

# Basic functionnality test on a remote host
test-remote: sysext
    scp ./test-data/basic-test.sh ./sysexts-manager/sysexts-manager-*.raw fcos-next:
    ssh fcos-next ./basic-test.sh

# Serve the build dir for remote testing
#
# When using libvirtd, open firewall port with:
# sudo firewall-cmd --zone=libvirt --add-port=8000/tcp
serve:
    cd target/debug && simple-http-server

# Run on a remote host
#
# `run.sh`:
# #!/bin/bash
# curl -O --silent http://192.168.100.1:8000/sysexts-manager && chmod a+x sysexts-manager && sudo ./sysexts-manager "${@}"
build-run-remote *args:
    cargo build
    ssh -t fcos-next ./run.sh {{args}}

# Run outside of toolbox
build-run-local *args:
    cargo build
    flatpak-spawn --watch-bus --host ./target/debug/sysexts-manager {{args}}
