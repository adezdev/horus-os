// SPDX-License-Identifier: MIT OR Apache-2.0

//! Architecture support. Horus targets x86_64 only for now (ADR-0001).

mod x86_64;

pub use self::x86_64::*;
