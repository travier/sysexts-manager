#!/bin/bash

# SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
# SPDX-License-Identifier: CC0-1.0

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

# Install the sysexts-manager sysext manually and statically enable it
sudo install -d -m 0755 -o 0 -g 0 "/var/lib/extensions"{,.d} "/run/extensions"
sysext="$(ls sysexts-manager-*.raw)"
sudo mv "${sysext}" "/var/lib/extensions.d"
sudo ln -snf "/var/lib/extensions.d/${sysext}" "/var/lib/extensions/sysexts-manager.raw"
sudo restorecon -RFv "/var/lib/extensions"{,.d} "/run/extensions" > /dev/null
sudo systemctl enable systemd-sysext.service
sudo systemctl restart systemd-sysext.service
sleep 1 # FIXME: Workaround for restart not waiting for completion
