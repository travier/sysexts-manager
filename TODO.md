## How to

- Install the sysexts-manager sysext.
- After each rpm-ostree/bootc update and before rebooting, run `sysexts-manager
  update`.

## Configuration files

/usr/lib/sysexts-manager/sysexts-manager.conf
/usr/lib/sysexts-manager/<name>.conf
/etc/sysexts-manager/sysexts-manager.conf
/etc/sysexts-manager/<name>.conf
/run/sysexts-manager/sysexts-manager.conf
/run/sysexts-manager/<name>.conf

```
name=strace
updatekind=matching
```

```
name=kubernetes-cri-o1.32
updatekind=matching
```

```
name=libvirtd
updatekind=matching
```

```
name=vscode
updatekind=latest
```

```
name=google-chrome
updatekind=version
tag=...
```

```
name=config
updatekind=git-branch
branch=main

updatekind=git-matching

updatekind=git-tag
tag=...
```

sysexts stored in `/var/lib/extensions.d`
symlinked at boot in `/run/extensions`
sysexts-manager extensions stored in `/var/lib/extensions/`

Boot:
- systemd-sysext.service enables the sysexts-manager service
- sysexts-manager service starts, looks at its configuration, enables sysexts
  as requested via symlinks in `/run/extensions`, then trigger a
  systemd-sysext.service restart, then run "post" commands for each sysexts.

Update:
- Looks at current state
- Cleanup old versions from /var/lib/extensions.d
- Download new versions as needed in /var/lib/extensions.d
- Update symlinks in /run/extensions as needed according to configuration
- Restarts systemd-sysext.service and runs "post" commands for each sysexts.
