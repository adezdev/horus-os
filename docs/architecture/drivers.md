# Drivers

The per-device inventory, difficulty, and milestones are in
[hardware.md](../hardware.md). This document covers *how* drivers are
built.

## Two kinds of drivers

### Kernel drivers

Used for the hot path (NVMe, xHCI + mass storage, GPU display) and
for early boot. They:

- live in `horus-kernel/src/drivers/<name>/`,
- implement a small trait for their subsystem (`BlockDevice`,
  `DisplayEngine`, `UsbHostController`, ...),
- are probed from the PCIe/ACPI device tree by vendor/device ID or ACPI `_HID`,
- use MSI-X interrupts with per-CPU queues where the hardware allows
  (NVMe has one submission/completion queue pair per CPU).

### Userspace drivers

Used for everything else. Each one is a normal process
(`horus-drv-<name>`) started by the device manager in `horus-init` when
a matching device appears. The kernel gives it only the capabilities
for that device:

| Capability     | Gives                                                           |
| -------------- | --------------------------------------------------------------- |
| `Mmio`         | Mapping of one device's BAR or ACPI memory resource, uncached   |
| `Irq`          | A waitable handle that is signaled when the device interrupts; acknowledged by the driver |
| `DmaBuffer`    | Physically contiguous or scatter-gather memory, mapped in the **VT-d IOMMU** only for that device |
| `PortIo`       | A specific I/O port range (rare; e.g. the i8042 keyboard controller) |
| `I2cBus`, `GpioLine` | Handles from the I2C/GPIO drivers for downstream drivers  |
| `UsbInterface` | One claimed interface on a USB device (endpoints, transfers)    |

With VT-d enabled, a userspace driver **cannot DMA outside the buffers it
was given**, so a buggy or compromised driver can't corrupt the kernel.

The driver exposes its service to the rest of the system through a
channel (e.g. the touchpad driver feeds HID reports to the input
server).

### Restart policy

If a userspace driver crashes, the device manager resets the device
(PCIe Function Level Reset or ACPI `_RST` where available) and restarts
the driver with backoff. After three crashes in a minute, it gives up
and posts a notification.

## Device discovery

```
ACPI namespace (AML) ─┬─► platform devices (_HID/_CID): EC, battery,
                      │   lid, buttons, GPIO, I2C devices, WMI
                      └─► _PRT / _CRS routing info
PCIe ECAM (MCFG) ─────► PCI functions by vendor:device / class
USB (xHCI) ───────────► devices/interfaces by VID:PID / class
I2C (via ACPI) ───────► I2C-HID devices (SYNA32DC, GTCH7503)
```

All of these feed one **device tree** in the kernel. Userspace can
subscribe to device-added and device-removed events (hotplug for USB,
HDMI, AC adapter).

## Driver stacks on this laptop

### Input

```
i8042 (PS/2 kbd) ─────────────────────────────────────┐
LPSS I2C ─► I2C-HID ─► touchpad (SYNA32DC) ────────────┤
LPSS I2C ─► I2C-HID ─► touchscreen (GTCH7503) ─────────┼─► input server ─► compositor
USB HID (external keyboard/mouse) ─────────────────────┤
ACPI: power button, lid, HP WMI hotkeys ───────────────┘
GPIO (INTC1055) provides the interrupt lines for the I2C-HID devices.
```

The input server turns raw HID reports into one event stream: key
events with keymaps and repeat, pointer motion with acceleration, scroll,
touch points, and **gestures** (2/3/4-finger swipe, pinch).

### Storage

```
NVMe (read-only until approved) ─┐
xHCI ─► USB mass storage ────────┼─► block layer ─► encryption ─► Horus FS / FAT32 ─► VFS
initrd (RAM) ────────────────────┘
```

### Network

```
xHCI ─► CDC-ECM / CDC-NCM (Anker RTL8153) ─┐
PCIe ─► RTL8852BT Wi-Fi (later) ───────────┼─► network stack (userspace) ─► sockets API
```

### Audio

```
HDA controller (legacy mode) ─► ALC236 codec ─► speakers / headphones
SOF DSP (later) ─► DMIC (from NHLT)
HDMI codec ─► HDMI audio
          └──────────────► audio server (mixing, per-app volume, routing)
```

### Display

```
v0.1–v0.9: Limine/GOP framebuffer (fixed 1366×768 mode, no vblank)
v0.10+:    Intel Xe-LP display engine: eDP + HDMI, page flip, vblank, backlight PWM
```

## Writing a new driver: checklist

1. Add the device to [hardware.md](../hardware.md) if missing.
2. Decide kernel or userspace using the rule in
   [overview.md](overview.md#what-lives-where). If kernel, explain why in
   the PR.
3. Find the public spec (NVMe, xHCI, USB class specs, HID-over-I2C, HDA,
   Intel PRMs, ACPI). Record the document and version in the module docs.
4. Get it working in QEMU first if QEMU emulates it (`nvme`,
   `qemu-xhci`, `usb-storage`, `usb-net`, `intel-hda`, `hda-duplex`,
   `usb-kbd`, `usb-tablet`).
5. Add a smoke test to `cargo xtask test` where possible.
6. Test on the laptop from the USB stick.
7. Mark progress in [roadmap.md](../roadmap.md).

Reference sources: Linux drivers are GPL and may be read but never
copied or translated; see
[ADR-0017](../decisions/0017-dual-mit-apache-license.md).
