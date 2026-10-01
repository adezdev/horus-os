# Open questions

Decisions still waiting on the owner. When one is resolved, record it
in an ADR under [decisions/](decisions/README.md) and remove it here.

**There are no open decisions right now.**

## Resolved

| Question                         | Answer                                                       | ADR |
| -------------------------------- | ------------------------------------------------------------ | --- |
| License                          | MIT OR Apache-2.0                                            | [0017](decisions/0017-dual-mit-apache-license.md) |
| Web browser                      | Port Ladybird; re-check against Servo at v0.9                | [0018](decisions/0018-ladybird-browser.md) |
| Security beyond encryption       | Adopt S1–S6, S8, S11, S12; defer S7, S9, S10                 | [0019](decisions/0019-security-hardening-set.md) |
| Debugging without a second computer | QEMU + GDB, real devices passed into QEMU, on-screen and persistent logs | [0020](decisions/0020-single-machine-debugging.md) |
| Long-term disk layout            | USB stick → fast USB drive → NVMe dual boot at v1.0 → owner decides | [0021](decisions/0021-disk-layout-phases.md) |
| Firmware blobs                   | Not in the repo; manifest + hash-checked copy from `/usr/lib/firmware` | [0022](decisions/0022-firmware-blobs.md) |
| Repository details               | `adezdev/horus-os`, protected `main`, contributions welcome  | [0023](decisions/0023-repository-settings.md) |

## Still to verify on hardware

These are facts to test, not decisions. They are tracked in the
[verification checklist](hardware.md#verification-checklist).
