// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Horus kernel.
//!
//! Limine loads this image in 64-bit long mode, mapped in the higher half,
//! and jumps to [`kmain`] on the bootstrap processor with interrupts off.

#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

#[macro_use]
mod log;

mod arch;
mod boot;
mod framebuffer;
mod panic;
mod stage;

use stage::Stage;

/// Kernel entry point, called by Limine on the bootstrap processor.
#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    if !boot::BASE_REVISION.is_supported() {
        stage::enter(Stage::Unsupported);
        kprintln!("horus: Limine does not support the requested base revision");
        arch::halt_forever();
    }
    stage::enter(Stage::Entry);
    kprintln!("horus: kernel {} starting", env!("CARGO_PKG_VERSION"));

    boot::log_boot_info();
    stage::enter(Stage::BootInfo);

    arch::cpu::log_cpu_info();

    stage::enter(Stage::Ready);
    kprintln!("horus: boot complete");
    arch::halt_forever();
}
