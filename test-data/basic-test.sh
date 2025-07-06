#!/bin/bash
set -euo pipefail
set -x

cmd=(
    'sysexts-manager'
    "${@}"
)

sysext="$(ls sysexts-manager-*.raw)"
sudo mv "${sysext}" /var/lib/extensions.d
mkdir -p /run/extensions/
sudo ln -snf /var/lib/extensions.d/"${sysext}" /run/extensions/sysexts-manager.raw
sudo systemctl restart systemd-sysext

# Clean up
sudo rm -rf /etc/sysexts-manager/*.conf

"${cmd[@]}" status
systemd-sysext status

sudo "${cmd[@]}" add tree https://extensions.fcos.fr/extensions/tree
sudo "${cmd[@]}" add htop https://extensions.fcos.fr/extensions/htop
sudo "${cmd[@]}" add gdb  https://extensions.fcos.fr/extensions/gdb
test -f /etc/sysexts-manager/tree.conf
test -f /etc/sysexts-manager/htop.conf
test -f /etc/sysexts-manager/gdb.conf

sudo "${cmd[@]}" update
test -f /var/lib/extensions.d/tree*.raw

sudo "${cmd[@]}" symlinks
test -L /run/extensions/tree.raw

sudo "${cmd[@]}" refresh
test -f /usr/bin/tree
tree > /dev/null

"${cmd[@]}" status
systemd-sysext status

sudo "${cmd[@]}" remove tree
sudo "${cmd[@]}" remove htop
sudo "${cmd[@]}" remove gdb
test ! -L /run/extensions/tree.raw
test ! -f /var/lib/extensions.d/tree*.raw
test ! -f /etc/sysexts-manager/tree.conf

"${cmd[@]}" status

sudo "${cmd[@]}" refresh
tree || echo "command not found as expected"

sudo "${cmd[@]}" remove sysexts-manager
sudo "${cmd[@]}" refresh

systemd-sysext status

echo "OK"
