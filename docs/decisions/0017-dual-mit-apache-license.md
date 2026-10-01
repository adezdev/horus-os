# ADR-0017: License under MIT OR Apache-2.0

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** owner

## Context

The repository will be public on GitHub. Without a license, the code
would be "all rights reserved": readable, but nobody could legally use,
modify, or contribute to it.

## Decision

License Horus under **MIT OR Apache-2.0**, at the user's option, the
standard dual license of the Rust ecosystem.

- `LICENSE-MIT` and `LICENSE-APACHE` live at the repository root.
- Every crate declares `license = "MIT OR Apache-2.0"` (set once in the
  workspace `Cargo.toml` and inherited).
- Source files carry an SPDX header:
  `// SPDX-License-Identifier: MIT OR Apache-2.0`.
- Contributions are accepted under the same terms (inbound = outbound).
  The repository `README.md` states:

  > Unless you explicitly state otherwise, any contribution
  > intentionally submitted for inclusion in Horus by you, as defined
  > in the Apache-2.0 license, shall be dual licensed as above, without
  > any additional terms or conditions.

## Consequences

- Anyone can reuse Horus code, including in closed-source projects.
  Apache-2.0 adds an explicit patent grant; MIT keeps compatibility with
  GPL-2.0-only projects.
- **GPL code can never be copied or translated into Horus.** This
  applies to the Linux drivers for this laptop's hardware: `rtw89`
  (Wi-Fi), `i915`/`xe` (GPU), `r8152` (Ethernet), `snd-hda`/SOF (audio),
  and others. Linux source may be read to learn hardware behavior
  (register meanings, initialization order, quirks), but drivers must
  be written from public specifications, datasheets, and observation
  (`usbmon`, `acpidump`, register dumps). The code must be original.
- Dependencies must be compatible with MIT OR Apache-2.0 (see the
  dependency policy in [workflow.md](../development/workflow.md#dependency-licenses)).
- Ported software that runs on Horus as a separate program (for
  example a browser or a POSIX shell) keeps its own license; it isn't
  linked into Horus components.
- Vendor firmware blobs have their own redistribution terms and are
  handled separately ([ADR-0022](0022-firmware-blobs.md)).

## Alternatives considered

- MPL-2.0: file-level copyleft; less common in the Rust OSDev ecosystem.
- GPL-2.0/3.0: would allow reusing Linux driver code, but everything
  built on Horus would have to be GPL as well.
- No license: public but unusable by others; blocks outside contributions.
