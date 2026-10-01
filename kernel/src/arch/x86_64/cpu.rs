// SPDX-License-Identifier: MIT OR Apache-2.0

//! CPU identification through `CPUID`.

use core::arch::x86_64::{__cpuid, __cpuid_count, CpuidResult};

/// Hybrid core type reported by `CPUID.1Ah:EAX[31:24]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreType {
    /// Efficiency core (Intel Atom microarchitecture, e.g. Gracemont).
    Efficiency,
    /// Performance core (Intel Core microarchitecture, e.g. Raptor Cove).
    Performance,
    /// A core type this kernel doesn't know.
    Other(u8),
}

fn cpuid(leaf: u32) -> CpuidResult {
    __cpuid(leaf)
}

fn cpuid_count(leaf: u32, subleaf: u32) -> CpuidResult {
    __cpuid_count(leaf, subleaf)
}

fn max_basic_leaf() -> u32 {
    cpuid(0).eax
}

fn max_extended_leaf() -> u32 {
    cpuid(0x8000_0000).eax
}

/// Copies the little-endian bytes of `regs` into `out`.
fn copy_regs(out: &mut [u8], regs: &[u32]) {
    for (chunk, reg) in out.as_chunks_mut::<4>().0.iter_mut().zip(regs) {
        *chunk = reg.to_le_bytes();
    }
}

/// The 12-byte vendor string, e.g. `GenuineIntel`.
pub fn vendor(buf: &mut [u8; 12]) -> &str {
    let r = cpuid(0);
    copy_regs(buf, &[r.ebx, r.edx, r.ecx]);
    core::str::from_utf8(buf).unwrap_or("?")
}

/// The processor brand string, e.g. `13th Gen Intel(R) Core(TM) i3-1315U`.
pub fn brand(buf: &mut [u8; 48]) -> &str {
    if max_extended_leaf() < 0x8000_0004 {
        return "unknown";
    }
    for (i, leaf) in (0x8000_0002..=0x8000_0004).enumerate() {
        let r = cpuid(leaf);
        copy_regs(
            &mut buf[i * 16..(i + 1) * 16],
            &[r.eax, r.ebx, r.ecx, r.edx],
        );
    }
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len]).map_or("unknown", str::trim)
}

/// Whether the CPU mixes core types (`CPUID.07h:EDX[15]`).
pub fn is_hybrid() -> bool {
    max_basic_leaf() >= 7 && cpuid_count(7, 0).edx & (1 << 15) != 0
}

/// The type of the core this code runs on, if the CPU is hybrid.
pub fn core_type() -> Option<CoreType> {
    if !is_hybrid() || max_basic_leaf() < 0x1A {
        return None;
    }
    Some(match (cpuid(0x1A).eax >> 24) as u8 {
        0x20 => CoreType::Efficiency,
        0x40 => CoreType::Performance,
        other => CoreType::Other(other),
    })
}

/// Logs the CPU vendor, brand, and the bootstrap core's type.
pub fn log_cpu_info() {
    let mut vendor_buf = [0; 12];
    let mut brand_buf = [0; 48];
    kprintln!(
        "cpu: {} ({})",
        brand(&mut brand_buf),
        vendor(&mut vendor_buf)
    );
    match core_type() {
        Some(kind) => kprintln!("cpu: hybrid, bootstrap processor is {kind:?}"),
        None => kprintln!("cpu: not hybrid (or core type not reported)"),
    }
}
