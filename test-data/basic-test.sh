#!/bin/bash
set -euo pipefail
# set -x

# Execute a command, also writing the cmdline to stdout
runv() {
    echo "+" "$@"
    "$@"
}

cmd=(
    'sysexts-manager'
    "${@}"
)

extensions=(
    'gdb'
    'htop'
    'tree'
)

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

runv "${cmd[@]}" status
runv systemd-sysext status

for ext in "${extensions[@]}"; do
    runv sudo "${cmd[@]}" add "${ext}" "https://extensions.fcos.fr/extensions"
    test -f "/etc/sysexts-manager/${ext}.conf"
done

runv sudo "${cmd[@]}" update
for ext in "${extensions[@]}"; do
    test -f "/var/lib/extensions.d/${ext}"*".raw"
done

runv sudo "${cmd[@]}" enable
for ext in "${extensions[@]}"; do
    test -L "/run/extensions/${ext}.raw"
done

runv sudo "${cmd[@]}" refresh
for ext in "${extensions[@]}"; do
    test -f "/usr/bin/${ext}"
done
tree > /dev/null

runv "${cmd[@]}" status
runv systemd-sysext status

runv sudo "${cmd[@]}" remove tree || echo "failed to remove still enabled sysext as expected"

for ext in "${extensions[@]}"; do
    runv sudo "${cmd[@]}" disable "${ext}"
    test ! -L "/run/extensions/${ext}.raw"
    runv sudo "${cmd[@]}" refresh
    runv systemd-sysext status
done

runv sudo "${cmd[@]}" refresh

for ext in "${extensions[@]}"; do
    runv sudo "${cmd[@]}" remove "${ext}"
    test ! -f "/var/lib/extensions.d/${ext}"*".raw"
    test ! -f "/etc/sysexts-manager/${ext}.conf"
done

runv "${cmd[@]}" status

for ext in "${extensions[@]}"; do
    test ! -f "/usr/bin/${ext}"
done

runv sudo "${cmd[@]}" disable sysexts-manager
runv sudo "${cmd[@]}" remove sysexts-manager
test ! -f "/var/lib/extensions.d/sysexts-manager"*".raw"
runv sudo "${cmd[@]}" refresh

runv systemd-sysext status

echo "OK"
