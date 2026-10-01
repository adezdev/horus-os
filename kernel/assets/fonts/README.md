# Kernel console font

| File                 | What                                                   |
| -------------------- | ------------------------------------------------------ |
| `spleen-8x16.psfu`   | Spleen 2.2.0, 8×16, PSF1 with Unicode table            |
| `LICENSE-spleen`     | Its license: BSD-2-Clause, © 2018-2026 Frederic Cambus |

Source: <https://github.com/fcambus/spleen/releases/tag/2.2.0>
(`spleen-2.2.0.tar.gz`, SHA-256
`ec42925c6b56d2138c862b2f97147c872e472f674bf03423417d827a08d69a89`).

The font is compiled into the kernel (`kernel/src/font.rs`), so
`cargo xtask image` copies `LICENSE-spleen` onto the boot partition
under `/boot/horus/licenses/`, as the license requires for binary
redistribution.
