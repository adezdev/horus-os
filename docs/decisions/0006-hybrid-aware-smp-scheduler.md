# ADR-0006: SMP scheduler aware of P-cores and E-cores

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The i3-1315U has 2 P-cores (with SMT) and 4 E-cores. Running interactive work on an E-core noticeably hurts responsiveness.

## Decision

Bring up **all 8 threads early** (v0.2) and build a scheduler with per-CPU run queues and classes (realtime, interactive, normal, background) that **places threads by core type** using `CPUID.1Ah` and the Hardware Feedback Interface. See [kernel.md](../architecture/kernel.md#smp-and-the-scheduler).

## Consequences

- Interactive and compositor threads get the fast cores.
- Locking and per-CPU design are right from the start; SMP is not retrofitted.
- QEMU can't emulate hybrid cores; placement policy is tested on the laptop and with host-side unit tests.

## Alternatives considered

- Plain SMP: wastes the P-cores.
- Single core first: SMP is very painful to retrofit.
