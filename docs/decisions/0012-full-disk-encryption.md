# ADR-0012: Full-disk encryption

- **Status:** Accepted
- **Date:** 2026-09-30
- **Deciders:** owner (interview)

## Context

The owner encrypts the Arch install with LUKS and wants the same protection for Horus.

## Decision

Encrypt every Horus FS volume with **AES-256-XTS** (AES-NI/VAES), a random master key wrapped by **Argon2id** passphrase key slots, and an early-boot passphrase prompt. See [storage.md](../architecture/storage.md#encryption).

## Consequences

- Data at rest is protected if the laptop or USB stick is lost.
- The kernel needs keyboard input and a framebuffer prompt before mounting root.
- TPM unlock is possible later if Intel PTT can be enabled.

## Alternatives considered

- No encryption.
- File-level encryption only.
