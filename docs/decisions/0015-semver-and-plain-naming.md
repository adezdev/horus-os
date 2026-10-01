# ADR-0015: SemVer and plain technical component names

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

Releases need version numbers and components need names. Egyptian component names and pharaoh codenames were offered and declined.

## Decision

Version releases with plain **SemVer** (one minor version per milestone before 1.0). Name components plainly: `horus-kernel`, `horus-compositor`, `horus-fs-format`, `horus-drv-<device>`, and so on. The Egyptian identity stays in the OS name and branding.

## Consequences

- Names say what things do; no lookup table.
- Versions communicate compatibility: 0.x means anything may break.

## Alternatives considered

- Egyptian names (Ra, Aten, Ptah, ...).
- Pharaoh release codenames.
- CalVer.
