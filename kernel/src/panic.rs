// SPDX-License-Identifier: MIT OR Apache-2.0

//! Kernel panic handler.

use core::panic::PanicInfo;

use crate::arch;
use crate::stage::{self, Stage};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    match info.location() {
        Some(location) => kprintln!("horus: PANIC at {location}: {}", info.message()),
        None => kprintln!("horus: PANIC: {}", info.message()),
    }
    stage::enter(Stage::Panic);
    arch::halt_forever();
}
