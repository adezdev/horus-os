# ADR-0003: Hybrid kernel

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The owner wants speed and a modern desktop, which favors a monolithic kernel. A daily driver also has to survive driver bugs, which favors a microkernel.

## Decision

Use a **hybrid kernel**: memory, scheduling, IPC, interrupts, ACPI, PCIe, IOMMU, the storage stack (NVMe, xHCI, USB mass storage, block, VFS, filesystems, encryption), and the GPU display engine run in the kernel. Input, USB class drivers, audio, networking, Wi-Fi, and all policy run as **isolated userspace processes** with capability-granted MMIO, IRQ, and IOMMU-confined DMA. See [overview.md](../architecture/overview.md).

## Consequences

- Hot paths (disk, display) avoid IPC overhead.
- Complex, firmware-heavy, or rarely used drivers can crash and restart without a panic.
- Requires a good IPC design and VT-d support early (v0.5).

## Alternatives considered

- Monolithic: one driver bug takes down the system.
- Microkernel: IPC costs on every disk and display operation.
- Exokernel: too experimental for a daily driver.
