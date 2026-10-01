# Architecture Decision Records

Each file records one decision: its context, the choice, and its consequences. The process is described in [workflow.md](../development/workflow.md#architecture-decision-records).

To add one, copy [template.md](template.md) to the next number.

| ADR | Decision | Status |
| --- | -------- | ------ |
| [0001](0001-x86_64-first-on-this-laptop.md) | Target x86_64 and the HP 15-fd0xxx first | Accepted |
| [0002](0002-rust.md) | Rust as the implementation language | Accepted |
| [0003](0003-hybrid-kernel.md) | Hybrid kernel | Accepted |
| [0004](0004-limine-boot-protocol.md) | Limine boot protocol | Accepted |
| [0005](0005-usb-first-install.md) | Install to a USB stick first; never touch the NVMe | Accepted |
| [0006](0006-hybrid-aware-smp-scheduler.md) | SMP scheduler aware of P-cores and E-cores | Accepted |
| [0007](0007-native-api-with-posix-layer.md) | Native capability-based API with a POSIX layer | Accepted |
| [0008](0008-custom-cow-filesystem.md) | Custom copy-on-write filesystem, plus FAT32 | Accepted |
| [0009](0009-rust-std-port-and-elf.md) | Rust std port; ELF64, static first | Accepted |
| [0010](0010-desktop-stack.md) | Desktop: tiling + floating compositor, staged graphics, own toolkit, full theming | Accepted |
| [0011](0011-usb-ethernet-first-network.md) | Anker USB-C Ethernet (RTL8153) as the first network device | Accepted |
| [0012](0012-full-disk-encryption.md) | Full-disk encryption | Accepted |
| [0013](0013-cargo-workspace-and-xtask.md) | Cargo workspace with xtask | Accepted |
| [0014](0014-shared-agents-md-workflow.md) | Shared AGENTS.md workflow for Claude Code and Codex | Accepted |
| [0015](0015-semver-and-plain-naming.md) | SemVer and plain technical component names | Accepted |
| [0016](0016-structured-shell.md) | Own structured shell | Accepted |
| [0017](0017-dual-mit-apache-license.md) | License under MIT OR Apache-2.0 | Accepted |
| [0018](0018-ladybird-browser.md) | Port Ladybird as the web browser | Accepted |
| [0019](0019-security-hardening-set.md) | Adopt a security hardening set | Accepted |
| [0020](0020-single-machine-debugging.md) | Debug on a single machine | Accepted |
| [0021](0021-disk-layout-phases.md) | Disk layout in phases | Accepted |
| [0022](0022-firmware-blobs.md) | Firmware blobs outside the repository | Accepted |
| [0023](0023-repository-settings.md) | GitHub repository settings | Accepted |

Pending decisions are tracked in [open-questions.md](../open-questions.md).
