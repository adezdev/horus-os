# Vision and principles

## What Horus is

Horus is a personal operating system that starts on one specific laptop
(see [hardware.md](hardware.md)). The goal is that one day the owner
can turn it on and use it all day: browse, write code, listen to music,
and close the lid, without reaching for Arch.

The name comes from the Egyptian falcon god, known for speed and sharp
sight. Those are the two qualities Horus aims for: **a fast system** and
**a clear, modern desktop**.

## Goals

1. **Daily driver on the HP 15-fd0xxx.** Every design decision is
   checked against "does this get us closer to using it every day on
   this laptop?"
2. **Fast where it is felt.** Input-to-pixel latency, boot time, app
   launch time, and compositor frame time come before raw throughput.
3. **Modern desktop.** Keyboard-driven tiling with floating windows,
   touchpad gestures, touchscreen, smooth animations, and full theming.
4. **Robust by construction.** Rust everywhere. Drivers that are not on
   the hot path run in userspace so a crash doesn't take down the system.
5. **Clean native design, with a compatibility escape hatch.** The
   native API is capability-based and async. A POSIX layer lets existing
   software be ported instead of rewritten.

## Non-goals (for now)

- Other machines or architectures. Portability is welcome where it is
  cheap, but no design decision should be made worse to support
  hardware the owner doesn't have.
- Running unmodified Linux binaries (no Linux ABI emulation).
- Binary compatibility with any other OS.
- Being a teaching OS. The code should be readable, but the target
  reader is a contributor, not a student.

## Principles

These settle trade-offs when the docs don't cover a case.

1. **The laptop is the spec.** Prefer the specific, working solution
   for this hardware over a general one that isn't needed yet.
2. **Latency over throughput.** When in doubt, schedule, allocate, and
   draw for the interactive user.
3. **Never risk the owner's data.** The NVMe drive holds the owner's
   encrypted Arch install. Horus must not write to it until a dedicated
   partition exists and the owner explicitly approves (see
   [ADR-0005](decisions/0005-usb-first-install.md)).
4. **Emulator first, hardware second.** Every feature that QEMU can
   emulate is developed and tested there before it touches real hardware.
5. **Write the decision down.** Any choice that someone would later
   ask "why?" about gets an ADR in [decisions/](decisions/README.md).
6. **Small, reviewable steps.** One logical change per commit, one
   feature per PR, each one leaving the tree bootable.
7. **Unsafe is a liability.** Every `unsafe` block carries a
   `// SAFETY:` comment, and unsafe code is kept in small, audited
   modules.
