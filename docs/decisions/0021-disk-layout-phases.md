# ADR-0021: Disk layout in phases

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** Claude, at the owner's request

## Context

Horus starts on a USB stick ([ADR-0005](0005-usb-first-install.md)). The NVMe is 238 GB, of which Arch uses 78 GB. USB sticks are slow, and copy-on-write filesystems wear cheap flash quickly.

## Decision

Move through four phases; each later phase needs the owner's explicit go-ahead:

| Phase | When | Layout |
| ----- | ---- | ------ |
| 1 | v0.1 to v0.4 | USB stick: Limine ESP + initrd; no persistent storage needed |
| 2 | v0.5 to v0.9 | A **fast USB 3 drive**: a quality stick, or ideally a small external SSD. ESP + encrypted Horus FS |
| 3 | v1.0 | **Dual boot on the NVMe**: after a full backup, shrink the Arch btrfs and LUKS volume to ~150 GB and give Horus ~85 GB; add a `/Horus` entry to the existing Limine menu |
| 4 | After a month of daily use | The owner decides whether Horus replaces Arch |

## Consequences

- The owner's Arch install isn't touched until Horus is mature and a backup exists.
- Phase 2 may need a better USB drive. A cheap stick will work but will be slow and wear out.
- Shrinking LUKS and btrfs is a manual, careful procedure, to be documented step by step before phase 3.

## Alternatives considered

- Dual boot now: risk to Arch data while drivers are immature.
- Replace Arch at 1.0: no fallback while Horus is unproven.
