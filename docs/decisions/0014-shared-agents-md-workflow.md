# ADR-0014: Shared AGENTS.md workflow for Claude Code and Codex

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

Two AI agents will write most of the code. Without shared conventions they'll drift apart and conflict.

## Decision

Keep one **`AGENTS.md`** at the repo root as the single source of conventions; `CLAUDE.md` only imports it (`@AGENTS.md`). Both agents follow the docs and ADRs; the owner reviews and merges every PR. See [workflow.md](../development/workflow.md).

## Consequences

- One set of rules; no duplicated instructions.
- Either agent can work on any subsystem; issue assignment prevents overlap.
- Docs must stay current, because they are the agents' memory.

## Alternatives considered

- Split subsystems per agent.
- One agent builds, the other reviews.
