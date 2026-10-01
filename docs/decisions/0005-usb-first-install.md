# ADR-0005: Install to a USB stick first; never touch the NVMe

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The only internal drive is a 256 GB NVMe holding the owner's LUKS-encrypted btrfs Arch install and its 2 GB ESP. A bug in an early disk driver could destroy that data.

## Decision

Horus boots and runs **from a USB stick** (selected with F9). The NVMe driver is **read-only** (write paths compiled out) until the owner explicitly approves a dedicated Horus partition. `xtask flash` refuses NVMe targets.

## Consequences

- The owner's data and Arch boot path stay safe.
- The kernel needs xHCI + USB mass storage early (v0.4) to reach its own disk; until then everything comes from an initrd loaded by Limine.
- USB storage is slower than NVMe; performance work on storage waits for the NVMe partition.

## Alternatives considered

- Shrink btrfs and add a partition now: risky resizing of an encrypted volume.
- ESP-only: no persistent storage.
