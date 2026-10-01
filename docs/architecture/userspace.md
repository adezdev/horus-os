# Userspace

Related decisions: [ADR-0007](../decisions/0007-native-api-with-posix-layer.md),
[ADR-0009](../decisions/0009-rust-std-port-and-elf.md),
[ADR-0016](../decisions/0016-structured-shell.md).

## Layers

```
 Rust apps ─► std (Horus target) ─┐
                                  ├─► horus-rt ─► native syscalls (horus-abi)
 C/C++ apps ─► C libc ─► POSIX layer ─┘
```

## Native API

Every native interface follows the same principles:

- **Capabilities, not names.** Access is granted by passing handles.
  A process starts with exactly the handles its parent chose to give it
  (stdio, a root directory, a channel to the service registry).
- **Async first.** I/O is submitted to rings and completed later.
  Blocking calls are thin wrappers.
- **Typed messages.** IPC messages are defined once (in `horus-abi` or
  a service's interface crate) and shared by client and server.
- **Services by name through a registry.** `horus-init` runs a service
  registry; a process with the right capability can ask for
  `"horus.compositor"` and get a channel back.

## POSIX compatibility layer

A userspace library, not part of the kernel, that maps POSIX calls onto
the native API so existing C software can be ported.

| POSIX concept          | Native mapping                                          |
| ---------------------- | ------------------------------------------------------- |
| File descriptors       | Table of handles inside the library                     |
| `/`-rooted paths       | Resolved against the process's root directory handle    |
| `fork()`               | Copy-on-write address-space clone (kernel support); `posix_spawn` preferred and faster |
| `exec*()`              | Native process creation that replaces the image         |
| Signals                | Delivered as messages on an exception channel; the library runs the handler |
| `poll`/`select`/`epoll`| Multi-handle wait                                       |
| `mmap`                 | Memory objects                                          |
| BSD sockets            | Channels to the network stack                           |
| Users/permissions      | Emulated for compatibility; real access control comes from capabilities |
| `/dev`, `/proc`        | Synthetic filesystems served by userspace services      |

The goal is **source compatibility** for common software (coreutils-like
tools, compilers, editors, eventually a browser), not binary
compatibility with Linux.

## Runtimes

### Rust `std` (primary)

- A new target **`x86_64-unknown-horus`** with a `std::sys::horus`
  backend: files, directories, threads, TLS, time, environment,
  processes, pipes, sockets, and random numbers.
- Built with `-Zbuild-std` from the pinned nightly until upstreaming
  is worth the effort.
- Until the port exists (v0.3–v0.4), userspace programs are
  `#![no_std]` + `horus-rt`.

### C libc (for ported software)

- Port **relibc** (Rust, MIT, from Redox) or **mlibc** (C++, MIT, from
  Managarm) on top of the POSIX layer. Decide when porting starts
  (after v0.5); both are permissively licensed.
- A Horus target for LLVM/Clang (`x86_64-unknown-horus`) lets C/C++
  code be cross-compiled from the Arch host.

## Executables

- **ELF64**, statically linked, position-independent (`static-pie`) so
  userspace ASLR works from day one.
- Dynamic linking (`ld-horus.so`) comes later, when shared libraries
  are worth it for memory use (e.g. the UI toolkit across many apps).
- The loader maps `PT_LOAD` segments with exact permissions (W^X),
  sets up the stack (argv, env, auxv-like startup info, initial
  handles), and jumps to the entry point.

## Init and services

`horus-init` is the first process. It:

1. Mounts the root filesystem (initrd first, then Horus FS once unlocked).
2. Runs the **device manager**: matches devices to userspace drivers and
   starts them.
3. Runs the **service registry** and **service manager**: starts
   services from declarative unit files (TOML), restarts them on
   failure, and orders them by dependencies.
4. Starts the login session → compositor → user session.

## Shell

A **structured shell** (`horus-shell`) where commands exchange **typed
values**, such as records, tables, lists, strings, numbers, sizes, and
dates, instead of byte streams (similar in spirit to Nushell and
PowerShell).

```
> ls /users/me | where size > 10mb | sort-by modified | first 5
> ps | where class == "background" | get name
> devices | where bus == "usb"
```

- Built-in commands are native and return structured data directly.
- External native programs can produce structured output through a
  typed channel; plain programs fall back to text.
- POSIX `sh` for scripts and ported software comes from a ported shell
  later. `horus-shell` does not try to be `sh`-compatible.
- Runs in the text console (v0.5) and later in the desktop terminal app.
