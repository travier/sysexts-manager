# systemd system extensions manager

Work in progress manager for systemd system extensions (sysexts).

See the [demo for v0.3.0](https://asciinema.org/a/745601).

## How to use

Install the `sysexts-manager` pre-built sysext:

```bash
VERSION="0.3.1" # sysexts-manager version
VERSION_ID="43" # Fedora release
ARCH="x86-64"   # or arm64
URL="https://github.com/travier/sysexts-manager/releases/download/sysexts-manager/"
NAME="sysexts-manager-${VERSION}-${VERSION_ID}-${ARCH}.raw"
sudo install -d -m 0755 -o 0 -g 0 "/var/lib/extensions"{,.d} "/run/extensions"
curl --silent --fail --location "${URL}/${NAME}" \
    | sudo bash -c "cat > /var/lib/extensions.d/${NAME}"
ln -snf "/var/lib/extensions.d/${NAME}" "/var/lib/extensions/sysexts-manager.raw"
sudo restorecon -RFv "/var/lib/extensions"{,.d} "/run/extensions"
sudo systemctl enable systemd-sysext.service
sudo systemctl restart systemd-sysext.service
```

As `sysexts-manager` is a sysext, it is capable of managing itself:

```bash
sysexts-manager status
```

Install the tree sysext from [extensions.fcos.fr](https://extensions.fcos.fr):

```bash
sudo sysexts-manager add tree https://extensions.fcos.fr/extensions
```

Update all sysexts managed by sysexts-manager:

```bash
sudo sysexts-manager update
```

Enable the sysext by creating temporary symlinks in `/run/extensions`:

```bash
sudo sysexts-manager enable tree
```

Ask systemd to refresh enabled sysexts:

```bash
sudo sysexts-manager refresh
```

List all sysexts managed by sysexts-manager:

```bash
sysexts-manager status
```

Disable the tree sysext:

```bash
sudo sysexts-manager disable tree
```

Remove the tree sysext and all installed images:

```bash
sudo sysexts-manager remove tree
sudo sysexts-manager refresh
```

## How does this work on boot?

We statically install a copy of the `sysexts-manager` sysext and enable
`systemd-sysext.service` to run on boot. When systemd loads the
`sysexts-manager` sysext on boot, it will trigger `sysexts-manager.service`
which will enable and load all other sysexts on demand.

## Expected layout for hosted sysexts

sysexts-manager expects to find the following layout at the URL used to
setup a sysext. This layout is intended to match the one defiend in
[systemd's sysupdate.d](https://www.freedesktop.org/software/systemd/man/latest/sysupdate.d.html).
Any deviations will be considered a bug.

For the systext `tree`, with the URL configured to
`https://extensions.fcos.fr/extensions`, sysexts-manager will expect:

```
.
└── tree
    ├── SHA256SUMS
    ├── tree-2.1.0-6.fc41-41-arm64.raw
    ├── tree-2.1.0-6.fc41-41-x86-64.raw
    ├── tree-2.2.1-1.fc42-42-arm64.raw
    └── tree-2.2.1-1.fc42-42-x86-64.raw
```

It will thus fetch `https://extensions.fcos.fr/extensions/tree/SHA256SUMS`
first to get the list of available versions, and then will fetch updates as
needed.

The name of the systexts image must follow the following format:
`<sysext name>-<sysext version>-<major Fedora release>-<architecture>.raw`.

Currently supported architectures are `x86-64` and `arm64`. The architecture
names are those used by systemd (see:
[ConditionArchitecture in `systemd.unit`](https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#ConditionArchitecture=)).

You can host your own sysexts anywhere that offers access over HTTPS. See the
[actions](.github/actions) in this repo for an example to build and host your
own using GitHub releases. See
[issue#10](https://github.com/travier/sysexts-manager/issues/10) if you are
interested in turning this into an independent GitHub Action.

## Why?

Why do we need something else when we already have `systemd-sysupdate`?

While installing and updating sysexts via `systemd-sysupdate` *works* (as done
with [extensions.fcos.fr](https://extensions.fcos.fr/)), it also comes with a
few limitations:
- The commands to install, update and remove sysexts at
  [extensions.fcos.fr](https://extensions.fcos.fr/) are prone to errors and are
  not really user friendly.
- The sysexts are enabled "statically" in `/var` for all deployments. If you
  update the sysexts but rollback to a previous version, you will get the
  updated sysexts. This is mainly an issue when you rebase between major Fedora
  versions as the sysexts will not match the Fedora release when you boot into
  the new version. If you update them and then rollback, then the sysexts won't
  work with the rollback version. One way to partially fix that would be to use
  `/etc` to enable the sysexts but that would only fix the rollback issue and
  not the update one.
- The SELinux policy is currently incomplete for `systemd-importd`, used by
  `systemd-sysupdate`, which thus prevent us from running updates as a service
  in the background for now. See:
  - <https://github.com/fedora-selinux/selinux-policy/pull/2604>
  - <https://github.com/fedora-selinux/selinux-policy/issues/2622>

See: [Current limitation of systemd-sysupdate](https://travier.github.io/fedora-sysexts/#current-limitation-of-systemd-sysupdate)

## What about bootc?

In the future, this should be integrated into bootc directly (see
[bootc#7](https://github.com/bootc-dev/bootc/issues/7)). In the meantime, we
are prototyping the interface and usage here.

## Licenses

See: [LICENSES](LICENSES).
