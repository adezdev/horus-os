# Horus

A from-scratch 64-bit operating system written in Rust, built for one
laptop first and aiming to become a fast daily driver with a modern
tiling + floating desktop.

> **Status:** design phase. No code yet; the first milestone (v0.1) is
> booting on the target laptop from a USB stick.

## Highlights

- **Hybrid kernel:** storage and display on the fast path in the kernel;
  input, USB class, audio, network, and Wi-Fi drivers isolated in userspace
- **Hybrid-aware scheduler** for Intel P-cores and E-cores
- **Capability-based native API** with a POSIX compatibility layer
- **Copy-on-write, checksummed filesystem** with full-disk encryption
- **Themeable desktop** with keyboard-driven tiling, floating windows,
  touchpad gestures, and touchscreen support
- Boots with **Limine** on UEFI

## Target hardware

HP Laptop 15-fd0xxx: Intel Core i3-1315U, 8 GB RAM, Intel UHD Graphics,
Samsung NVMe, Realtek RTL8852BT Wi-Fi. Full inventory and driver plan in
[docs/hardware.md](docs/hardware.md).

## Documentation

Start at [docs/README.md](docs/README.md): vision, architecture,
roadmap, development setup, and the decision records behind each choice.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in Horus by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.
