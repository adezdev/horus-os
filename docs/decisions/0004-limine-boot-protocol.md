# ADR-0004: Limine boot protocol

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The laptop already boots Arch with Limine 12.8.0 on UEFI. Horus needs a way to get from firmware to a 64-bit kernel.

## Decision

Boot with the **Limine boot protocol** using the `limine` crate. The USB stick carries its own Limine copy.

## Consequences

- The kernel starts in 64-bit long mode, higher-half, with framebuffer, memory map, HHDM, RSDP, SMP startup, and modules provided. That saves weeks of bootloader work.
- Matches the owner's existing setup; a Horus entry can later go in the NVMe `limine.conf`.
- Horus depends on Limine's protocol revisions; `xtask` checks compatibility.

## Alternatives considered

- Own UEFI bootloader (`uefi-rs`): more control, more work, no benefit yet.
- Multiboot2/GRUB: starts in 32-bit mode; not used on this machine.
