# ADR-0020: Debug on a single machine

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** Claude, at the owner's request

## Context

The owner has only this laptop: no second computer to receive network logs or act as a USB debug host. The laptop has no serial port. The IOMMU is active with these groups: Wi-Fi (`02:00.0`) alone in group 11; xHCI (`00:14.0`) with the shared SRAM (`00:14.2`) in group 5; both LPSS I2C controllers in group 6; NVMe alone in group 10; audio in group 9 with the LPC bridge, SMBus, and SPI flash controller; the GPU alone in group 0.

## Decision

Debug with methods that need nothing but this laptop:

1. **QEMU + GDB** for everything QEMU emulates.
2. **Real devices inside QEMU:** USB passthrough (`usb-host`) for the Anker RTL8153, camera, Bluetooth, and spare USB sticks; **VFIO PCI passthrough** for the Wi-Fi card, and optionally the xHCI controller and the I2C controllers. This lets real-hardware drivers be developed under GDB.
3. **On bare metal:** framebuffer log console, boot stage colors, and the panic screen.
4. **Persistent logs:** every boot writes its log (not only panics) to the USB stick's ESP; `cargo xtask logs` reads them back from Arch after rebooting.
5. **In-kernel debug monitor** on a keyboard chord.

**Not used:** netconsole and xHCI Debug Capability, since both need a second computer.

## Consequences

- Most driver work, including the hard Wi-Fi driver, can happen with a debugger attached.
- **Never passed through:** the NVMe (it holds the running Arch system), the GPU (the host would lose its display), and the audio controller (its group includes the LPC bridge the host needs). Audio, GPU, ACPI/power, and suspend are debugged on bare metal with logs.
- Passing the Wi-Fi card drops Arch's Wi-Fi for the session; the Anker adapter can provide host networking unless it is itself passed to the guest.
- VFIO needs root to rebind devices. `xtask` wraps it and always asks for confirmation.
- Devices behind ACPI and GPIO (touchpad, touchscreen) only half-work in a VM because the guest's ACPI doesn't describe them; final testing is on bare metal.

## Alternatives considered

- Netconsole or DbC: need a second computer.
- Bare-metal-only debugging: slow photo-of-the-screen loop.
