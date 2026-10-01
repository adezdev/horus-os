# Workflow

Horus is built by two AI coding agents, **Claude Code** and **Codex**,
directed and reviewed by the owner. This document is the contract
between all three. Related decision:
[ADR-0014](../decisions/0014-shared-agents-md-workflow.md).

## Roles

| Who        | Does                                                                 |
| ---------- | -------------------------------------------------------------------- |
| Owner      | Sets direction, makes the decisions in [open-questions.md](../open-questions.md), reviews and merges every PR, tests on the laptop |
| Claude Code | Implements tasks, writes tests and docs, reviews Codex's PRs on request |
| Codex      | Implements tasks, writes tests and docs, reviews Claude's PRs on request |

Either agent can work on any subsystem. To avoid conflicts, **only one
agent works on a given subsystem at a time**, tracked by GitHub issue
assignment.

## Shared instructions

- **`AGENTS.md`** at the repo root is the single source of conventions.
  Codex reads it natively.
- **`CLAUDE.md`** contains only `@AGENTS.md`, so Claude Code loads the
  same file. Never put rules in `CLAUDE.md` that aren't in `AGENTS.md`.
- `AGENTS.md` stays short (build commands, conventions, hard rules) and
  links into `docs/` for depth.
- Subdirectories may add their own `AGENTS.md` (e.g. `kernel/AGENTS.md`
  for kernel-specific rules); agents read the nearest one.

### Hard rules for agents (to copy into `AGENTS.md`)

1. **Never write to the laptop's NVMe** or its ESP, and never edit the
   host's `/boot/limine.conf`. Real-hardware testing uses the USB stick only.
2. **Never run `cargo xtask flash`, `dd`, or `cargo xtask run --vfio`**
   without the owner present and confirming the target device. Never
   pass the NVMe, GPU, or audio IOMMU groups to a VM
   ([ADR-0020](../decisions/0020-single-machine-debugging.md)).
3. **Never copy or translate GPL code** (Linux drivers). Horus is
   MIT OR Apache-2.0 ([ADR-0017](../decisions/0017-dual-mit-apache-license.md)).
   Linux may be read to learn hardware behavior; the implementation must
   come from specs and original work.
4. **The tree must boot** after every commit: `cargo xtask ci` passes.
5. **Docs move with code:** a change that alters a design described in
   `docs/` updates that doc in the same PR; a new design choice gets an ADR.
6. **`unsafe` needs a `// SAFETY:` comment** that explains why each
   invariant holds.
7. **Never commit firmware blobs**; add them to `firmware/manifest.toml`
   instead ([ADR-0022](../decisions/0022-firmware-blobs.md)).
8. **No new dependency** without stating its license, why it's needed,
   and whether it is `no_std`-compatible in the PR description. The
   license must pass the [dependency policy](#dependency-licenses).
9. **Every source file starts with an SPDX header:**
   `// SPDX-License-Identifier: MIT OR Apache-2.0`.

## Dependency licenses

Horus is MIT OR Apache-2.0, so everything linked into a Horus component
must be compatible with both. Checked in CI with `cargo deny`.

| License                                         | Policy                      |
| ----------------------------------------------- | --------------------------- |
| MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, 0BSD, CC0-1.0 | Allowed |
| MPL-2.0                                         | Ask the owner first (file-level copyleft) |
| GPL, LGPL, AGPL (any version), SSPL, no license | Not allowed                 |

Separate programs ported to run *on* Horus (a browser, a POSIX shell)
keep their own licenses and are not linked into Horus components.

## Task flow

```
issue (owner or agent drafts; owner approves)
  └─► branch  <type>/<short-topic>   e.g. feat/x2apic-timer
        └─► commits (Conventional Commits, one logical change each)
              └─► PR → CI green → second-agent review (optional) → owner review → squash or rebase merge
```

- Every task starts from a GitHub issue that names the milestone,
  acceptance criteria, and which docs it touches.
- Small PRs: aim for under ~400 changed lines excluding tests and
  generated files.
- The PR description says what was tested in QEMU and whether it was
  tested on the laptop.

## Commit and PR conventions

Conventional Commits, following the owner's house rules:

```
<type>(<scope>)<!>: <description>
```

- **Types:** `feat` `fix` `docs` `style` `refactor` `perf` `test`
  `build` `ci` `chore` `revert`. Lowercase. No others.
- **Scope:** lowercase noun naming the part of the code base, e.g.
  `kernel`, `mm`, `sched`, `ipc`, `acpi`, `nvme`, `xhci`, `fs`,
  `compositor`, `ui`, `shell`, `xtask`, `docs`.
- **Description:** imperative mood, lowercase first letter, no
  trailing period.
- **Subject line ≤ 50 characters**, hard limit. Must match:
  `^(?=.{1,50}$)(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._-]+\))?(!)?: [a-z].*[^.]$`
- **Body:** one blank line after the subject, wrapped at 72 columns,
  explains *what* and *why*, not *how*.
- **Footers:** `Refs: #12`, `Closes: #12`, `BREAKING CHANGE: ...`.
- One logical change per commit; if the subject needs "and", split it.
- PR titles follow the same format.
- **No AI attribution** in commits or PRs: no `Co-authored-by` for
  AI tools, no "Generated with" footers or links.

Examples:

```
feat(sched): prefer p-cores for interactive class
fix(xhci): handle port reset timeout on resume
docs(adr): record limine boot protocol decision
build(xtask): add usb flash safety checks
```

## Architecture Decision Records

- Live in [`docs/decisions/`](../decisions/README.md), numbered `NNNN-title.md`.
- Written when a choice is hard to reverse, affects several
  subsystems, or answers a question someone will ask later.
- Status values: `Proposed` → `Accepted` → (`Superseded by NNNN` | `Deprecated`).
- An agent may write an ADR as **Proposed**; only the owner accepts it.

## Definition of done

- [ ] `cargo xtask ci` passes (fmt, clippy with `-D warnings`, build, tests, `cargo deny`)
- [ ] New source files have the SPDX license header
- [ ] New behavior has tests (host unit tests or QEMU tests)
- [ ] Docs and ADRs updated
- [ ] Roadmap checkbox ticked if a milestone item is complete
- [ ] For hardware-facing changes: tested on the laptop, or the PR
      says it wasn't
