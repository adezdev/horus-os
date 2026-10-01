# ADR-0001: Target x86_64 and the HP 15-fd0xxx first

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

Horus needs a first platform. The owner wants a daily driver on their own laptop, an HP 15-fd0xxx with an Intel i3-1315U (see [hardware.md](../hardware.md)).

## Decision

Target **x86_64 only**, and tune for **this exact laptop**. Other machines and architectures are not supported until after 1.0.

## Consequences

- Drivers, the scheduler, and power management can be built for known hardware instead of general cases.
- x86_64 has the best documentation, emulator support (QEMU/KVM on the same machine), and OSDev resources.
- Arch-specific code still goes under `kernel/src/arch/x86_64/` so a later port is possible, but no abstraction is built ahead of need.

## Alternatives considered

- AArch64, RISC-V: no matching hardware on hand.
- Multi-arch from day one: cost without a user.
