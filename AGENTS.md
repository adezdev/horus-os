# AGENTS.md

Instructions for AI coding agents (Codex, Claude Code) working on Horus.
`CLAUDE.md` imports this file; keep all rules here.

## Project

Horus is a from-scratch x86_64 operating system written in Rust,
targeting one machine first: the owner's HP Laptop 15-fd0xxx
(i3-1315U, 8 GB, Intel UHD, NVMe, RTL8852BT Wi-Fi). Long-term goal:
a fast daily-driver OS with a modern tiling + floating desktop.

**Status: pre-code.** The design lives in [`docs/`](docs/README.md).
Read the relevant docs and ADRs before changing anything they describe.

| Topic                       | Read                                   |
| --------------------------- | -------------------------------------- |
| Decisions and reasoning     | `docs/decisions/` (ADRs)               |
| Hardware and driver plan    | `docs/hardware.md`                     |
| Milestones                  | `docs/roadmap.md`                      |
| Kernel, drivers, storage, userspace, desktop, security | `docs/architecture/` |
| Toolchain and `xtask`       | `docs/development/setup.md`            |
| Debugging                   | `docs/development/debugging.md`        |
| Workflow and conventions    | `docs/development/workflow.md`         |

## Commands

Planned (they don't exist until the workspace is created in v0.1):

```sh
cargo xtask build        # kernel + userspace
cargo xtask run          # boot in QEMU (KVM, OVMF, USB storage)
cargo xtask run --gdb    # paused, GDB stub on :1234
cargo xtask test         # kernel tests in headless QEMU
cargo xtask ci           # fmt, clippy -D warnings, build, test, cargo deny
```

Run `cargo xtask ci` before every commit once it exists.

## Hard rules

1. **Never write to the laptop's NVMe** (`/dev/nvme0n1`) or its ESP, and
   never edit the host's `/boot/limine.conf`. The NVMe holds the
   owner's encrypted Arch install. Real-hardware testing uses the USB
   stick only.
2. **Never run `cargo xtask flash`, `dd`, or `cargo xtask run --vfio`**
   unless the owner is present and confirms the target device. Never
   pass the NVMe, GPU, or audio IOMMU groups to a VM.
3. **Never copy or translate GPL code**, including Linux drivers. Horus
   is MIT OR Apache-2.0. Linux may be read to learn hardware behavior;
   implementations come from specs and original work.
4. **The tree must build and boot after every commit.**
5. **Docs move with code.** If a change alters a design in `docs/`,
   update that doc in the same PR. New design choices get an ADR with
   status `Proposed`; only the owner accepts ADRs.
6. **Every `unsafe` block has a `// SAFETY:` comment** that explains why
   its invariants hold. Keep `unsafe` in small, dedicated modules.
7. **Never commit firmware blobs**; list them in `firmware/manifest.toml`.
8. **New dependencies** must be justified in the PR (purpose, license,
   `no_std` support) and use an allowed license: MIT, Apache-2.0,
   BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, 0BSD, CC0-1.0. Ask before
   MPL-2.0. Never GPL, LGPL, AGPL, SSPL, or unlicensed code.
9. **Every source file starts with**
   `// SPDX-License-Identifier: MIT OR Apache-2.0`.

## Code conventions

- Rust, pinned nightly via `rust-toolchain.toml`; kernel target
  `x86_64-unknown-none`.
- `cargo fmt`; `cargo clippy -- -D warnings`;
  `#![deny(unsafe_op_in_unsafe_fn)]`.
- No `unwrap()`/`expect()` on data from outside the kernel (hardware
  tables, devices, disks, userspace, network); return errors.
- Treat ACPI tables, USB descriptors, HID reports, disk contents, and
  packets as untrusted; parsers are bounds-checked and fuzzed.
- Crate names are plain and technical: `horus-kernel`, `horus-abi`,
  `horus-drv-<device>`, ...
- Host-testable logic (allocators, parsers, FS format, scheduler policy)
  lives in crates that also build for the host, so `cargo test` covers it.

## Git

- One task per branch: `<type>/<short-topic>` (e.g. `feat/x2apic-timer`).
  `main` is protected: changes land through PRs only.
- Conventional Commits. Subject `<type>(<scope>)<!>: <description>`,
  **50 characters max**, imperative mood, lowercase first letter, no
  trailing period. Types: `feat fix docs style refactor perf test build
  ci chore revert`. Body wrapped at 72 columns, explaining what and why.
- One logical change per commit; if the subject needs "and", split it.
- PR titles use the same format. PR descriptions say what was tested
  in QEMU and whether it was tested on the laptop.
- **No AI attribution** in commits or PRs: no `Co-authored-by` for AI
  tools, no "Generated with" footers or links.
