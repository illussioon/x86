# Full Arch native audit findings

The Arch profile in upstream v86 configures `filesystem.baseurl` as `../output/images/arch/` and `filesystem.basefs` as `../output/images/fs.json`, then boots with `root=host9p rootfstype=9p rootflags=trans=virtio,cache=loose` and `init=/usr/bin/init-openrc`.

The supplied saved state contains 98,013 inodes and 772 active 9P fids. The v86 filesystem state has four local inodedata entries (inode IDs 98005, 98008, 98011, 98012) backed by typed buffers 12–15. Most regular files are `STATUS_ON_STORAGE` entries identified by SHA-256, so the compressed saved state does not itself contain all root filesystem file contents. A native full boot therefore needs either the generated v86 filesystem manifest plus content blobs, or a native host filesystem backend that can serve equivalent data.

The upstream documentation points to generated build output rather than files committed in `copy/v86`. The apparent deployed URL `https://copy.sh/v86/output/images/fs.json` currently returns HTTP 404, so the exact Arch filesystem asset source must be recovered from the upstream build/image pipeline or supplied as an additional resource.

The current native core has CPU state restore and placeholder host callbacks, but no PCI/virtio queue transport, no 9P server, and no full device bus. The immediate implementation order is: resource/backend representation for fs manifest and blobs; VirtIO queue descriptor traversal; PCI configuration and BAR routing; virtio-9p request handling; virtio-console/serial I/O; then timer/PIT and remaining devices.

## External references

The upstream v86 Arch documentation is available at https://github.com/copy/v86/blob/master/docs/archlinux.md. It documents generation of `fs.json` with `tools/fs2json.py`, content blobs with `tools/copy-to-sha256.py`, a filesystem base URL under `output/images/arch/`, and the alternative raw disk artifact `output/images/arch.img`. The deployed `https://copy.sh/v86/output/images/fs.json` path was checked during this audit and returned 404.

The v86 Linux 9P requirements are documented at https://github.com/copy/v86/blob/master/docs/linux-9p-image.md: the guest kernel needs `CONFIG_VIRTIO_PCI`, `CONFIG_NET_9P`, `CONFIG_NET_9P_VIRTIO`, and `CONFIG_9P_FS`, and the root mount uses the `host9p` tag with `trans=virtio`.

## Current native device milestone

Commit `e65c96c` adds the native device-bus foundation and commit `21a3aea` adds minimal PCI config discovery; both are pushed to `master`. Native tests currently cover reset-vector HLT and a host-directory 9P version/attach/walk/open/read round trip.

The remaining transport blocker is PCI capability/BAR mapping. Upstream `src/virtio.js` creates common, notification, ISR, device-specific, and PCI configuration capabilities in separate BAR regions. The native callbacks `mmap_read8/16/32` and `mmap_write8/16/32` are still stubs, so the next implementation must route BAR addresses to the VirtIO common/notify/ISR/device-specific handlers before a guest can discover and use the restored 9P device.
