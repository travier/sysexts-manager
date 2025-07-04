# systemd system extensions manager

Work in progress manager for systemd system extensions (sysexts).

## How to use

Install the `sysexts-manager` pre-built sysext:

```
$ VERSION="0.0.1" # sysexts-manager version
$ VERSION_ID="42" # Fedora release
$ ARCH="x86-64"   # or arm64
$ URL="https://github.com/travier/sysexts-manager/releases/download/sysexts-manager/"
$ sudo install -d -m 0755 -o 0 -g 0 /var/lib/extensions
$ curl --silent --fail --location "${URL}/sysexts-manager-${VERSION}-${VERSION_ID}-${ARCH}.raw" \
      | sudo bash -c "cat > /var/lib/extensions/sysexts-manager.raw"
$ sudo restorecon -RFv /var/lib/extensions
$ sudo systemctl enable systemd-sysext.service
$ sudo systemctl restart systemd-sysext.service
```

As `sysexts-manager` is a sysext, it is capable of managing itself:

```
$ sysexts-manager status
```

Install the tree sysext from [extensions.fcos.fr](https://extensions.fcos.fr):

```
$ sudo sysexts-manager add tree https://extensions.fcos.fr/extensions/tree
```

Update all sysexts managed by sysexts-manager:

```
$ sudo sysexts-manager update
```

Create temporary symlinks in `/run/extensions`:

```
$ sudo sysexts-manager symlinks
```

Ask systemd to refresh enabled sysexts:

```
$ sudo sysexts-manager refresh
```

List all sysexts managed by sysexts-manager:

```
$ sysexts-manager status
```

Remove the tree sysext and all installed images:

```
$ sudo sysexts-manager remove tree
$ sudo sysexts-manager refresh
```

## How does this work on boot?

We statically install a copy of the `sysexts-manager` sysext and enable
`systemd-sysext.service` to run on boot. When systemd loads the
`sysexts-manager` sysext on boot, it will trigger `sysexts-manager.service`
which will enable and load all other sysexts on demand.

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

## License

[MIT](LICENSE).
