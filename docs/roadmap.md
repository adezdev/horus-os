# Roadmap

Versions follow [SemVer](https://semver.org). Until 1.0.0, every minor
version is one milestone and may break anything. Each milestone ends
with a tag, a short release note, and a boot test on the real laptop.

A milestone is **done** when:

- every listed item works in QEMU (automated where possible),
- it boots from the USB stick on the HP 15-fd0xxx,
- the docs and ADRs match what was built.

---

## v0.1: Boots on the laptop

Proves the toolchain, the boot path, and the feedback loop.

- [x] Cargo workspace, `rust-toolchain.toml`, `cargo xtask` skeleton
- [x] Kernel builds for `x86_64-unknown-none`, linked higher-half
- [ ] Limine boot protocol: framebuffer, memory map, HHDM, RSDP, SMP info, modules
- [x] Bootable USB image built by `cargo xtask image` (GPT + FAT32 ESP + Limine)
- [ ] Framebuffer console with a bitmap font; Horus logo on boot
- [ ] Prints memory map, CPU model, and core types (P/E via `CPUID.1Ah`)
- [ ] PS/2 keyboard input echoed to the screen (polled is fine)
- [ ] Logging to QEMU `debugcon` / serial and to the framebuffer
- [ ] Panic screen with message and source location
- [x] GitHub Actions: build, `clippy`, `fmt`, and a headless QEMU boot test that greps the log

## v0.2: Kernel core

- [ ] GDT, TSS, IDT; all CPU exceptions handled with readable dumps
- [ ] Physical frame allocator (buddy) from the Limine memory map
- [ ] Kernel page tables: direct map with 1 GiB pages, guard pages, NX
- [ ] Kernel heap (slab allocators + large-object path); `alloc` works
- [ ] x2APIC; IOAPIC from `MADT`; legacy PIC masked
- [ ] TSC as the clock source (frequency from `CPUID.15h`); TSC-deadline timer
- [ ] ACPI static tables parsed (`MADT`, `HPET`, `MCFG`, `FADT`, `DMAR`)
- [ ] SMP: all 8 threads online, per-CPU data via GS base
- [ ] Kernel test harness running in QEMU (`cargo xtask test`)
- [ ] Symbolized kernel backtraces on panic

## v0.3: Userspace

- [ ] Ring 3, `syscall`/`sysret`, SMEP/SMAP/UMIP enabled
- [ ] Address spaces with PCID; demand paging; copy-on-write
- [ ] ELF64 static loader; initial ramdisk (Limine module)
- [ ] Processes, threads, preemptive scheduler with per-CPU run queues
- [ ] Core-type-aware placement (P-cores for interactive, E-cores for background)
- [ ] Capability handles; IPC (call/reply + shared-memory rings)
- [ ] `init` process; userspace "hello world" via the native API
- [ ] `horus-rt`: a minimal `no_std` userspace runtime

## v0.4: Buses and storage hardware

- [ ] PCIe enumeration via ECAM; MSI / MSI-X
- [ ] xHCI driver; USB device enumeration; hubs
- [ ] USB mass storage (Bulk-Only Transport; UAS later)
- [ ] GPT parsing; FAT32 read/write (the USB stick's ESP)
- [ ] Boot logs and crash reports saved to the stick; `cargo xtask logs`
- [ ] NVMe driver, **read-only** (see [ADR-0005](decisions/0005-usb-first-install.md))
- [ ] Block-layer request queue and page cache

## v0.5: Filesystem, encryption, Rust `std`

- [ ] VFS with path resolution, mounts, and capability-scoped directories
- [ ] Horus FS v1 on a fast USB drive (copy-on-write, checksums; [ADR-0021](decisions/0021-disk-layout-phases.md))
- [ ] Encrypted volume layer (AES-XTS with AES-NI/VAES, Argon2id passphrase)
- [ ] Early-boot passphrase prompt on the framebuffer
- [ ] VT-d IOMMU on, so userspace drivers get confined DMA
- [ ] Horus target for Rust `std` (files, threads, time, env, processes)
- [ ] Structured shell (text console) with core built-in commands

## v0.6: ACPI and input

- [ ] AML interpreter; `_STA`/`_INI`/`_CRS`/`_PRW` evaluation; ACPI device tree
- [ ] Embedded controller, power button, lid switch, AC adapter, battery
- [ ] Userspace driver framework (MMIO and IRQ capabilities)
- [ ] Intel GPIO (`INTC1055`), DesignWare I2C, HID-over-I2C
- [ ] Touchpad: pointer, tap-to-click, two-finger scroll, multi-finger gestures
- [ ] Touchscreen: touch events
- [ ] Input server: one event stream for keyboard, pointer, touch, and gestures; keymaps

## v0.7: Desktop

- [ ] Display server/compositor on the framebuffer, AVX2 blending, damage tracking
- [ ] Tiling + floating window management, workspaces, keyboard bindings
- [ ] Horus UI toolkit: layout, text (shaping and rasterizing), core widgets
- [ ] Theme engine with hot reload; a default theme
- [ ] Terminal emulator app running the structured shell
- [ ] Launcher, status bar (time, battery, network), notifications
- [ ] Touchpad gestures for workspace switching

## v0.8: Networking

- [ ] USB CDC-ECM driver (Anker RTL8153); CDC-NCM
- [ ] Network stack: Ethernet, ARP, IPv4/IPv6, ICMP, UDP, TCP
- [ ] DHCP, DNS resolver
- [ ] TLS library
- [ ] POSIX layer for Ladybird: Unix sockets with fd passing, `pthread`, shared memory; Clang + libc++ targeting Horus
- [ ] Start the Ladybird port
- [ ] Network time sync

## v0.9: Power and audio

- [ ] S3 suspend/resume (`deep`), lid-close to sleep
- [ ] Backlight control and brightness keys
- [ ] CPU power: HWP, `MWAIT` C-states, tickless idle
- [ ] Battery status and estimates in the status bar
- [ ] HDA (legacy mode) + ALC236: speakers, headphone jack, jack detection
- [ ] Audio server with low-latency mixing
- [ ] SOF DSP for the two digital microphones (stretch goal)
- [ ] Checkpoint: re-evaluate Ladybird vs. Servo ([ADR-0018](decisions/0018-ladybird-browser.md))

## v0.10: Native Intel graphics

- [ ] Display engine modesetting for eDP and HDMI; atomic page flips; vblank
- [ ] External monitor hotplug
- [ ] GPU memory management (GTT/PPGTT); load GuC/HuC/DMC firmware
- [ ] 2D/3D acceleration path for the compositor and toolkit

## v0.11: Wireless and peripherals

- [ ] RTL8852BT Wi-Fi: firmware load, scan, WPA2/WPA3 connection
- [ ] Bluetooth (HID devices and audio)
- [ ] UVC webcam

## v1.0: Daily driver

- [ ] Dual boot from the NVMe after a full backup ([ADR-0021](decisions/0021-disk-layout-phases.md), with explicit owner approval)
- [ ] Package manager and update mechanism
- [ ] Ladybird web browser ported ([ADR-0018](decisions/0018-ladybird-browser.md))
- [ ] Text editor/IDE, file manager, image viewer, music player
- [ ] One full working week on Horus without booting Arch
