# Security

## Threat model

Horus is a single-user laptop OS. It defends against:

1. **Theft or loss of the laptop:** data at rest must be unreadable.
2. **Malicious or buggy software** the user runs: it shouldn't reach
   other apps' data, the user's files beyond what it was given, or the
   kernel.
3. **Network attackers:** once networking exists.
4. **Buggy drivers:** a driver fault shouldn't corrupt the kernel.

Out of scope for now: physical attacks on a running, unlocked machine
(cold boot, malicious USB devices), firmware implants, and side channels
beyond applying microcode-provided mitigations. The laptop has no
Thunderbolt controller, so external DMA attacks don't apply.

## Decided

### Full-disk encryption

Every Horus FS volume is encrypted (AES-256-XTS with Argon2id-protected
key slots). Details in [storage.md](storage.md#encryption).
[ADR-0012](../decisions/0012-full-disk-encryption.md).

## Hardening set

Decided in [ADR-0019](../decisions/0019-security-hardening-set.md):
**adopted** S1–S6, S8, S11, S12; **deferred** to after 1.0: S7, S9, S10.

| # | Protection | What it does | Cost | Milestone |
| - | ---------- | ------------ | ---- | --------- |
| S1 | **Capability-based access control** (Adopted) | Processes only use handles they were given; no global root user. This is already how the native API works ([ADR-0007](../decisions/0007-native-api-with-posix-layer.md)). Recording it as a security property makes it binding. | None extra | v0.3 |
| S2 | **Kernel hardening baseline** (Adopted) | SMEP, SMAP, UMIP, NX, W^X for kernel and user, guard pages around stacks, KASLR (Limine). All supported by the i3-1315U. | Low | v0.2–v0.3 |
| S3 | **IOMMU confinement of drivers** (Adopted) | VT-d (2 DMAR units present) limits each device's DMA to its own buffers, which makes userspace drivers actually safe. | Medium | v0.5 |
| S4 | **App sandboxing by default** (Adopted) | Apps get only a private data directory, plus files the user picks in a system file dialog (a "portal"). Camera, mic, screen capture, and network need permission prompts. | Medium | v0.7 |
| S5 | **Userspace ASLR** (Adopted) | Static-PIE executables loaded at random addresses; randomized stack, heap, and mmap bases. | Low | v0.3 |
| S6 | **Control-flow protection (CET)** (Adopted) | Shadow stacks (`user_shstk`) and indirect branch tracking (`ibt`) for the kernel and userspace. Blocks most return-oriented programming. | Medium | after v0.7 |
| S7 | **Memory protection keys** (Deferred) | `pku` lets a process lock parts of its own memory (e.g. key material) from the rest of its code. | Low | later |
| S8 | **Verified system image** (Adopted) | `/system` is read-only and covered by a hash tree, checked at boot; updates are atomic with snapshot rollback. | Medium | v1.0 |
| S9 | **Secure Boot signing** (Deferred) | Sign Limine and enroll a key or hash for the kernel so the laptop can run with Secure Boot on. Requires setting up keys in firmware. | Medium | v1.0 |
| S10 | **TPM-sealed unlock** (Deferred) | Unlock the disk key with the TPM bound to measured boot, with the passphrase as a fallback. Needs Intel PTT enabled in the BIOS (no TPM is visible today). | Medium | after v1.0 |
| S11 | **Entropy** (Adopted) | Kernel CSPRNG (ChaCha20) seeded from `RDSEED`/`RDRAND`, timing jitter, and a saved seed file; never trust one source alone. | Low | v0.3 |
| S12 | **Memory-safety discipline** (Adopted) | `unsafe` only in small, audited modules with `// SAFETY:` comments; `#![deny(unsafe_op_in_unsafe_fn)]`; fuzzing (`cargo fuzz`) for parsers: ELF, FAT32, Horus FS, ACPI/AML, USB descriptors, HID reports, network packets. | Low | from v0.1 |

The adopted protections are cheap at the milestone they're scheduled for
and much harder to add later. The deferred ones need things Horus won't
have before 1.0: a concrete use for protection keys, Secure Boot keys
for both OSes, and a TPM.

## Secure coding rules for contributors (human and AI)

- Treat all hardware-supplied and user-supplied data as untrusted:
  ACPI tables, USB descriptors, HID reports, disk contents, and
  network packets get bounds-checked parsers.
- No `unwrap()` on data from outside the kernel; return errors.
- Every syscall validates handles, rights, and user pointers before use.
- Secrets (keys, passphrases) live in zeroize-on-drop types.
