# ADR-0023: GitHub repository settings

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** Claude, at the owner's request

## Context

The code will be public on GitHub under the owner's account (`adezdev`, authenticated with `gh`).

## Decision

- Repository: **`adezdev/horus-os`** (the `-os` suffix makes it easier to find than plain `horus`). The local directory stays `horus/`.
- Default branch `main`, protected: changes only by PR, required CI status check, linear history (squash or rebase merges), no force pushes, no deletions.
- Outside contributions are welcome through PRs under the dual license ([ADR-0017](0017-dual-mit-apache-license.md)); the owner reviews every PR. A `CONTRIBUTING.md` is added with the first code.
- Issues use labels per milestone (`v0.1`, ...) and per subsystem (`kernel`, `drivers`, `desktop`, ...).
- Copyright line: `adezdev`.

## Consequences

- The protected branch keeps the tree bootable and the history clean.
- Creating the repository is a separate step, done when the owner asks.

## Alternatives considered

- Private repo: the owner chose public.
- Plain `horus` name: harder to find.
