// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relocate GPT in a private regular file, then write through the held disk
//! handle. External partition tools never receive a real device path.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::copy_image;
use crate::{Error, Result, bail};

const SECTOR: u64 = 512;
// Linux O_TMPFILE includes O_DIRECTORY and creates an unnamed regular file.
const O_TMPFILE: i32 = 0o20200000;

struct TempImage {
    path: PathBuf,
    file: File,
}

impl TempImage {
    fn new() -> Result<Self> {
        // No directory entry can be replaced or collide with a previous run.
        // The kernel releases storage when the final handle closes, including
        // after SIGKILL. Ignore user-controlled TMPDIR during elevated runs.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_TMPFILE)
            .mode(0o600)
            .open("/tmp")
            .map_err(|err| {
                Error(format!(
                    "cannot create anonymous GPT staging file in /tmp: {err}"
                ))
            })?;
        // Refer to the parent's held fd: CLOEXEC closes that fd in sgdisk,
        // so /proc/self/fd would instead refer to the wrong process.
        let path = PathBuf::from(format!(
            "/proc/{}/fd/{}",
            std::process::id(),
            file.as_raw_fd()
        ));
        Ok(Self { path, file })
    }
}

pub(super) struct PreparedImage {
    scratch: TempImage,
    image_size: u64,
    disk_size: u64,
    tail_size: u64,
}

impl PreparedImage {
    pub(super) fn new(image: &mut File, image_size: u64, disk_size: u64) -> Result<Self> {
        if image_size < 3 * SECTOR
            || !image_size.is_multiple_of(SECTOR)
            || disk_size < image_size
            || !disk_size.is_multiple_of(SECTOR)
        {
            bail!("invalid image or disk size for GPT relocation");
        }
        let mut scratch = TempImage::new()?;
        image.seek(SeekFrom::Start(0))?;
        copy_image(image, &mut scratch.file, image_size).map_err(|err| {
            Error(format!(
                "cannot stage image in /tmp (needs up to {image_size} bytes of free space): {err}"
            ))
        })?;
        // Refuse a damaged or non-GPT source instead of letting sgdisk repair
        // or replace a partition table that the owner did not review.
        verify(&scratch.path)?;
        scratch.file.set_len(disk_size)?;
        command("--move-second-header", &scratch.path)?;
        verify(&scratch.path)?;
        let tail_size = gpt_tail_size(&mut scratch.file, disk_size)?;
        if tail_size > image_size {
            bail!("GPT backup is larger than the source image");
        }
        Ok(Self {
            scratch,
            image_size,
            disk_size,
            tail_size,
        })
    }

    pub(super) fn write_to(&mut self, output: &mut (impl Write + Seek)) -> Result {
        self.scratch.file.seek(SeekFrom::Start(0))?;
        output.seek(SeekFrom::Start(0))?;
        copy_image(&mut self.scratch.file, output, self.image_size)?;
        // Leave the unused middle of a large USB disk alone. Only the GPT
        // entries and header at its actual end need an additional write.
        let tail = self.disk_size - self.tail_size;
        self.scratch.file.seek(SeekFrom::Start(tail))?;
        output.seek(SeekFrom::Start(tail))?;
        copy_image(&mut self.scratch.file, output, self.tail_size)
    }
}

fn command(option: &str, path: &Path) -> Result<std::process::Output> {
    let output = Command::new("sgdisk")
        .env("LC_ALL", "C")
        .arg(option)
        .arg(path)
        .output()?;
    if !output.status.success() {
        bail!(
            "sgdisk {option} failed on temporary image ({}): {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}

fn verify(path: &Path) -> Result {
    let output = command("--verify", path)?;
    let text =
        String::from_utf8(output.stdout).map_err(|_| Error("invalid sgdisk output".into()))?;
    // sgdisk can repair headers in memory while loading, then report a clean
    // verification. Reject those initial warnings as well as a failed check.
    if !output.stderr.is_empty() || !text.trim_start().starts_with("No problems found.") {
        bail!(
            "GPT verification failed for temporary image: {text}{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

/// Fixed offsets are from UEFI 2.11, section 5.3.2, GPT header format:
/// https://uefi.org/specs/UEFI/2.11/05_GUID_Partition_Table_Format.html
fn gpt_tail_size(file: &mut (impl Read + Seek), disk_size: u64) -> Result<u64> {
    if disk_size < 3 * SECTOR || !disk_size.is_multiple_of(SECTOR) {
        bail!("invalid GPT disk size");
    }
    let mut header = [0u8; SECTOR as usize];
    file.seek(SeekFrom::Start(SECTOR))?;
    file.read_exact(&mut header)?;
    let entries = u32::from_le_bytes([header[80], header[81], header[82], header[83]]) as u64;
    let entry_size = u32::from_le_bytes([header[84], header[85], header[86], header[87]]) as u64;
    let table_size = entries * entry_size; // product of two u32 values fits u64.
    let table_sectors = table_size.div_ceil(SECTOR);
    let mut alternate = [0u8; 8];
    alternate.copy_from_slice(&header[32..40]);
    if &header[..8] != b"EFI PART"
        || u64::from_le_bytes(alternate) != disk_size / SECTOR - 1
        || entry_size < 128
        || !entry_size.is_multiple_of(128)
        || !(16 * 1024..=1024 * 1024).contains(&table_size)
        || table_sectors + 1 >= disk_size / SECTOR
    {
        bail!("invalid relocated GPT geometry");
    }
    let mut backup = [0u8; SECTOR as usize];
    file.seek(SeekFrom::Start(disk_size - SECTOR))?;
    file.read_exact(&mut backup)?;
    let mut table_lba = [0u8; 8];
    table_lba.copy_from_slice(&backup[72..80]);
    if &backup[..8] != b"EFI PART"
        || backup[24..32] != header[32..40]
        || backup[32..40] != header[24..32]
        || backup[80..88] != header[80..88]
        || u64::from_le_bytes(table_lba) != disk_size / SECTOR - 1 - table_sectors
    {
        bail!("invalid relocated backup GPT geometry");
    }
    Ok((table_sectors + 1) * SECTOR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    const IMAGE_SIZE: u64 = 8 * 1024 * 1024;

    #[test]
    fn staging_is_anonymous_and_private() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let image = TempImage::new().unwrap();
        assert_eq!(
            image.path.parent(),
            Some(Path::new(&format!("/proc/{}/fd", std::process::id())))
        );
        assert_eq!(image.file.metadata().unwrap().nlink(), 0);
        assert_eq!(
            image.file.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&image.path).unwrap().ino(),
            image.file.metadata().unwrap().ino()
        );
    }

    fn source() -> TempImage {
        let mut image = TempImage::new().unwrap();
        image.file.set_len(IMAGE_SIZE).unwrap();
        let status = Command::new("sgdisk")
            .args(["--clear", "--new=1:2048:4095", "--typecode=1:ef00"])
            .arg(&image.path)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        image
            .file
            .seek(SeekFrom::Start(1024 * 1024 + 8192))
            .unwrap();
        image.file.write_all(b"partition contents").unwrap();
        image
    }

    #[test]
    fn relocate_backup_gpt_without_changing_partition_contents() {
        let mut source = source();
        for disk_size in [IMAGE_SIZE, 32 * 1024 * 1024, 128_043_712_512] {
            let mut prepared = PreparedImage::new(&mut source.file, IMAGE_SIZE, disk_size).unwrap();
            let mut output = TempImage::new().unwrap();
            output.file.set_len(disk_size).unwrap();
            // Bytes outside the image and backup GPT must not be overwritten.
            if disk_size > IMAGE_SIZE {
                output
                    .file
                    .seek(SeekFrom::Start(IMAGE_SIZE + 1024))
                    .unwrap();
                output.file.write_all(b"leave unused space alone").unwrap();
            }
            prepared.write_to(&mut output.file).unwrap();
            output.file.sync_all().unwrap();
            verify(&output.path).unwrap();
            assert_eq!(
                gpt_tail_size(&mut output.file, disk_size).unwrap(),
                33 * SECTOR
            );
            let mut contents = [0; 18];
            output
                .file
                .seek(SeekFrom::Start(1024 * 1024 + 8192))
                .unwrap();
            output.file.read_exact(&mut contents).unwrap();
            assert_eq!(&contents, b"partition contents");
            if disk_size > IMAGE_SIZE {
                let mut contents = [0; 24];
                output
                    .file
                    .seek(SeekFrom::Start(IMAGE_SIZE + 1024))
                    .unwrap();
                output.file.read_exact(&mut contents).unwrap();
                assert_eq!(&contents, b"leave unused space alone");
            }
        }
        // The original image still has its backup at its own end and verifies.
        verify(&source.path).unwrap();
        assert_eq!(source.file.metadata().unwrap().len(), IMAGE_SIZE);
        assert_eq!(
            gpt_tail_size(&mut source.file, IMAGE_SIZE).unwrap(),
            33 * SECTOR
        );
    }

    #[test]
    fn refuse_corrupt_source_and_invalid_disk_geometry() {
        let mut source = source();
        for disk_size in [IMAGE_SIZE - SECTOR, IMAGE_SIZE + 1] {
            assert!(PreparedImage::new(&mut source.file, IMAGE_SIZE, disk_size).is_err());
        }
        // Damage the header's CRC in both copies; automatic repair is refused.
        for position in [SECTOR + 16, IMAGE_SIZE - SECTOR + 16] {
            source.file.seek(SeekFrom::Start(position)).unwrap();
            source.file.write_all(&[0; 4]).unwrap();
        }
        assert!(PreparedImage::new(&mut source.file, IMAGE_SIZE, IMAGE_SIZE * 2).is_err());
    }
}
