# ADR-0022: Firmware blobs outside the repository

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** Claude, at the owner's request

## Context

Several devices need vendor firmware. The files Linux loads on this laptop (from its kernel log) are: `rtw89/rtw8852bt_fw.bin` (Wi-Fi), `i915/adlp_dmc.bin`, `i915/adlp_guc_70.bin`, `i915/tgl_huc.bin` (GPU), `rtl_bt/rtl8852btu_fw.bin` + `rtl8852btu_config.bin` (Bluetooth), and `intel/sof/sof-rpl.ri` + `intel/sof-tplg/sof-hda-generic-2ch.tplg` (audio DSP). All are already installed on the host under `/usr/lib/firmware` (zstd-compressed). The RTL8153 Ethernet adapter needs none in CDC-ECM mode.

## Decision

**Do not commit blobs.** The repository holds `firmware/manifest.toml` listing each file, its SHA-256 (of the decompressed file), its upstream source, and its license. `cargo xtask firmware`:

1. copies each file from `/usr/lib/firmware`, decompresses `.zst`, and checks the hash;
2. falls back to downloading from `linux-firmware` at a pinned tag, also hash-checked;
3. stores the result in a git-ignored cache that `xtask image` puts in `/system/firmware` along with each blob's license file.

## Consequences

- The repository stays purely MIT OR Apache-2.0.
- Builds are reproducible: a hash mismatch fails loudly.
- Published images include the blobs alongside their license terms (linux-firmware licenses allow redistribution).

## Alternatives considered

- Commit blobs to the repo: mixes licenses and bloats history.
- Read firmware from the Arch partition at runtime: Horus can't read LUKS/btrfs, and it shouldn't depend on Arch.
