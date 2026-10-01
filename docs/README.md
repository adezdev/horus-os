# Horus documentation

Horus is a from-scratch, 64-bit operating system written in Rust. It is
built for one machine first, the owner's **HP Laptop 15-fd0xxx**, with
the long-term goal of becoming a **daily-driver OS** that is fast and
has a modern desktop.

> Status: pre-code. These documents record the design decisions made
> before the first line of kernel code. Treat them as the source of
> truth, and update them when a decision changes.

## At a glance

| Area               | Decision                                                        |
| ------------------ | --------------------------------------------------------------- |
| Goal               | Long-term daily driver on the HP 15-fd0xxx                      |
| Architecture       | x86_64 only (for now)                                           |
| Language           | Rust (kernel and userspace)                                     |
| Kernel model       | Hybrid: hot-path drivers in kernel, the rest in userspace       |
| Boot               | Limine boot protocol, UEFI                                      |
| Install target     | USB stick first; Arch on the NVMe is never touched              |
| Scheduler          | SMP, aware of P-cores and E-cores (Intel Thread Director / HFI) |
| System API         | Native capability-based API plus a POSIX compatibility layer    |
| Executables        | ELF64, statically linked first                                  |
| Runtime            | Horus target for Rust `std`; small C libc for ported software   |
| Filesystem         | Custom copy-on-write, checksummed FS; FAT32 for ESP/USB         |
| Encryption         | Full-disk encryption                                            |
| Desktop            | Tiling + floating compositor, fully themeable                   |
| Graphics           | Framebuffer + AVX2 software compositor, then native Intel GPU   |
| GUI toolkit        | Own retained-mode Rust toolkit                                  |
| Shell              | Own structured-data shell                                       |
| First network      | Anker USB-C Ethernet adapter (Realtek RTL8153)                  |
| Build              | Cargo workspace + `cargo xtask`                                 |
| Hosting            | Public GitHub: `adezdev/horus-os`, protected `main`             |
| Browser            | Port Ladybird                                                   |
| Debugging          | Single machine: QEMU + GDB, device passthrough, logs on the USB stick |
| Security extras    | Capabilities, kernel hardening, IOMMU, sandboxing, ASLR, CET, verified image |
| Firmware blobs     | Not in the repo; hash-checked copies from `/usr/lib/firmware`   |
| Disk plan          | USB stick → fast USB drive → NVMe dual boot at v1.0             |
| Versioning         | SemVer; plain technical component names                         |
| Builders           | Claude Code and Codex, sharing `AGENTS.md`; the owner reviews   |
| License            | MIT OR Apache-2.0                                               |

## Map

### Project
- [Vision and principles](vision.md): what Horus is, what it is not, and how trade-offs get decided
- [Target hardware](hardware.md): every device in the laptop and its driver plan
- [Roadmap](roadmap.md): milestones from v0.1 to v1.0
- [Open questions](open-questions.md): decisions still waiting on the owner (none right now)

### Architecture
- [Overview](architecture/overview.md): layers, components, kernel vs userspace split
- [Boot](architecture/boot.md): firmware, Limine, USB layout, early init
- [Kernel](architecture/kernel.md): memory, interrupts, timers, SMP, scheduler, IPC, syscalls
- [Drivers](architecture/drivers.md): driver model and per-device plan
- [Storage](architecture/storage.md): VFS, Horus FS, FAT32, encryption
- [Userspace](architecture/userspace.md): native API, POSIX layer, Rust `std`, ELF, shell
- [Desktop](architecture/desktop.md): compositor, window management, toolkit, theming
- [Security](architecture/security.md): threat model, decided and recommended protections

### Development
- [Setup](development/setup.md): toolchain, packages, repository layout, `xtask` commands
- [Debugging](development/debugging.md): QEMU + GDB and other ways to debug on the laptop
- [Workflow](development/workflow.md): how Claude, Codex, and the owner work together

### Decisions
- [Architecture Decision Records](decisions/README.md): the reasoning behind each choice
