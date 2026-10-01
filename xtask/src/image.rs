// SPDX-License-Identifier: MIT OR Apache-2.0

//! Builds the bootable disk image: GPT with one FAT32 EFI System Partition
//! holding Limine, its configuration, and the kernel.
//!
//! Uses `sgdisk` and mtools, so no root access or loop devices are needed.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, process};

use crate::{Options, Result, bail, build_kernel, root, run_command, target_dir};

const SECTOR: u64 = 512;
const MIB: u64 = 1024 * 1024;
/// The ESP starts at 1 MiB (sector 2048), the usual alignment.
const ESP_OFFSET: u64 = MIB;
/// ESP size, as in `docs/architecture/boot.md`.
const ESP_SIZE: u64 = 512 * MIB;
/// Room after the ESP for the backup GPT.
const TAIL: u64 = MIB;

fn limine_dir() -> PathBuf {
    env::var_os("LIMINE_DIR").map_or_else(|| PathBuf::from("/usr/share/limine"), PathBuf::from)
}

/// Builds `target/horus.img` and returns its path.
pub fn build(options: &Options) -> Result<PathBuf> {
    let kernel = build_kernel(options)?;
    let limine_efi = limine_dir().join("BOOTX64.EFI");
    if !limine_efi.is_file() {
        bail!(
            "{} not found; install Limine or set LIMINE_DIR",
            limine_efi.display()
        );
    }

    // Build into a temporary file, then rename it into place. A QEMU still
    // running from the previous image keeps that file, instead of having
    // it rewritten underneath it.
    let image = target_dir().join("horus.img");
    let partial = target_dir().join(format!("horus.img.{}.partial", process::id()));
    let result = write_image(&partial, &kernel, &limine_efi)
        .and_then(|()| fs::rename(&partial, &image).map_err(Into::into));
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result?;

    println!("xtask: built {}", image.display());
    Ok(image)
}

/// Writes a complete disk image to `image`.
fn write_image(image: &Path, kernel: &Path, limine_efi: &Path) -> Result {
    // Start from an empty, sparse file.
    File::create(image)?.set_len(ESP_OFFSET + ESP_SIZE + TAIL)?;

    run_command(
        Command::new("sgdisk")
            .arg("--clear")
            .args([
                &format!(
                    "--new=1:{}:{}",
                    ESP_OFFSET / SECTOR,
                    (ESP_OFFSET + ESP_SIZE) / SECTOR - 1
                ),
                "--typecode=1:ef00",
                "--change-name=1:Horus ESP",
            ])
            .arg(image)
            .stdout(std::process::Stdio::null()),
    )?;

    let fat = format!("{}@@{}", image.display(), ESP_OFFSET);
    run_command(Command::new("mformat").args([
        "-i",
        &fat,
        "-F",
        "-v",
        "HORUS",
        "-T",
        &(ESP_SIZE / SECTOR).to_string(),
        "-h",
        "64",
        "-s",
        "32",
        "::",
    ]))?;
    run_command(Command::new("mmd").args(["-i", &fat]).args([
        "::/EFI",
        "::/EFI/BOOT",
        "::/boot",
        "::/boot/limine",
        "::/boot/horus",
        "::/boot/horus/licenses",
    ]))?;
    copy_in(&fat, limine_efi, "::/EFI/BOOT/BOOTX64.EFI")?;
    copy_in(
        &fat,
        &root().join("boot/limine.conf"),
        "::/boot/limine/limine.conf",
    )?;
    copy_in(&fat, kernel, "::/boot/horus/kernel")?;
    // Third-party notices for what's compiled into the kernel.
    copy_in(
        &fat,
        &root().join("kernel/assets/fonts/LICENSE-spleen"),
        "::/boot/horus/licenses/spleen.txt",
    )
}

fn copy_in(fat: &str, source: &Path, dest: &str) -> Result {
    run_command(
        Command::new("mcopy")
            .args(["-i", fat])
            .arg(source)
            .arg(dest),
    )
}
