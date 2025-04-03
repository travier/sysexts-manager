# systemd system extensions manager

Work in progress manager for systemd system extensions (sysexts).

## Why?

Why do we need something else when we already have `systemd-sysupdate`?

While installing and updating via `systemd-sysupdate` *works* (as done with
[extensions.fcos.fr](https://extensions.fcos.fr/)), it also comes with a few
limitations:
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

## How to

TODO

## License

[MIT](LICENSE).
