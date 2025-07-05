#!/bin/bash
set -euo pipefail
set -x

sysext="$(ls sysexts-manager-*.raw)"
sudo mv "${sysext}" /var/lib/extensions.d
mkdir -p /run/extensions/
sudo ln -snf /var/lib/extensions.d/"${sysext}" /run/extensions/sysexts-manager.raw
sudo systemctl restart systemd-sysext

sysexts-manager status
systemd-sysext status

sudo sysexts-manager add tree https://extensions.fcos.fr/extensions/tree
test -f /etc/sysexts-manager/tree.conf

sudo sysexts-manager update
test -f /var/lib/extensions.d/tree*.raw

sudo sysexts-manager symlinks
test -L /run/extensions/tree.raw

sudo sysexts-manager refresh
test -f /usr/bin/tree
tree > /dev/null

sysexts-manager status
systemd-sysext status

sudo sysexts-manager remove tree
test ! -L /run/extensions/tree.raw
test ! -f /var/lib/extensions.d/tree*.raw
test ! -f /etc/sysexts-manager/tree.conf

sysexts-manager status

sudo sysexts-manager refresh
tree || echo "command not found as expected"

sudo sysexts-manager remove sysexts-manager
sudo sysexts-manager refresh

systemd-sysext status

echo "OK"
