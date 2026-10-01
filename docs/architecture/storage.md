# Storage

Related decisions: [ADR-0008](../decisions/0008-custom-cow-filesystem.md),
[ADR-0012](../decisions/0012-full-disk-encryption.md),
[ADR-0005](../decisions/0005-usb-first-install.md).

## Stack

```
 applications (std::fs, POSIX layer)
          │  rings / channels with directory and file capabilities
          ▼
 ┌────────────────────── VFS ───────────────────────┐
 │ path lookup · mounts · dentry cache · page cache │
 └──────┬───────────────┬──────────────────┬────────┘
     Horus FS          FAT32          initrd (read-only, in RAM)
        │                 │
 ┌──────┴──────┐          │
 │ encryption  │          │      (FAT32 is never encrypted: the ESP)
 └──────┬──────┘          │
 ┌──────┴─────────────────┴──┐
 │ block layer: request queue│  merging, per-CPU queues, TRIM
 │ GPT partitions            │
 └──────┬──────────────┬─────┘
       NVMe       USB mass storage
```

## VFS

- **Capability-based paths:** a process has no global root unless it
  is given one. It receives directory handles, and paths resolve
  relative to them. The POSIX layer simulates `/` with the root handle
  the process was given.
- **Page cache** shared with memory-mapped files; write-back with
  per-filesystem flush ordering.
- **Async I/O** through rings by default; blocking calls are wrappers.
- Standard namespace (as presented by the POSIX layer):

```
/system      read-only OS files (kernel, drivers, services, libraries)
/apps        installed applications
/users/<u>   home directories
/config      system configuration
/var         logs, caches, state
/boot        the ESP (FAT32), mounted read-only by default
/devices     device nodes for POSIX compatibility
```

## Horus FS

A Horus-native filesystem designed for SSDs and for this OS.

### Goals

1. **Never lose data on power loss or crash.** Copy-on-write: a write
   never overwrites live data. A commit is one atomic superblock update.
2. **Detect corruption.** Every metadata and data block carries a checksum.
3. **Fast on flash.** Extent-based allocation, large sequential writes,
   TRIM/discard support, minimal write amplification.
4. **Snapshots.** Cheap, read-only snapshots of the system before
   updates, so a failed update can be rolled back.
5. **Simple enough to verify.** The format is specified in
   `docs/specs/horus-fs.md` (to be written in v0.5) and implemented in
   the shared `horus-fs-format` crate, which a host-side `mkfs`/`fsck`
   tool and the kernel both use.

### Design sketch (v1)

| Area          | Design                                                         |
| ------------- | -------------------------------------------------------------- |
| Block size    | 4 KiB                                                          |
| Structure     | Copy-on-write B+trees (inodes, directory entries, extents, free space) |
| Commits       | Two superblock copies, alternated; each points to a tree root plus a generation number |
| Checksums     | Per block, stored in the parent pointer (Merkle-style); BLAKE3 or CRC32C (decide in the spec, weighing speed against strength) |
| Data          | Extents; small files inline in the inode                       |
| Snapshots     | A snapshot is a retained tree root; reference counts on extents |
| Compression   | Per-file LZ4 or Zstd (later)                                   |
| Encryption    | Handled one layer below (whole volume)                         |
| Max sizes     | 64-bit block addresses and file sizes                          |
| Tools         | `horus-mkfs`, `horus-fsck`, and an image builder in `xtask`; all run on Linux too |

### What v1 leaves out

RAID, deduplication, quotas, and send/receive. The format reserves
feature flags for them.

## FAT32

Needed for the ESP on the USB stick and later on the NVMe. Supports
read/write, long file names, and no permissions (mounted with
fixed ownership). exFAT can be added later for large USB drives.

## Encryption

Full-disk encryption of every Horus FS volume.

| Item          | Choice                                                        |
| ------------- | ------------------------------------------------------------- |
| Cipher        | AES-256-XTS, accelerated with AES-NI / VAES + VPCLMULQDQ      |
| Sector size   | 4 KiB encryption units                                        |
| Key hierarchy | Random volume master key, wrapped by one or more key slots     |
| Key slots     | Passphrase via **Argon2id** (memory-hard); TPM slot later if Intel PTT can be enabled |
| Header        | Horus-specific header with a backup copy; a magic value and version |
| Unlock        | Early-boot passphrase prompt on the framebuffer before the root FS mounts |
| Integrity     | Horus FS checksums detect tampering at the block level; authenticated encryption is a later option |

The master key lives only in kernel memory, in pages that are never
swapped and are zeroed on shutdown or suspend-to-disk.

## Safety rules for the NVMe

- The NVMe driver ships **read-only** (write commands compiled out)
  until the owner approves a Horus partition.
- Even after that, Horus writes **only** to partitions with the Horus
  partition-type GUID. It never writes to the existing ESP, the LUKS
  partition, or unknown partitions.
- `cargo xtask flash` refuses to target any NVMe device.
