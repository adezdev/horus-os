# ADR-0018: Port Ladybird as the web browser

- **Status:** Accepted
- **Date:** 2026-10-01
- **Deciders:** Claude, at the owner's request

## Context

A daily driver needs a modern web browser, and none can be written from scratch in reasonable time. The candidates are Ladybird (C++, BSD-2-Clause, independent engine), Servo (Rust, MPL-2.0, uses the C++ SpiderMonkey JavaScript engine and a GPU renderer), or a minimal native browser.

## Decision

Make **Ladybird** the target browser. Scope the POSIX layer and the C/C++ toolchain to what Ladybird needs, and start the port after networking works (v0.8).

Ladybird's requirements become explicit POSIX-layer goals:

- a C++23 toolchain (Clang targeting `x86_64-unknown-horus`) and a C++ standard library (libc++),
- `pthread`, `mmap`, anonymous shared memory, `poll`,
- Unix domain sockets **with file-descriptor passing** (`SCM_RIGHTS`), used for its multi-process design (WebContent, RequestServer, ImageDecoder),
- ports of its third-party libraries (Skia, HarfBuzz, ICU, curl, libjpeg-turbo, libpng, libwebp, and others), each checked for license compatibility,
- a Horus windowing backend talking to `horus-compositor`.

## Consequences

- The engine came out of a hobby OS (SerenityOS), so it has a history of being portable to new systems, and its permissive license fits Horus.
- It is still pre-1.0. Some sites will break, and its status should be re-checked at v0.9.
- The port is large and adds a lot of C++ to the system, which runs as a separate program and is not linked into Horus components.
- **Checkpoint at v0.9:** compare Ladybird and Servo compatibility and portability; if Servo is clearly ahead, write a superseding ADR.
- Until the port works, browsing happens on Arch.

## Alternatives considered

- Servo: Rust is attractive, but SpiderMonkey (C++, JIT) and the GPU-based renderer make it at least as hard to port.
- Minimal native browser: poor compatibility with the modern web.
- No browser: rules out daily-driver use.
