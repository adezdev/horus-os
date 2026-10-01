# Target hardware

Horus targets one machine first. This inventory was collected on
2026-09-30 from the running Arch Linux install (`lspci`, `/sys`,
`/proc`). Linux driver names are listed only to identify each device.
Linux driver code is GPL and must never be copied into Horus; see
[ADR-0017](decisions/0017-dual-mit-apache-license.md) for how it may
be used as a reference.

## System

| Item         | Value                                                     |
| ------------ | --------------------------------------------------------- |
| Model        | HP Laptop 15-fd0xxx (board `8DD7`)                        |
| Firmware     | AMI UEFI 2.80, BIOS F.26 (2026-04-22), 64-bit UEFI        |
| Secure Boot  | Disabled                                                  |
| Current boot | Limine 12.8.0 → Arch Linux (systemd-stub UKI)             |
| RAM          | 8 GB (7.4 GiB visible to the OS)                          |
| Display      | Internal eDP panel, 1366×768                              |
| Sleep states | `s2idle` (current) and `deep` (S3) both offered           |
| TPM          | None visible to the OS (Intel PTT may be disabled in BIOS) |
| Boot menu    | F9 at power-on (Esc for the startup menu, F10 for setup)  |

## CPU: Intel Core i3-1315U (Raptor Lake-U)

- **Hybrid:** 2 P-cores (Raptor Cove, 2 threads each) + 4 E-cores
  (Gracemont), 8 hardware threads.
- Core type is reported by `CPUID.1Ah`; Thread Director data comes from
  the **Hardware Feedback Interface** (`hfi`).
- No AVX-512 and no 5-level paging (`la57`), so Horus uses **4-level
  paging**.

Features Horus relies on or plans to use:

| Feature                             | Used for                                            |
| ----------------------------------- | --------------------------------------------------- |
| `x2apic`                            | Interrupt controller (MSR-based APIC)               |
| `tsc_deadline_timer`, `arat`        | Per-CPU one-shot timer that keeps running in C-states |
| `constant_tsc`, `nonstop_tsc`       | Invariant TSC as the main clock source              |
| `tsc_known_freq`                    | TSC frequency from `CPUID.15h`, no calibration needed |
| `pcid`                              | Cheap address-space switches (fewer TLB flushes)    |
| `pdpe1gb`                           | 1 GiB pages for the direct physical map             |
| `smep`, `smap`, `umip`              | Kernel hardening                                    |
| `pku`                               | Protection keys (userspace memory domains, later)   |
| `ibt`, `user_shstk`                 | CET control-flow protection (later)                 |
| `fsgsbase`                          | Fast per-CPU / TLS base switching                   |
| `xsave`                             | FPU/SSE/AVX state save and restore                  |
| `avx2`                              | Software compositor, memcpy, checksums              |
| `aes`, `vaes`, `vpclmulqdq`         | Full-disk encryption (AES-XTS) at high speed        |
| `sha_ni`                            | Fast SHA-256 hashing                                |
| `rdrand`, `rdseed`                  | Kernel entropy                                      |
| `hwp`, `hwp_notify`                 | Intel Speed Shift (hardware P-state control)        |
| `hfi`                               | Thread Director hints for the scheduler             |
| `monitor`, `waitpkg`                | `MWAIT` idle and `UMWAIT`/`TPAUSE`                  |

## Devices and driver plan

Difficulty: ★ easy, ★★ moderate, ★★★ hard, ★★★★ very hard.
Placement: **K** = in the kernel, **U** = userspace driver
(see [drivers.md](architecture/drivers.md)).

| Device                           | IDs                    | Bus      | Linux driver   | Difficulty | Place | Milestone |
| -------------------------------- | ---------------------- | -------- | -------------- | ---------- | ----- | --------- |
| GOP / Limine framebuffer         | —                      | firmware | —              | ★          | K     | v0.1      |
| PS/2 keyboard (EC, i8042)        | AT Translated Set 2    | ISA/LPC  | `atkbd`        | ★          | K → U | v0.1      |
| Local APIC / IOAPIC / HPET       | —                      | —        | —              | ★★         | K     | v0.2      |
| PCIe ECAM (`MCFG`)               | —                      | PCIe     | —              | ★          | K     | v0.4      |
| USB 3.2 xHCI controller          | `8086:51ed`            | PCIe     | `xhci_hcd`     | ★★★        | K     | v0.4      |
| USB mass storage (the boot stick) | class 08h             | USB      | `usb-storage`  | ★★         | K     | v0.4      |
| Samsung PM9C1a NVMe SSD, 256 GB  | `144d:a80d`            | PCIe     | `nvme`         | ★★         | K     | v0.4 (read-only) |
| ACPI tables + AML interpreter    | DSDT + 13 SSDTs        | —        | ACPICA         | ★★★        | K     | v0.6      |
| Intel GPIO (pinctrl)             | ACPI `INTC1055`        | MMIO     | `pinctrl-alderlake` | ★★    | U     | v0.6      |
| LPSS I2C controllers (DesignWare) | `8086:51e8`, `51e9`   | PCIe     | `intel-lpss`   | ★★         | U     | v0.6      |
| Synaptics touchpad (I2C-HID)     | ACPI `SYNA32DC`, `06CB:CEE7` | I2C | `i2c-hid`  | ★★         | U     | v0.6      |
| Touchscreen (I2C-HID)            | ACPI `GTCH7503`, `2A94:D009` | I2C | `i2c-hid`  | ★★         | U     | v0.6      |
| Lid, power button, AC, battery   | `PNP0C0D`, `PNP0C0C`, `ACPI0003`, `PNP0C0A` | ACPI/EC | `acpi` | ★★ | K | v0.6 |
| HP hotkeys (WMI), Intel HID events | `PNP0C14` (WMI) + Intel HID ACPI device | ACPI | `hp-wmi`, `intel-hid` | ★★ | U | v0.9 |
| Anker USB-C Ethernet (RTL8153)   | USB `0bda:8153`        | USB      | `r8152`        | ★★         | U     | v0.8      |
| Backlight (eDP PWM)              | in GPU display engine  | MMIO     | `i915`         | ★★         | K     | v0.9      |
| HDA controller + Realtek ALC236 codec | `8086:51ca`, codec `10ec:0236` | PCIe | `snd-hda-intel` | ★★★ | U | v0.9 |
| Intel SOF audio DSP (digital mic) | `8086:51ca`, `NHLT`   | PCIe     | `sof-audio-pci-intel-tgl` | ★★★★ | U | v0.9+ |
| HDMI audio                       | codec `8086:281f`      | HDA      | `snd-hda-codec-hdmi` | ★★   | U     | v0.9+     |
| Intel UHD Graphics (Xe-LP)       | `8086:a7a9`            | PCIe     | `i915` / `xe`  | ★★★★       | K     | v0.10     |
| HDMI output                      | via GPU                | —        | `i915`         | ★★★        | K     | v0.10     |
| Realtek RTL8852BE-VT (8852BT) Wi-Fi 6 | `10ec:b520`       | PCIe     | `rtw89_8852bte` | ★★★★      | U     | v0.11     |
| Realtek Bluetooth (8852BT combo) | USB `0bda:b86a`        | USB      | `btrtl`/`btusb` | ★★★       | U     | v0.11     |
| HP True Vision HD camera         | USB `30c9:00c7` (UVC)  | USB      | `uvcvideo`     | ★★         | U     | v0.11     |
| VT-d IOMMU                       | ACPI `DMAR` (2 units)  | MMIO     | `intel-iommu`  | ★★★        | K     | v0.5      |
| Thermal (DPTF), MEI, GNA, SMBus  | `a71d`, `51e0`, `a74f`, `51a3` | PCIe | various | —     | —     | Not planned |

### Notes per device

- **Keyboard:** the keyboard reaches the OS as a PS/2 device through the
  embedded controller's i8042 emulation. That makes it the easiest input
  device, and the reason it is the v0.1 input.
- **Booting from USB means the kernel needs USB.** After Limine hands
  over, firmware services are gone. Until the xHCI and mass-storage
  drivers exist, everything Horus needs is loaded into RAM by Limine as
  a module (initial ramdisk).
- **NVMe safety.** The NVMe holds the owner's LUKS-encrypted btrfs Arch
  install and the 2 GB ESP. The NVMe driver is **read-only** until a
  dedicated Horus partition exists and the owner approves. Code must
  refuse to write to it otherwise.
- **Touchpad and touchscreen** sit on Intel LPSS I2C controllers.
  They are enumerated through ACPI (`_HID`, `_CRS`) and signal through
  GPIO interrupts, so they need the **AML interpreter**, the **GPIO
  driver**, and the **I2C driver** first. The HID-over-I2C protocol
  itself is simple and publicly specified.
- **Audio:** the ALC236 codec sits behind the SOF DSP. Linux picks
  SOF because the `NHLT` table describes **2 digital microphones**,
  which are wired to the DSP. Plan: use the HDA controller in *legacy
  HDA mode* first (speakers and headphones; the internal mic won't work
  in this mode), then the SOF DSP path (`intel/sof/sof-rpl.ri`) for the
  microphones.
- **Sleep:** firmware offers S3 (`deep`) as well as `s2idle`; the
  kernel log confirms `ACPI: PM: (supports S0 S3 S4 S5)`. S3 is
  usually easier for a new OS: firmware powers devices down and resumes
  through the FACS waking vector. `s2idle` / S0ix needs every device to
  reach its low-power state. Horus targets **S3 first**.
- **GPU:** Intel publishes open Programmer's Reference Manuals for Xe-LP
  (Tiger Lake / Alder Lake volumes). Display version 13 (Alder Lake-P
  family) covers this chip. The display microcontroller firmware (DMC)
  is needed for deep display power states.
- **Wi-Fi** is the RTL8852BE-VT, which Linux drives as the **8852BT**
  variant (`rtw89_8852bte`, firmware `rtw89/rtw8852bt_fw.bin`). It has
  no public datasheet and needs Realtek firmware. This is the last major
  driver on the roadmap. The Anker RTL8153 adapter covers networking
  until then. The card is alone in its IOMMU group, so it can be passed
  into QEMU for development ([ADR-0020](decisions/0020-single-machine-debugging.md)).
- **Firmware** for every device that needs it is listed in
  [ADR-0022](decisions/0022-firmware-blobs.md).
- **Ethernet:** the adapter is a Realtek RTL8153. It exposes a standard
  **CDC-ECM** USB configuration besides its vendor mode, so Horus can
  start with a generic class driver. QEMU's `usb-net` device emulates
  CDC-ECM, so the driver can be developed in the emulator.
- **Camera sensors.** ACPI also lists `OVTI*` and `INT347*` devices.
  The working camera is the USB UVC one; these entries can be ignored.

## Verification checklist

Facts that can only be settled by experiment. Each one is quick to
test from Arch. Record results here with the date.

| # | Question | How to test | Result |
| - | -------- | ----------- | ------ |
| V1 | Do speakers and headphones work in legacy HDA mode (without SOF)? | At the Limine menu, press `e` on the Arch entry, append `snd_intel_dspcfg.dsp_driver=1` to the kernel command line, and boot. Play audio on speakers, then headphones. The internal mic is expected to be missing. The change lasts for that boot only. | Not yet tested |
| V2 | Does S3 suspend and resume reliably? | `echo deep \| sudo tee /sys/power/mem_sleep`, then `systemctl suspend`; wake with the power button. Check display, keyboard, touchpad, Wi-Fi, and USB. Repeat 5 times. Reverts on reboot. | Firmware advertises S3 (kernel log); reliability not yet tested |
| V3 | Is a firmware TPM (Intel PTT) available? | BIOS setup (F10) → Security: look for TPM / PTT. Not needed before 1.0 ([ADR-0019](decisions/0019-security-hardening-set.md)). | Currently off or absent: no `TPM2` ACPI table, no `/dev/tpm*` |
| V4 | Does RAM survive a warm reboot (for early crash logs)? | Needs Horus v0.1: write a marker to a reserved RAM region, reboot, check it on the next boot. | Not yet tested |
| V5 | Does the laptop boot from USB with the F9 menu? | Make any bootable USB stick (e.g. the Arch ISO), press F9 at power-on. | Not yet tested |

## Re-collecting this data

```sh
lspci -nn; lspci -k
cat /sys/class/dmi/id/{sys_vendor,product_name,board_name,bios_version}
grep -m1 flags /proc/cpuinfo
for d in /sys/bus/usb/devices/*; do [ -e $d/idVendor ] && echo "$(cat $d/idVendor):$(cat $d/idProduct) $(cat $d/product 2>/dev/null)"; done
ls /sys/bus/acpi/devices/ /sys/firmware/acpi/tables/
cat /proc/asound/card*/codec#* | grep -E '^Codec|^Vendor Id'
```

To dump the ACPI tables for offline study (needed for the AML work):

```sh
sudo pacman -S acpica
sudo acpidump -b   # writes *.dat files into the current directory
iasl -d dsdt.dat   # decompile to dsdt.dsl
```
