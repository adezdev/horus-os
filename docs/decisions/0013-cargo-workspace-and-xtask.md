# ADR-0013: Cargo workspace with xtask

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The build must produce a kernel, userspace binaries, an initrd, and a bootable image, and launch QEMU, for both humans and agents.

## Decision

One **Cargo workspace**; all orchestration in a Rust **`xtask`** binary (`cargo xtask build|image|run|test|flash|ci`). No Makefiles or shell-script build logic. See [setup.md](../development/setup.md).

## Consequences

- One language and one entry point; easy for agents to discover and run.
- Cross-platform logic is testable Rust instead of shell.

## Alternatives considered

- Cargo + Makefile.
- Cargo + justfile.
