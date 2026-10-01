# ADR-0007: Native capability-based API with a POSIX layer

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

A clean native API makes for a better system; existing software (shells, compilers, a browser) expects POSIX.

## Decision

Design a **native API** around capability handles, channels, and async rings. Provide **POSIX as a userspace library** on top of it for source-level porting. No Linux binary compatibility. See [userspace.md](../architecture/userspace.md).

## Consequences

- The kernel stays free of POSIX baggage (no global namespace, no ambient authority).
- `fork` needs copy-on-write support in the kernel; `posix_spawn` is preferred.
- Some POSIX semantics (signals, permissions) are emulated, not exact.

## Alternatives considered

- Full POSIX kernel: locks in Unix design.
- Linux ABI emulation: huge surface.
- Native only: no software to port.
