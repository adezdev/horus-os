# Debugging

Horus is developed on **one machine**: the laptop it targets. There is
no second computer and no serial port, so every method here works with
the laptop alone ([ADR-0020](../decisions/0020-single-machine-debugging.md)).

| Method                                  | Where        | Best for                                 |
| --------------------------------------- | ------------ | ---------------------------------------- |
| 1. QEMU + GDB                           | Emulator     | Kernel core, emulated devices            |
| 2. Real devices passed into QEMU        | Emulator + real hardware | USB devices, Wi-Fi, xHCI, I2C |
| 3. Framebuffer log, stage colors, panic screen | Bare metal | Boot hangs, crashes                    |
| 4. Persistent logs on the USB stick     | Bare metal → Arch | Anything after storage works        |
| 5. In-kernel debug monitor              | Bare metal   | Inspecting a live system without userspace |
| 6. Tests, fuzzing, Linux as an oracle   | Host         | Logic bugs, parsers, learning hardware   |

## 1. QEMU + GDB

```sh
cargo xtask run --gdb          # QEMU paused, GDB stub on localhost:1234
gdb target/x86_64-unknown-none/debug/horus-kernel \
    -ex 'target remote :1234' \
    -ex 'hbreak kernel_main' -ex 'continue'
```

- Use **hardware breakpoints** (`hbreak`) until the kernel has switched
  to its own page tables. Software breakpoints on code that gets
  remapped are unreliable.
- With KVM, single-stepping through interrupt handlers can be flaky;
  `--no-kvm` (TCG) is slower but deterministic.
- `-d int,cpu_reset -D qemu.log` (TCG only) logs every exception and
  triple-fault reset. Use it when the machine reboots instantly.
- QEMU monitor (`Ctrl-Alt-2` in the QEMU window, or `-monitor stdio`):
  `info registers`, `info mem`, `info tlb`, `info pic`, `x/16gx <addr>`.
- `xtask` writes a `.gdbinit` with helpers to print the current thread,
  run queues, and the handle table.

## 2. Real devices inside QEMU

This brings real-hardware driver work under the debugger, on one
machine. Horus runs in QEMU while Linux hands a physical device to it.

### USB passthrough (easy)

```sh
cargo xtask run --usb 0bda:8153     # Anker RTL8153 Ethernet
cargo xtask run --usb 30c9:00c7     # HP webcam (UVC)
cargo xtask run --usb 0bda:b86a     # Realtek Bluetooth
```

This becomes QEMU's `-device usb-host,vendorid=0x...,productid=0x...`.
The device disappears from Arch while QEMU runs. Access needs a udev
rule granting the `wheel` group access to these IDs; `xtask` prints the
rule the first time.

### PCI passthrough with VFIO (advanced)

The IOMMU (VT-d) is active on this laptop. A device can be passed to
QEMU only together with its whole IOMMU group:

| Device                    | Group | Pass through? | Effect on Arch while QEMU runs |
| ------------------------- | ----- | ------------- | ------------------------------ |
| Wi-Fi RTL8852BT `02:00.0` | 11 (alone) | **Yes**: main use case | Arch loses Wi-Fi; use the Anker adapter for host networking |
| xHCI `00:14.0` + shared SRAM `00:14.2` | 5 | Yes, carefully | Arch loses all USB (keyboard and touchpad keep working: they aren't USB) |
| LPSS I2C `00:15.0`, `00:15.1` | 6 | Possible, limited | Arch loses touchpad and touchscreen. The guest's ACPI doesn't describe the devices, so the driver must be configured by hand (I2C address, polling instead of GPIO interrupts) |
| NVMe `01:00.0`            | 10 | **Never** | It holds the running Arch system |
| GPU `00:02.0`             | 0 | **No**    | Arch would lose its display |
| Audio `00:1f.3`           | 9 | **No**    | Shares a group with the LPC bridge, SMBus, and SPI flash controller |

```sh
cargo xtask run --vfio 02:00.0      # asks for confirmation and sudo, binds to vfio-pci,
                                    # runs QEMU, then rebinds the Linux driver on exit
```

`xtask` refuses group 10 (NVMe), group 0 (GPU), and group 9 (audio).

## 3. Framebuffer log, stage colors, panic screen

- All kernel logs go to an in-memory ring buffer and to the framebuffer console.
- During boot, solid **stage colors** show how far boot got before a
  hang ([boot.md](../architecture/boot.md#early-failure-visibility)).
- The panic screen shows message, location, CPU, registers, and a
  symbolized backtrace, laid out to be readable in a phone photo.

## 4. Persistent logs on the USB stick

The main bare-metal feedback loop on a single machine:

1. Boot Horus from the stick and reproduce the problem.
2. Reboot into Arch.
3. `cargo xtask logs` mounts the stick's ESP (via `udisksctl`, no
   sudo), copies `/boot/horus/logs/`, and prints the latest boot log
   and any crash report.

Details:

- **Every boot** writes its full log to `/boot/horus/logs/<boot-id>.log`
  once USB storage works (v0.4), not only on a panic. The last 20 boots
  are kept.
- On panic, the last 64 KiB of the log and the panic report are
  written to `/boot/horus/crash/` through a minimal, polling-only USB
  storage path that works with interrupts off.
- Before USB storage works (v0.1 to v0.3), logs exist only on screen,
  plus a RAM region that Horus checks on the next **warm** reboot.
  Whether this laptop's firmware preserves RAM across a warm reboot
  is on the [verification checklist](../hardware.md#verification-checklist).

## 5. In-kernel debug monitor

A keyboard chord (e.g. `Ctrl+Alt+SysRq`-style) opens a kernel monitor
on the framebuffer that works without userspace: list threads and run
queues, dump memory, show the device tree and IRQ counts, write the log
to the stick now, force a crash dump.

## 6. Tests, fuzzing, and Linux as an oracle

- **Boot tests** (`cargo xtask test`, today): boot headless, check the
  kernel log, and take a QEMU screenshot over QMP to check what the
  screen shows (stage stripe, scrolled console, panic screen). The
  screenshots stay in `target/test-*.ppm` for a look when a test fails.
- **Kernel test harness** (v0.2): `#[test_case]` functions run in QEMU
  and report through `debugcon`; QEMU exits with the result code via
  `isa-debug-exit`.
- **Host-side unit tests** for pure logic (allocators, FS format,
  parsers, scheduler policy) run with plain `cargo test`, which is much
  faster than QEMU.
- **Fuzzing** with `cargo fuzz` for every parser of untrusted data.
- **Linux as an oracle**, on the same laptop: `acpidump` + `iasl` for
  ACPI, `lspci -vvv` for BARs and MSI, `evtest` and `hid-recorder` for
  input, `usbmon` + Wireshark for USB traffic, the kernel log for
  firmware and init order.

## Triage guide

| Symptom                           | First thing to try                                    |
| --------------------------------- | ----------------------------------------------------- |
| Instant reboot in QEMU            | Triple fault: `--no-kvm -d int,cpu_reset`; check IDT/TSS/IST stacks |
| Black screen in QEMU              | Check `debugcon` output; then the Limine base revision |
| Works in QEMU, hangs on laptop    | Note the stage color; compare ACPI/APIC assumptions; run `cargo xtask logs` in Arch |
| Random corruption under SMP       | Run with `-smp 1`; check TLB shootdowns and per-CPU data |
| Device silent on hardware         | Pass it into QEMU and debug under GDB; compare with Linux (`lspci -vvv`, `usbmon`) |
