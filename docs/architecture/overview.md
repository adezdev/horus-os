# Architecture overview

Horus uses a **hybrid kernel**. The kernel holds everything that is on
the performance-critical path or that must be trusted anyway: memory,
scheduling, IPC, interrupts, the storage stack, and the display engine.
Every other driver, and every service, runs as an isolated userspace
process that talks to the kernel and to other processes through
**capabilities** and **IPC**.

## Layers

```
┌──────────────────────────────────────────────────────────────────────┐
│ Applications                                                         │
│   terminal · launcher · file manager · settings · ported software    │
├──────────────────────────────────────────────────────────────────────┤
│ Libraries                                                            │
│   Rust std (Horus target) · horus-ui toolkit · POSIX layer · C libc  │
├──────────────────────────────────────────────────────────────────────┤
│ System services (userspace)                                          │
│   init/service manager · compositor · input server · audio server    │
│   network stack · power daemon · shell                               │
├──────────────────────────────────────────────────────────────────────┤
│ Userspace drivers (isolated, IOMMU-confined DMA)                     │
│   I2C-HID touchpad/touchscreen · GPIO · I2C · USB class drivers      │
│   (Ethernet, HID, UVC, BT) · HDA/SOF audio · Wi-Fi                   │
╞══════════════════════════ syscall boundary ══════════════════════════╡
│ Kernel                                                               │
│   memory (paging, allocators, COW) · scheduler (P/E-aware SMP)       │
│   IPC + capabilities · syscalls · interrupts, APIC, timers           │
│   ACPI (tables + AML) · PCIe · IOMMU · VFS · Horus FS · FAT32        │
│   block layer · encryption · NVMe · xHCI + USB storage · GPU display │
├──────────────────────────────────────────────────────────────────────┤
│ Limine (boot only) · UEFI firmware                                   │
├──────────────────────────────────────────────────────────────────────┤
│ HP 15-fd0xxx: i3-1315U · 8 GB · NVMe · Intel UHD · RTL8852BT · ...   │
└──────────────────────────────────────────────────────────────────────┘
```

## What lives where

The rule: **code goes in the kernel only if it is on a latency-critical
path, needed to boot to the point where userspace runs, or so tied to
memory or interrupt management that splitting it costs more than it
gains.** Everything else is a userspace driver or service.

| In the kernel                       | Why                                                      |
| ----------------------------------- | -------------------------------------------------------- |
| Memory manager, scheduler, IPC      | Core of the kernel                                       |
| Interrupts, APIC, timers, SMP       | Core of the kernel                                       |
| ACPI tables + AML interpreter       | Power management and device discovery need it early     |
| PCIe, MSI-X, IOMMU                  | Hands devices to drivers safely                          |
| NVMe, xHCI core, USB mass storage   | Storage hot path; also needed to mount the root FS       |
| Block layer, VFS, Horus FS, FAT32, encryption | Avoids IPC round-trips on every file access    |
| GPU display engine + memory manager | Frame timing and buffer sharing on the hot path          |
| Early PS/2 keyboard and framebuffer console | Needed before userspace exists (moved out later) |

| In userspace                        | Why                                                      |
| ----------------------------------- | -------------------------------------------------------- |
| I2C, GPIO, HID-over-I2C             | Low bandwidth; a crash must not panic the system         |
| USB class drivers (Ethernet, HID, UVC, Bluetooth) | Isolated; restartable                      |
| Audio (HDA, SOF), Wi-Fi             | Complex, firmware-heavy, best isolated                   |
| Network stack                       | Large attack surface; restartable                        |
| Compositor, input, audio, power servers | Policy belongs in userspace                          |

A crashed userspace driver is restarted by the service manager. Its
clients see an error on their IPC channel and reconnect.

## Core concepts

- **Capability (handle):** an unforgeable, per-process reference to a
  kernel object (process, thread, memory object, channel, IRQ, MMIO
  range, file, directory) with a set of rights. There is no ambient
  authority: a process can only use what it holds.
- **Channel:** a bidirectional IPC endpoint that carries messages and
  can transfer capabilities.
- **Ring:** a shared-memory submission/completion queue for high-volume
  async I/O (file, network, input, audio), in the style of io_uring.
- **Memory object:** a range of pages that can be mapped into one or
  more address spaces. Window buffers, audio buffers, and DMA buffers
  are all memory objects.

## Workspace crates (planned)

Names are plain and technical ([ADR-0015](../decisions/0015-semver-and-plain-naming.md)).

| Crate               | Kind        | Purpose                                         |
| ------------------- | ----------- | ----------------------------------------------- |
| `horus-kernel`      | binary      | The kernel                                      |
| `horus-abi`         | `no_std` lib | Syscall numbers, handle types, IPC message layouts (shared by kernel and userspace) |
| `horus-rt`          | `no_std` lib | Minimal userspace runtime (entry, syscalls, allocator) |
| `horus-fs-format`   | `no_std` lib | On-disk Horus FS structures (shared by kernel and tools) |
| `horus-init`        | binary      | First process; service manager                  |
| `horus-shell`       | binary      | Structured shell                                |
| `horus-compositor`  | binary      | Display server and window manager               |
| `horus-ui`          | lib         | GUI toolkit                                     |
| `horus-input`       | binary      | Input server                                    |
| `horus-drv-*`       | binary      | Userspace drivers (e.g. `horus-drv-i2c-hid`)    |
| `xtask`             | binary (host) | Build, image, run, test, flash                |

See [development/setup.md](../development/setup.md) for the directory layout.
