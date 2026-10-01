# ADR-0009: Rust std port; ELF64, static first

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

Userspace programs need a runtime and an executable format.

## Decision

Port **Rust `std`** to a new `x86_64-unknown-horus` target as the primary runtime; port a small permissive C libc (relibc or mlibc, decided later) for C software. Executables are **ELF64 static-PIE** first; dynamic linking comes later.

## Consequences

- Native apps are idiomatic Rust.
- ELF is produced natively by LLVM/Rust; static-PIE gives ASLR without a dynamic loader.
- `-Zbuild-std` ties userspace to the pinned nightly.

## Alternatives considered

- Port musl: assumes a Linux syscall layer.
- Write own libc: high effort.
- Custom executable format: no benefit.
