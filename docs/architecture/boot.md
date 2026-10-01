# Boot

Related decisions: [ADR-0004](../decisions/0004-limine-boot-protocol.md),
[ADR-0005](../decisions/0005-usb-first-install.md).

## Boot chain

```
Power on ─► AMI UEFI firmware ─► F9 boot menu ─► USB stick
        ─► EFI/BOOT/BOOTX64.EFI (Limine) ─► limine.conf ─► Horus entry
        ─► Limine loads kernel ELF + modules, sets up 64-bit long mode,
           higher-half mapping, HHDM, framebuffer, starts APs parked
        ─► horus-kernel _start (BSP, interrupts off)
```

The laptop already boots Arch with Limine 12.8.0 from the NVMe ESP.
**Horus does not touch that ESP or its `limine.conf`.** The USB stick
carries its own copy of Limine, so the Arch boot path stays as it is.
Later, a Horus entry can be added to the NVMe Limine menu once the
owner approves it.

## USB stick layout

`cargo xtask image` builds `target/horus.img`, which
`cargo xtask flash` writes to the stick.

```
GPT
└── Partition 1: EFI System Partition, FAT32, 512 MiB
    ├── EFI/BOOT/BOOTX64.EFI      ← from /usr/share/limine/
    ├── boot/limine/limine.conf
    ├── boot/horus/kernel         ← horus-kernel ELF
    └── boot/horus/initrd         ← initial ramdisk (init, drivers, shell)
└── Partition 2 (from v0.5): Horus FS, encrypted, rest of the stick
```

### `limine.conf`

```
timeout: 3

/Horus
    protocol: limine
    path: boot():/boot/horus/kernel
    module_path: boot():/boot/horus/initrd
    kaslr: yes
```

## Limine requests used

The kernel declares Limine requests (via the `limine` crate) in a
dedicated linker section:

| Request               | Gives the kernel                                          |
| --------------------- | --------------------------------------------------------- |
| Base revision         | Protocol version check                                    |
| Framebuffer           | Address, size, pitch, pixel format of the GOP framebuffer |
| Memory map            | Usable, reserved, ACPI, bootloader-reclaimable regions    |
| HHDM                  | Offset of the higher-half direct map                      |
| Executable address    | Physical and virtual base of the kernel (for KASLR)       |
| RSDP                  | ACPI root pointer                                         |
| EFI system table      | Kept for reading the UEFI memory map and runtime services later |
| MP (SMP)              | List of all CPUs (LAPIC IDs) with a goto address to start them |
| Module                | The initrd in memory                                      |
| Date at boot          | Wall-clock time before the RTC driver exists              |
| Stack size            | A bigger boot stack than the default                      |

## Kernel early init order

1. Check the Limine base revision; halt with a visible color on the
   framebuffer if it's unsupported.
2. Framebuffer console and log sink (QEMU `debugcon` port `0xE9` too).
3. GDT, TSS (with IST stacks for double fault, NMI, machine check), IDT.
4. Physical memory: build the frame allocator from the memory map.
5. Switch to the kernel's own page tables (direct map, kernel image
   with correct W^X permissions, guard pages).
6. Kernel heap.
7. ACPI static tables: `MADT`, `FADT`, `HPET`, `MCFG`, `DMAR`.
8. x2APIC on the BSP, IOAPIC routes, TSC-deadline timer.
9. Start the APs through the Limine MP response; each AP sets up its own
   GDT/TSS/IDT, APIC, per-CPU data, and reports its core type.
10. Scheduler on all CPUs; reclaim bootloader memory.
11. Unpack the initrd, start `horus-init` in ring 3.

## Early failure visibility

Without a serial port, anything that goes wrong before the console works
is invisible. Rules:

- The first kernel instructions fill the framebuffer with a solid color,
  and each later init stage changes it. A hang then shows *where* it
  stopped, even before text rendering works.
- Exceptions during early init print straight to the framebuffer with
  a minimal renderer that does not allocate.

## Future: installing to NVMe

Not before v1.0, and only with the owner's explicit approval and a
full backup ([ADR-0021](../decisions/0021-disk-layout-phases.md)):
shrink Arch to ~150 GB, add a ~85 GB Horus partition, copy the kernel
to the NVMe ESP under `/boot/horus/`, and add a `/Horus` entry to the
existing `limine.conf`.
