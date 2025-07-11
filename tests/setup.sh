#!/bin/bash
set -euo pipefail
# set -x

# Execute a command, also writing the cmdline to stdout
runv() {
    echo "+" "$@"
    "$@"
}

cmd=('sysexts-manager' "${@}")

# Clean up
sudo rm -rf "/etc/sysexts-manager"
sudo rm -rf "/run/extensions"
sudo rm -rf "/var/lib/extensions"
sudo rm -rf "/var/lib/extensions.d"

# Install sysexts-manager sysext manually
sudo install -d -m 0755 -o 0 -g 0 "/var/lib/extensions.d" "/run/extensions"
sysext="$(ls sysexts-manager-*.raw)"
sudo mv "${sysext}" "/var/lib/extensions.d"
sudo ln -snf "/var/lib/extensions.d/${sysext}" "/run/extensions/sysexts-manager.raw"
sudo restorecon -RFv "/var/lib/extensions.d" "/run/extensions" > /dev/null
sudo systemctl enable systemd-sysext.service
sudo systemctl restart systemd-sysext.service
sleep 1 # FIXME: Workaround for restart not waiting for completion
