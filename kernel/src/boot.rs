// SPDX-License-Identifier: MIT OR Apache-2.0

//! Limine boot protocol requests and the information they return.
//!
//! The requests live in `.limine_requests` between start and end markers,
//! where Limine finds them and fills in the responses before jumping to the
//! kernel (see `kernel/linker.ld`).

use limine::memmap;
use limine::request::{BootloaderInfoRequest, FramebufferRequest, HhdmRequest, MemmapRequest};
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker};

#[used]
#[unsafe(link_section = ".limine_requests_start")]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static MEMMAP: MemmapRequest = MemmapRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests_end")]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

const MIB: u64 = 1024 * 1024;

fn memmap_type_name(kind: u64) -> &'static str {
    match kind {
        memmap::MEMMAP_USABLE => "usable",
        memmap::MEMMAP_RESERVED => "reserved",
        memmap::MEMMAP_ACPI_RECLAIMABLE => "ACPI reclaimable",
        memmap::MEMMAP_ACPI_NVS => "ACPI NVS",
        memmap::MEMMAP_BAD_MEMORY => "bad memory",
        memmap::MEMMAP_BOOTLOADER_RECLAIMABLE => "bootloader reclaimable",
        memmap::MEMMAP_EXECUTABLE_AND_MODULES => "kernel and modules",
        memmap::MEMMAP_FRAMEBUFFER => "framebuffer",
        memmap::MEMMAP_MAPPED_RESERVED => "mapped reserved",
        _ => "unknown",
    }
}

/// Logs what Limine handed over: bootloader, HHDM, framebuffer, memory map.
pub fn log_boot_info() {
    if let Some(info) = BOOTLOADER_INFO.response() {
        kprintln!("boot: {} {}", info.name(), info.version());
    }
    if let Some(revision) = BASE_REVISION.actual_revision() {
        kprintln!("boot: Limine base revision {revision}");
    }
    match HHDM.response() {
        Some(hhdm) => kprintln!("boot: higher-half direct map at {:#x}", hhdm.offset),
        None => kprintln!("boot: no higher-half direct map"),
    }
    match FRAMEBUFFER
        .response()
        .and_then(|r| r.framebuffers().first().copied())
    {
        Some(fb) => kprintln!(
            "boot: framebuffer {}x{}, {} bpp, pitch {}",
            fb.width,
            fb.height,
            fb.bpp,
            fb.pitch
        ),
        None => kprintln!("boot: no framebuffer"),
    }

    let Some(memmap) = MEMMAP.response() else {
        kprintln!("boot: no memory map");
        return;
    };
    let mut usable = 0;
    kprintln!("boot: memory map:");
    for entry in memmap.entries() {
        kprintln!(
            "  {:#018x}-{:#018x} {:>10} KiB  {}",
            entry.base,
            entry.base + entry.length,
            entry.length / 1024,
            memmap_type_name(entry.type_)
        );
        if entry.type_ == memmap::MEMMAP_USABLE {
            usable += entry.length;
        }
    }
    kprintln!("boot: {} MiB usable memory", usable / MIB);
}
