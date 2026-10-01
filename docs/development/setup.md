# Development setup

The development host is the laptop itself, running Arch Linux
(Omarchy). Horus is built and tested in QEMU/KVM on the same machine,
then booted from a USB stick on the same machine.

## Host packages

Besides `rustup`, `git`, `gdb`, and `limine`, install (all present on
the laptop since 2026-10-01):

```sh
sudo pacman -S --needed qemu-desktop edk2-ovmf mtools libisoburn gptfdisk dosfstools acpica cargo-deny
```

| Package       | Why                                                      |
| ------------- | -------------------------------------------------------- |
| `qemu-desktop`| `qemu-system-x86_64` with KVM, USB, NVMe, HDA emulation  |
| `edk2-ovmf`   | UEFI firmware for QEMU (`/usr/share/edk2/x64/OVMF_CODE.4m.fd`) |
| `mtools`      | Build FAT32 images without root (`mformat`, `mcopy`)     |
| `libisoburn`  | `xorriso`, for optional ISO images                       |
| `gptfdisk`    | `sgdisk`, for GPT layouts                                |
| `dosfstools`  | `mkfs.fat`                                               |
| `acpica`      | `acpidump`/`iasl`, to dump and decompile this laptop's ACPI tables |
| `cargo-deny`  | Checks dependency licenses against the [policy](workflow.md#dependency-licenses) |

USB flashing also uses `lsblk` from `util-linux` (part of the base Arch
install) to inspect the target disk and all its descendants.

Limine binaries come from the installed `limine` package
(`/usr/share/limine/BOOTX64.EFI`; override with `LIMINE_DIR`). CI
downloads the same Limine release, pinned by version and SHA-256 in
`.github/workflows/ci.yml`. Bump both together. The kernel requests
Limine base revision 6, which Limine 12.8 supports.

`OVMF_CODE` and `OVMF_VARS` override the UEFI firmware paths; by
default `xtask` looks in the Arch and Debian/Ubuntu locations.

## Rust toolchain

`rust-toolchain.toml` at the repository root pins the toolchain
(currently `nightly-2026-09-30`) with `rust-src`, `rustfmt`, `clippy`,
`llvm-tools`, and the `x86_64-unknown-none` target. `rustup` picks it up
automatically; if it doesn't install it, run `rustup toolchain install`
in the repository. Bump the date in its own PR.

Nightly is needed by the `limine` crate today, and later for
`abi_x86_interrupt` and `-Zbuild-std` (userspace target). Each nightly
feature the kernel enables is listed in `kernel/src/main.rs` with a
comment explaining why.

A plain `cargo build` at the root builds only the host tools
(`default-members`); the kernel is always built through `cargo xtask`,
which passes `--target x86_64-unknown-none`.

## Repository layout

Items marked ✓ exist today; the rest are planned.

```
horus/
├── AGENTS.md               ✓ conventions for Codex and Claude (CLAUDE.md imports it)
├── CLAUDE.md               ✓ contains `@AGENTS.md`
├── LICENSE-MIT             ✓
├── LICENSE-APACHE          ✓
├── README.md               ✓
├── Cargo.toml              ✓ workspace; `license = "MIT OR Apache-2.0"` inherited by all crates
├── deny.toml               ✓ cargo-deny license policy
├── rust-toolchain.toml     ✓
├── .cargo/config.toml      ✓ `xtask` alias, kernel rustflags
├── .github/workflows/ci.yml ✓
├── boot/
│   └── limine.conf         ✓
├── kernel/                 ✓ horus-kernel
│   ├── build.rs            ✓ passes the linker script
│   ├── linker.ld           ✓
│   └── src/                ✓ arch, boot, framebuffer, log, panic, stage
│                             (later: mm, sched, ipc, syscall, acpi, pci, drivers, fs, block, ...)
├── libs/
│   ├── abi/                  horus-abi
│   ├── rt/                   horus-rt
│   ├── fs-format/            horus-fs-format
│   └── ui/                   horus-ui
├── services/                 init, compositor, input, audio, net, power
├── drivers/                  userspace drivers (horus-drv-*)
├── apps/                     shell, terminal, launcher, settings, ...
├── tools/                    host tools: mkfs, fsck, image builder
├── firmware/manifest.toml    firmware list with hashes and licenses (blobs are not committed)
├── xtask/                  ✓ build orchestration (host binary)
└── docs/                   ✓ you are here
```

## `cargo xtask` commands

Status: **✓** works today, otherwise planned. `--release` builds an
optimized kernel for `build`, `image`, `run`, and `test`.

| Command                       | Does                                                       | Status |
| ----------------------------- | ---------------------------------------------------------- | ------ |
| `cargo xtask build`           | Builds the kernel (userspace later)                        | ✓ |
| `cargo xtask image`           | Builds `target/horus.img` (GPT + FAT32 ESP + Limine + kernel; initrd later) | ✓ |
| `cargo xtask run`             | Boots the image in QEMU with KVM, OVMF, 8 CPUs, 4 GiB RAM, 1360×768 (see below), debugcon to stdout | ✓ |
| `cargo xtask run --gdb`       | Same, paused, with a GDB stub on `:1234`                   | ✓ |
| `cargo xtask run --no-kvm`    | TCG emulation (slower, more deterministic)                 | ✓ |
| `cargo xtask run --headless`  | No QEMU window                                             | ✓ |
| `cargo xtask run --usb VID:PID` | Passes a real USB device into QEMU ([debugging.md](debugging.md#2-real-devices-inside-qemu)) | Planned |
| `cargo xtask run --vfio BDF`  | Passes a real PCI device into QEMU with VFIO; refuses the NVMe, GPU, and audio groups | Planned |
| `cargo xtask logs`            | Copies boot logs and crash reports from the USB stick and prints the latest | Planned |
| `cargo xtask firmware`        | Fills the firmware cache from `/usr/lib/firmware` using `firmware/manifest.toml` ([ADR-0022](../decisions/0022-firmware-blobs.md)) | Planned |
| `cargo xtask test`            | Boots headless (KVM if available, else TCG) three times and checks the kernel log and the **text on screen**, read back from QEMU screenshots with the kernel's own font: a normal boot (last console line must read `horus: boot complete`), a scroll test (`console-test` kernel feature: 100 numbered lines, every visible row checked), and a forced panic (`panic-test`: every panic screen line checked against the debug log). Screenshots land in `target/test-*.ppm`. Kernel unit tests come in v0.2 | ✓ |
| `cargo xtask flash /dev/sdX`  | Writes the existing `target/horus.img` to a USB stick, after the safety checks below | ✓ |
| `cargo xtask ci`              | Everything CI runs: fmt, clippy (kernel and xtask, `-D warnings`), xtask host unit tests, `cargo deny`, boot test | ✓ |

### QEMU baseline

`xtask run` does the equivalent of the commands below (see
`xtask/src/qemu.rs`). The UEFI variable store must be a writable copy;
`xtask` makes one per QEMU (`target/ovmf-vars-<pid>.fd`) and deletes it
when QEMU exits. To run QEMU by hand:

```sh
cargo xtask image    # builds target/horus.img
cp /usr/share/edk2/x64/OVMF_VARS.4m.fd target/ovmf-vars-manual.fd
qemu-system-x86_64 \
  -machine q35 -smp 8 -m 4G -no-reboot -enable-kvm -cpu host \
  -drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd \
  -drive if=pflash,format=raw,file=target/ovmf-vars-manual.fd \
  -drive if=none,id=stick,format=raw,file=target/horus.img \
  -device qemu-xhci,id=xhci -device usb-storage,bus=xhci.0,drive=stick,bootindex=0 \
  -vga none -device VGA,xres=1360,yres=768 \
  -debugcon stdio
```

This matches the laptop's boot path: UEFI, xHCI, and the OS on USB
storage. The display is 1360×768, not the panel's 1366×768: QEMU's
standard VGA only shows widths that are a multiple of 8. Asked for 1366,
it displays 1360 while the firmware still reports 1366 to the kernel,
so every row lands at the wrong offset and text shears diagonally.
`xtask` refuses such widths. Devices are added as their drivers arrive: `-device usb-net`
(CDC-ECM, standing in for the Anker adapter) in v0.8, and
`-device intel-hda -device hda-duplex` in v0.9. Add `-device nvme` with a scratch image to test the NVMe
driver. QEMU can't emulate the hybrid P/E cores, so scheduler placement
must also be tested on the laptop.

### `flash` safety checks

`cargo xtask flash /dev/sdX`:

1. Resolves aliases and refuses any target that isn't a whole block
   device (`/dev/sdX`, confirmed as `TYPE=disk` by `lsblk`).
2. Refuses NVMe paths (`/dev/nvme*`, including resolved aliases), any
   mounted disk or descendant, swap, and active device holders.
3. Refuses devices whose transport isn't exactly `usb` (`lsblk TRAN`);
   failed or malformed `lsblk` output also stops the command.
4. Checks that the existing image is nonempty, sector aligned, and fits
   the device. The current GPT image requires 512-byte logical sectors.
5. Opens the disk exclusively before displaying the confirmation prompt
   and holds that same handle through the write and flush. Unplugging
   and replacing the stick cannot redirect writes to the replacement.
   Before prompting, prepares and verifies a private sparse copy of
   the image for the disk's full size. `sgdisk` relocates the backup GPT
   in that regular file; it never receives a real block device path.
   Staging uses Linux `O_TMPFILE` with mode `0600` under `/tmp` and gives
   `sgdisk` a `/proc/<pid>/fd/<fd>` reference to the held file. There is
   no writable directory entry for another process to replace.
6. Shows model, size in bytes, and current partitions, then requires
   typing the full resolved device name (for example `/dev/sdb`). A
   mismatch or EOF cancels before any write.
7. Checks the opened disk's identity and repeats the
   device checks after confirmation. Changed details cancel the write.
8. Copies the image and writes its relocated backup GPT at the disk's
   actual end, through the same held handle, then flushes before
   reporting success. All existing data on the target is disposable;
   an interrupted or failed write requires flashing it again.

Staging requires up to the image's size in free `/tmp` space (currently
514 MiB); an allocation failure stops the command before confirmation
or disk writes and reports this requirement. The filesystem containing
`/tmp` must support `O_TMPFILE` (Linux tmpfs and ext4 support it).
The kernel releases the anonymous file when its last descriptor closes,
including if the process is killed. It leaves no scratch directory or
name that can collide with a later run. Only the image and backup GPT
regions are written; the unused middle of a larger disk is not erased.
Flashing is not a secure wipe.

Build the image first with `cargo xtask image`. `flash` accepts exactly
one device argument and no options or confirmation bypass. It requires
Linux and write permission on the USB disk. If elevated permission is
needed, run the already-built host tool with
`sudo target/debug/xtask flash /dev/sdX` from the repository root;
this runs the same checks and prompt without building as root.

## Booting on the laptop

1. `cargo xtask image && cargo xtask flash /dev/sdX`
2. Reboot, press **F9** at the HP logo, and pick the USB stick.
3. Limine shows the `Horus` entry and boots it after 3 seconds.
4. To return to Arch, reboot and let the firmware boot the NVMe as usual.

If the USB stick isn't listed in the F9 menu, check in BIOS setup (F10)
that USB boot is enabled. Secure Boot is already off.
