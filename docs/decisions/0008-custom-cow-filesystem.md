# ADR-0008: Custom copy-on-write filesystem, plus FAT32

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

Horus needs a root filesystem. The owner wants a native design.

## Decision

Build **Horus FS**: copy-on-write B+trees, per-block checksums, extents, snapshots, TRIM-aware, implemented in a shared `horus-fs-format` crate used by the kernel and host tools. Implement **FAT32** for ESPs and USB exchange. See [storage.md](../architecture/storage.md).

## Consequences

- Crash-consistent by design; snapshots enable safe updates.
- Significant work: needs a written on-disk spec, `mkfs`, `fsck`, and fuzzing.
- Linux can't read Horus FS natively; host tools in `tools/` fill the gap.

## Alternatives considered

- ext2/ext4: well documented, but old design.
- FAT32 only: no permissions or crash safety.
- Reading btrfs: also needs LUKS; deferred.
