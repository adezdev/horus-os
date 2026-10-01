# Kernel

Crate: `horus-kernel`. Target: `x86_64-unknown-none`, `#![no_std]`,
built with the pinned nightly toolchain. Related decisions:
[ADR-0003](../decisions/0003-hybrid-kernel.md),
[ADR-0006](../decisions/0006-hybrid-aware-smp-scheduler.md).

## Memory

### Virtual address layout (4-level paging, 48-bit)

```
0x0000_0000_0000_0000 ┐
        ...           │ user space (lower half, 128 TiB)
0x0000_7fff_ffff_ffff ┘
        (non-canonical hole)
0xffff_8000_0000_0000 ┐ direct map of all physical memory (HHDM),
        ...           │ 1 GiB pages where possible, NX, no user access
                      ┘
0xffff_c000_0000_0000   kernel heap / vmalloc area
0xffff_e000_0000_0000   MMIO mappings (uncached / write-combining)
0xffff_f000_0000_0000   per-CPU areas and kernel stacks (with guard pages)
0xffff_ffff_8000_0000   kernel image (text R-X, rodata R--, data RW-),
                        randomized by Limine KASLR
```

The exact bases are constants in `horus-kernel` and the source of
truth. Limine's HHDM offset is used during early boot. The kernel
then builds its own direct map at a fixed base.

### Physical memory

- **Buddy allocator** for page frames, built from the Limine memory map.
- **Per-CPU page caches** in front of the buddy allocator to avoid lock
  contention on the hot path.
- Bootloader-reclaimable memory is freed once nothing references it.

### Kernel heap

- **Slab allocators** for common object sizes and hot kernel objects
  (threads, handles, IPC messages).
- Large allocations come straight from the page allocator, mapped into
  the vmalloc area.
- Implements `GlobalAlloc` so `alloc::{Box, Vec, Arc}` work in the kernel.
- Allocation failure returns an error in syscall paths; the kernel does
  not panic on out-of-memory caused by userspace.

### User address spaces

- One PML4 per process, with the kernel half shared.
- **PCID** tags each address space so switches don't flush the TLB.
- **Demand paging** and **copy-on-write** (needed for POSIX `fork`, and
  for zero-copy sharing).
- Memory objects (VMOs) can be mapped into several address spaces:
  shared window buffers, ring buffers, DMA buffers.
- TLB shootdowns via IPIs, batched per operation.

## CPU setup

| Feature                  | Setting                                                       |
| ------------------------ | ------------------------------------------------------------- |
| GDT/TSS                  | Per CPU; IST stacks for `#DF`, NMI, `#MC`                     |
| SMEP / SMAP / UMIP       | On from v0.3; `stac`/`clac` only in user-copy helpers         |
| NX                       | On; W^X enforced for kernel and user mappings                 |
| FSGSBASE                 | On; GS holds the per-CPU pointer in the kernel                |
| XSAVE                    | Eager save/restore with `XSAVES`/`XSAVEOPT`; kernel code is built without SSE/AVX except in marked regions |
| Syscall entry            | `syscall`/`sysret` via `STAR`/`LSTAR`/`FMASK` MSRs             |
| Speculative execution    | Microcode-provided mitigations (IBRS/eIBRS, etc.) applied based on CPUID |

## Interrupts and time

- **x2APIC** on every CPU; the legacy 8259 PIC is masked.
- **IOAPIC** for legacy and ACPI-routed interrupts (`MADT` overrides
  respected); **MSI/MSI-X** for all PCIe devices.
- **Clock source:** invariant TSC. Frequency from `CPUID.15h`
  (`tsc_known_freq`), cross-checked against the HPET.
- **Timer:** LAPIC in **TSC-deadline** mode, one per CPU. The kernel is
  **tickless**: it programs the next deadline, with no periodic tick.
  `arat` means the timer keeps running in deep C-states.
- **Wall clock:** CMOS RTC (or UEFI time at boot), then network time.
- Interrupt handlers do minimal work and defer the rest to a per-CPU
  softirq/work queue or wake a driver thread. For userspace drivers, an
  interrupt signals an IRQ capability and the driver thread runs.

## SMP and the scheduler

Topology on this laptop: **2 P-cores × 2 threads + 4 E-cores = 8 CPUs.**

### Core detection

- At AP startup, each CPU reads `CPUID.1Ah` (core type: Core = P-core,
  Atom = E-core) and `CPUID.Bh/1Fh` (SMT and core topology).
- The **Hardware Feedback Interface** (HFI, `CPUID.06h`) exposes a
  memory table the CPU updates with each core's current performance and
  efficiency capability. The scheduler reads it on HFI interrupts.

### Design

- **Per-CPU run queues** with work stealing; no global run-queue lock.
- **Scheduling classes**, in priority order:
  1. *Realtime*: compositor frame thread, audio mixer, input server.
     Fixed priority, bounded budget.
  2. *Interactive*: the focused app and its helpers. Gets a latency
     boost on wake-up (EEVDF-style virtual deadlines).
  3. *Normal*: everything else.
  4. *Background*: indexers, builds, updates.
- **Placement policy:**
  - Realtime and interactive threads prefer **P-cores**.
  - Background threads prefer **E-cores** and only move to a P-core
    when the E-cores are saturated and a P-core is idle.
  - Avoid putting two busy threads on the sibling SMT threads of one
    P-core while an E-core is idle.
  - HFI capabilities adjust the weights when thermal limits change
    which cores are fast.
- **Focus-driven priority:** the compositor tells the scheduler which
  process owns the focused window, and its threads get the
  interactive class.
- **Idle:** `MWAIT` with C-state hints from ACPI `_CST`; HWP (Speed
  Shift) energy-performance preference set per class.

## Processes, threads, handles

- A **process** owns an address space and a **handle table**.
- A **thread** has a kernel stack, saved register and XSAVE state, a
  scheduling class, and a CPU affinity mask.
- **Handles** are indices into the handle table, each with **rights**
  (read, write, map, transfer, duplicate, ...). Rights can only be
  reduced when a handle is duplicated or transferred.
- Kernel objects are reference-counted and destroyed when the last
  handle is closed.

## IPC

Two mechanisms, chosen per use case:

1. **Channels (synchronous call/reply):** small messages (up to
   4 KiB inline) plus up to 64 transferred handles. A call that blocks
   on a reply **donates** the caller's time slice to the server thread,
   so a round trip is a direct switch, not two scheduler decisions.
2. **Rings (asynchronous, shared memory):** submission and completion
   queues in a shared memory object. A single syscall (or none, when the
   peer is polling) submits a batch. Used for files, networking, input
   events, and audio.

Waiting: a thread can wait on several handles at once (channels, rings,
IRQs, timers, process exit) with one syscall.

## System calls

- Entry via `syscall`; number in `rax`, arguments in `rdi, rsi, rdx,
  r10, r8, r9`; result in `rax` as `Result`-like (negative = error).
- Numbers and argument structs live in **`horus-abi`**, shared by the
  kernel and userspace, so they cannot drift apart.
- Every pointer argument goes through `copy_from_user`/`copy_to_user`
  helpers that check ranges and use SMAP-safe `stac`/`clac`.
- The ABI is **not stable** before 1.0. Userspace links against
  `horus-rt`/`std`, never against raw numbers.

## Panics and diagnostics

- A kernel panic stops all other CPUs (NMI IPI), then draws a panic
  screen: message, location, CPU, registers, and a **symbolized
  backtrace** (the kernel embeds a compact symbol table). Today (v0.1)
  the screen shows the message, location, and kernel version; CPU,
  registers, and the backtrace arrive with SMP and exceptions in v0.2.
- The same report goes to the debug log and, from v0.4, to a
  persistent crash log on the USB ESP (see
  [debugging.md](../development/debugging.md)).
