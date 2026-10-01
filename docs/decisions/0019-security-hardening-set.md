# ADR-0019: Adopt a security hardening set

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** Claude, at the owner's request

## Context

The owner chose full-disk encryption and asked for further protections. Twelve options (S1 to S12) are listed in [security.md](../architecture/security.md).

## Decision

**Adopt:** S1 capability access control, S2 kernel hardening baseline (SMEP/SMAP/UMIP/NX/W^X, guard pages, KASLR), S3 IOMMU confinement of drivers, S4 app sandboxing by default, S5 userspace ASLR, S6 CET shadow stacks and IBT, S8 verified read-only system image, S11 CSPRNG and entropy, S12 memory-safety discipline and fuzzing.

**Defer (revisit after 1.0):** S7 memory protection keys (little benefit until there's a concrete use), S9 Secure Boot signing (Secure Boot is off for Arch too; turning it on means signing both OSes), S10 TPM-sealed unlock (no TPM is visible: the firmware publishes no `TPM2` ACPI table, so Intel PTT is off or missing).

## Consequences

- The adopted set is cheap at the stage it's scheduled and hard to retrofit.
- IOMMU confinement (S3) is what makes userspace drivers actually safe, so it's needed in any case.
- Laptop theft is covered by encryption alone, since there's no TPM-based unlock.

## Alternatives considered

- Adopt everything now: Secure Boot and TPM work give little until 1.0.
- Encryption only: leaves apps and drivers unconfined.
