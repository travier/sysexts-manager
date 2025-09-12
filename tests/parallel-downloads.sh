#!/bin/bash

# SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
# SPDX-License-Identifier: CC0-1.0

set -euo pipefail
# set -x

source ./setup.sh

extensions=(
    'bandwhich'
    'btop'
    'bwm-ng'
    'cilium-cli'
    'cloud-hypervisor'
    'distrobox'
    'emacs'
    'erofs-utils'
    'fd-find'
    'fish'
    'gdb'
    'git-absorb'
    'git-delta'
    'git-lfs'
    'glab'
    'helix'
    'htop'
    'iotop'
    'just'
    'neovim'
    'python3'
    'ripgrep'
    'semanage'
    'strace'
    'tmux'
    'tree'
    'vim'
    'virtctl'
    'youki'
    'zoxide'
    'zsh'
)

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
# for ext in "${extensions[@]}"; do
#     test -f "/usr/bin/${ext}"
# done
tree > /dev/null

runv "${cmd[@]}" status
runv systemd-sysext status

runv sudo "${cmd[@]}" remove tree || echo "failed to remove still enabled sysext as expected"

for ext in "${extensions[@]}"; do
    runv sudo "${cmd[@]}" disable "${ext}"
    test ! -L "/run/extensions/${ext}.raw"
done

runv sudo "${cmd[@]}" refresh

for ext in "${extensions[@]}"; do
    runv sudo "${cmd[@]}" remove "${ext}"
    test ! -f "/var/lib/extensions.d/${ext}"*".raw"
    test ! -f "/etc/sysexts-manager/${ext}.conf"
done

runv "${cmd[@]}" status

# for ext in "${extensions[@]}"; do
#     test ! -f "/usr/bin/${ext}"
# done

# Manually remove the persistent symlink for now
sudo rm "/var/lib/extensions/sysexts-manager.raw"

runv sudo "${cmd[@]}" disable sysexts-manager
runv sudo "${cmd[@]}" remove sysexts-manager
test ! -f "/var/lib/extensions.d/sysexts-manager"*".raw"
runv sudo "${cmd[@]}" refresh

runv systemd-sysext status

echo "OK"
