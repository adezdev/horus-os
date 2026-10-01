# ADR-0016: Own structured shell

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

Horus needs a command line. The native API is typed and capability-based.

## Decision

Build **`horus-shell`**, a shell where commands pass **typed data** (records, tables, lists) instead of text, in the spirit of Nushell and PowerShell. POSIX `sh` comes later from ported software. See [userspace.md](../architecture/userspace.md#shell).

## Consequences

- Fits the native API; built-ins return structured data.
- Not script-compatible with `sh`; ported software needs a ported POSIX shell.

## Alternatives considered

- POSIX shell.
- Port Nushell.
