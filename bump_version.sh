#!/bin/bash

# SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
# SPDX-License-Identifier: CC0-1.0

old_version="0.3.2"
new_version="0.3.3"

sed -i "s/version = \"${old_version}\"/version = \"${new_version}\"/" \
    cli/Cargo.toml lib/Cargo.toml
sed -i "s/echo \"${old_version}\"/echo \"${new_version}\"/" \
    sysexts-manager/justfile
sed -i "s/VERSION=\"${old_version}\"/VERSION=\"${new_version}\"/" \
    README.md

just test
