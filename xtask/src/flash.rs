// SPDX-License-Identifier: MIT OR Apache-2.0

//! Linux USB flashing. Tests exercise policy and copying without block devices.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::process::Command;

use crate::{Error, Result, bail, target_dir};

// Linux open(2) flags: exclusive block-device claim and no symlink following.
const O_EXCL: i32 = 0o200;
const O_NOFOLLOW: i32 = 0o400000;
const COLUMNS: [&str; 7] = [
    "NAME",
    "TYPE",
    "TRAN",
    "SIZE",
    "MODEL",
    "MOUNTPOINTS",
    "LOG-SEC",
];

#[derive(Debug, PartialEq, Eq)]
struct Device {
    name: String,
    kind: String,
    transport: String,
    size: u64,
    model: String,
    mounts: String,
    sector_size: u64,
}

pub fn run(args: &[String]) -> Result {
    let requested = device_argument(args)?;
    refuse_nvme(requested)?;
    let device = fs::canonicalize(requested)?;
    let metadata = fs::metadata(&device)?;
    validate_target(&device, metadata.file_type().is_block_device())?;

    // Open the existing image once: concurrent image builds replace its path
    // atomically, leaving this file descriptor attached to the reviewed image.
    let image_path = target_dir().join("horus.img");
    let mut image = File::open(&image_path).map_err(|err| {
        Error(format!(
            "{}: {err}; run cargo xtask image first",
            image_path.display()
        ))
    })?;
    let image_metadata = image.metadata()?;
    if !image_metadata.is_file() {
        bail!("{} is not a regular image file", image_path.display());
    }
    let image_size = image_metadata.len();
    let devices = inspect(&device)?;
    validate_devices(&devices, &device, image_size)?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    confirm(&devices, image_size, &mut stdin.lock(), &mut stdout.lock())?;

    // O_EXCL rejects a device in use (including mounts in other namespaces)
    // and holds the claim through sync_all. Do not create or truncate a path.
    let mut output = OpenOptions::new()
        .write(true)
        .custom_flags(O_EXCL | O_NOFOLLOW)
        .open(&device)
        .map_err(|err| Error(format!("cannot exclusively open {}: {err}; device must be unused and you need write permission (see setup.md)", device.display())))?;
    let opened = output.metadata()?;
    validate_target(&device, opened.file_type().is_block_device())?;
    if opened.rdev() != metadata.rdev() || fs::canonicalize(requested)? != device {
        bail!("device changed after confirmation; refusing to write");
    }
    // Repeat the full lsblk policy after the prompt and exclusive open.
    let current = inspect(&device)?;
    validate_snapshot(&devices, &current, &device, image_size)?;

    println!("xtask: writing {image_size} bytes to {}", device.display());
    copy_image(&mut image, &mut output, image_size)?;
    output.sync_all().map_err(|err| {
        Error(format!(
            "failed to flush device: {err}; flash again before booting"
        ))
    })?;
    println!(
        "xtask: flashed and synced {}; safe to unplug",
        device.display()
    );
    Ok(())
}

fn device_argument(args: &[String]) -> Result<&Path> {
    match args {
        [argument] if !argument.starts_with('-') => Ok(Path::new(argument)),
        _ => bail!("usage: cargo xtask flash /dev/sdX (exactly one device, no options)"),
    }
}

fn refuse_nvme(path: &Path) -> Result {
    if path.to_string_lossy().starts_with("/dev/nvme") {
        bail!("refusing NVMe target {}", path.display());
    }
    Ok(())
}

fn validate_target(path: &Path, is_block: bool) -> Result {
    refuse_nvme(path)?;
    let name = path.to_str().and_then(|name| name.strip_prefix("/dev/sd"));
    if !is_block
        || !name.is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_lowercase())
        })
    {
        bail!("{} must be a whole block device (/dev/sdX)", path.display());
    }
    Ok(())
}

fn inspect(path: &Path) -> Result<Vec<Device>> {
    let output = Command::new("lsblk")
        .env("LC_ALL", "C")
        .args([
            "--pairs",
            "--bytes",
            "--paths",
            "--output",
            &COLUMNS.join(","),
            "--",
        ])
        .arg(path)
        .output()
        .map_err(|err| Error(format!("failed to run lsblk: {err}")))?;
    if !output.status.success() {
        bail!(
            "lsblk failed for {} ({}): {}",
            path.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| Error("lsblk returned invalid UTF-8".into()))?;
    parse_devices(&text)
}

/// lsblk --pairs quotes each field and hex-escapes unsafe bytes, including
/// quotes, backslashes, and newlines in multiple mount points.
fn parse_devices(text: &str) -> Result<Vec<Device>> {
    text.lines()
        .map(|mut line| {
            let mut fields = Vec::new();
            for key in COLUMNS {
                let prefix = format!("{key}=\"");
                let value = line
                    .trim_start()
                    .strip_prefix(&prefix)
                    .ok_or_else(|| Error(format!("invalid lsblk field {key}")))?;
                let end = value
                    .find('"')
                    .ok_or_else(|| Error("unterminated lsblk field".into()))?;
                fields.push(decode(&value[..end])?);
                line = &value[end + 1..];
            }
            if !line.trim().is_empty() {
                bail!("unexpected trailing lsblk data");
            }
            let number = |index: usize| {
                fields[index]
                    .parse::<u64>()
                    .map_err(|_| Error(format!("invalid lsblk {}", COLUMNS[index])))
            };
            Ok(Device {
                name: fields[0].clone(),
                kind: fields[1].clone(),
                transport: fields[2].clone(),
                size: number(3)?,
                model: fields[4].clone(),
                mounts: fields[5].clone(),
                sector_size: number(6)?,
            })
        })
        .collect()
}

fn decode(value: &str) -> Result<String> {
    let mut bytes = value.bytes();
    let mut decoded = Vec::new();
    while let Some(byte) = bytes.next() {
        if byte == b'\\' {
            if bytes.next() != Some(b'x') {
                bail!("invalid lsblk escape");
            }
            let high = bytes.next().and_then(|b| (b as char).to_digit(16));
            let low = bytes.next().and_then(|b| (b as char).to_digit(16));
            match (high, low) {
                (Some(high), Some(low)) => decoded.push((high * 16 + low) as u8),
                _ => bail!("invalid lsblk hex escape"),
            }
        } else {
            decoded.push(byte);
        }
    }
    String::from_utf8(decoded).map_err(|_| Error("invalid UTF-8 in lsblk field".into()))
}

fn validate_devices(devices: &[Device], path: &Path, image_size: u64) -> Result {
    let Some(disk) = devices.first() else {
        bail!("lsblk returned no device information");
    };
    if Path::new(&disk.name) != path || disk.kind != "disk" {
        bail!("lsblk target must be the requested whole disk");
    }
    if disk.transport != "usb" {
        bail!(
            "{} transport is {:?}, not usb; refusing",
            path.display(),
            disk.transport
        );
    }
    if devices.iter().any(|device| !device.mounts.is_empty()) {
        bail!(
            "{} or a descendant is mounted or used as swap; unmount it first",
            path.display()
        );
    }
    if devices.iter().skip(1).any(|device| device.kind != "part") {
        bail!("{} has active device holders; refusing", path.display());
    }
    if disk.sector_size != 512 {
        bail!("the image requires 512-byte logical sectors");
    }
    if image_size == 0 || !image_size.is_multiple_of(512) || image_size > disk.size {
        bail!("image must be nonempty, sector aligned, and fit the device");
    }
    Ok(())
}

fn confirm(
    devices: &[Device],
    image_size: u64,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result {
    let disk = &devices[0]; // validate_devices has already checked this.
    writeln!(
        output,
        "Device: {}\nModel: {:?}\nSize: {} bytes\nCurrent partitions:",
        disk.name, disk.model, disk.size
    )?;
    for partition in devices.iter().skip(1) {
        writeln!(output, "  {} ({} bytes)", partition.name, partition.size)?;
    }
    if devices.len() == 1 {
        writeln!(output, "  (none)")?;
    }
    write!(
        output,
        "ALL DATA on {} will be overwritten with the {image_size}-byte Horus image.\nType {} to confirm: ",
        disk.name, disk.name
    )?;
    output.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    let answer = answer
        .strip_suffix('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    if answer != Some(disk.name.as_str()) {
        bail!("confirmation did not match; nothing written");
    }
    Ok(())
}

fn validate_snapshot(
    original: &[Device],
    current: &[Device],
    path: &Path,
    image_size: u64,
) -> Result {
    validate_devices(current, path, image_size)?;
    if current != original {
        bail!("device details changed after confirmation; refusing to write");
    }
    Ok(())
}

fn copy_image(input: &mut impl Read, output: &mut impl Write, size: u64) -> Result {
    let copied = io::copy(&mut input.take(size), output).map_err(|err| {
        Error(format!(
            "flash copy failed: {err}; device may contain an incomplete image"
        ))
    })?;
    if copied != size {
        bail!("image ended early; device may contain an incomplete image");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const DISK: &str = "NAME=\"/dev/sdb\" TYPE=\"disk\" TRAN=\"usb\" SIZE=\"1073741824\" MODEL=\"USB\\x20Stick\" MOUNTPOINTS=\"\" LOG-SEC=\"512\"";
    const PARTITION: &str = "NAME=\"/dev/sdb1\" TYPE=\"part\" TRAN=\"usb\" SIZE=\"536870912\" MODEL=\"\" MOUNTPOINTS=\"\" LOG-SEC=\"512\"";

    fn devices() -> Vec<Device> {
        parse_devices(&format!("{DISK}\n{PARTITION}\n")).unwrap()
    }

    fn rejected(result: Result, message: &str) {
        let error = result.expect_err("must reject unsafe input");
        assert!(error.to_string().contains(message), "{error}");
    }

    #[test]
    fn require_one_device_argument_without_options() {
        for args in [
            vec![],
            vec!["--help"],
            vec!["--release"],
            vec!["/dev/sdb", "--yes"],
            vec!["/dev/sdb", "/dev/sdc"],
        ] {
            let args = args.into_iter().map(String::from).collect::<Vec<_>>();
            assert!(device_argument(&args).is_err());
        }
        let args = vec!["/dev/sdb".into()];
        assert_eq!(device_argument(&args).unwrap(), Path::new("/dev/sdb"));
    }

    #[test]
    fn require_whole_block_device_names() {
        for name in ["/dev/sda", "/dev/sdz", "/dev/sdaa"] {
            assert!(validate_target(Path::new(name), true).is_ok());
            rejected(
                validate_target(Path::new(name), false),
                "whole block device",
            );
        }
        for name in [
            "/dev/sda1",
            "/dev/sda/../nvme0n1",
            "/dev/sd",
            "/dev/vda",
            "/dev/loop0",
            "/tmp/sdb",
            "/dev/sdA",
        ] {
            rejected(validate_target(Path::new(name), true), "whole block device");
        }
        // The same check runs on the canonical target, so NVMe aliases fail.
        for name in ["/dev/nvme0n1", "/dev/nvme0n1p1", "/dev/nvme1n1"] {
            rejected(validate_target(Path::new(name), true), "NVMe");
        }
    }

    #[test]
    fn accept_usb_disk_with_unmounted_partitions() {
        let devices = devices();
        assert_eq!(devices[0].model, "USB Stick");
        assert_eq!(devices[1].name, "/dev/sdb1");
        assert!(validate_devices(&devices, Path::new("/dev/sdb"), 514 * 1024 * 1024).is_ok());
    }

    #[test]
    fn require_usb_and_matching_disk() {
        for transport in ["", "sata", "nvme", "virtio", "USB"] {
            let mut devices = devices();
            devices[0].transport = transport.into();
            rejected(
                validate_devices(&devices, Path::new("/dev/sdb"), 512),
                "transport",
            );
        }
        let mut devices = devices();
        devices[0].kind = "part".into();
        rejected(
            validate_devices(&devices, Path::new("/dev/sdb"), 512),
            "whole disk",
        );
        devices[0].kind = "disk".into();
        rejected(
            validate_devices(&devices, Path::new("/dev/sdc"), 512),
            "requested",
        );
    }

    #[test]
    fn refuse_mounts_swap_and_active_holders() {
        for index in 0..2 {
            for mount in ["/media/usb", "[SWAP]", "/one\n/two", " "] {
                let mut devices = devices();
                devices[index].mounts = mount.into();
                rejected(
                    validate_devices(&devices, Path::new("/dev/sdb"), 512),
                    "mounted or used as swap",
                );
            }
        }
        let mut devices = devices();
        devices[1].kind = "crypt".into();
        devices[1].mounts = "/home".into();
        rejected(
            validate_devices(&devices, Path::new("/dev/sdb"), 512),
            "mounted",
        );
        devices[1].mounts.clear();
        rejected(
            validate_devices(&devices, Path::new("/dev/sdb"), 512),
            "holders",
        );
    }

    #[test]
    fn refuse_missing_geometry_and_images_that_do_not_fit() {
        rejected(
            validate_devices(&[], Path::new("/dev/sdb"), 512),
            "no device",
        );
        for size in [0, 1, 513, 1073741824 + 512] {
            rejected(
                validate_devices(&devices(), Path::new("/dev/sdb"), size),
                "image must",
            );
        }
        assert!(validate_devices(&devices(), Path::new("/dev/sdb"), 1073741824).is_ok());
        let mut devices = devices();
        devices[0].sector_size = 4096;
        rejected(
            validate_devices(&devices, Path::new("/dev/sdb"), 512),
            "512-byte",
        );
    }

    #[test]
    fn fail_closed_on_malformed_lsblk_output() {
        for text in [
            DISK.replace("SIZE=\"1073741824\"", "SIZE=\"unknown\""),
            DISK.replace("LOG-SEC=\"512\"", "LOG-SEC=\"\""),
            DISK.replace("TRAN=\"usb\" ", ""),
            DISK.replace("\\x20", "\\xzz"),
            DISK.replace("\\x20", "\\xff"),
            DISK.replace("\\x20", "\\n"),
            DISK.replace("\\x20", "\\x"),
            format!("{DISK} unexpected"),
            format!("{DISK}\n\n{PARTITION}"),
            DISK[..DISK.len() - 1].into(),
        ] {
            assert!(parse_devices(&text).is_err(), "accepted {text}");
        }
    }

    #[test]
    fn decode_multiple_mounts_and_quoted_model() {
        let text = DISK
            .replace("USB\\x20Stick", "USB\\x22Stick\\x5c")
            .replace("MOUNTPOINTS=\"\"", "MOUNTPOINTS=\"/first\\x0a/second\"");
        let devices = parse_devices(&text).unwrap();
        assert_eq!(devices[0].model, "USB\"Stick\\");
        assert_eq!(devices[0].mounts, "/first\n/second");
        rejected(
            validate_devices(&devices, Path::new("/dev/sdb"), 512),
            "mounted",
        );
    }

    #[test]
    fn require_exact_confirmation_and_display_details() {
        for answer in ["/dev/sdb\n", "/dev/sdb\r\n"] {
            let mut output = Vec::new();
            assert!(confirm(&devices(), 512, &mut Cursor::new(answer), &mut output).is_ok());
            let text = String::from_utf8(output).unwrap();
            for detail in [
                "/dev/sdb",
                "USB Stick",
                "1073741824 bytes",
                "/dev/sdb1",
                "536870912 bytes",
                "ALL DATA",
                "512-byte",
                "Type /dev/sdb",
            ] {
                assert!(text.contains(detail), "missing {detail}");
            }
        }
        for answer in [
            "",
            "\n",
            "/dev/sdb",
            "/dev/sdb\r\r\n",
            "yes\n",
            "sdb\n",
            "/dev/sdc\n",
            " /dev/sdb\n",
            "/dev/sdb \n",
        ] {
            rejected(
                confirm(&devices(), 512, &mut Cursor::new(answer), &mut Vec::new()),
                "nothing written",
            );
        }
        let mut devices = devices();
        devices.truncate(1);
        let mut output = Vec::new();
        assert!(confirm(&devices, 512, &mut Cursor::new("/dev/sdb\n"), &mut output).is_ok());
        assert!(String::from_utf8(output).unwrap().contains("(none)"));
    }

    #[test]
    fn refuse_device_changes_after_confirmation() {
        let original = devices();
        let path = Path::new("/dev/sdb");
        assert!(validate_snapshot(&original, &original, path, 512).is_ok());
        let mut changed = devices();
        changed[0].model = "Different stick".into();
        rejected(
            validate_snapshot(&original, &changed, path, 512),
            "changed after confirmation",
        );
        changed = devices();
        changed[1].size += 512;
        rejected(
            validate_snapshot(&original, &changed, path, 512),
            "changed after confirmation",
        );
        changed = devices();
        changed[1].mounts = "/media/usb".into();
        rejected(validate_snapshot(&original, &changed, path, 512), "mounted");
        changed = devices();
        changed[0].transport = "sata".into();
        rejected(
            validate_snapshot(&original, &changed, path, 512),
            "transport",
        );
    }

    #[test]
    fn copy_exact_image_bytes_and_propagate_failures() {
        let bytes = vec![0x5a; 1024];
        let mut output = Vec::new();
        assert!(copy_image(&mut Cursor::new(&bytes), &mut output, 512).is_ok());
        assert_eq!(output, &bytes[..512]);
        rejected(
            copy_image(&mut Cursor::new([1, 2]), &mut Vec::new(), 512),
            "incomplete image",
        );
        // A full slice writer accepts no bytes and produces WriteZero.
        let mut full: &mut [u8] = &mut [];
        rejected(
            copy_image(&mut Cursor::new(&bytes), &mut full, 512),
            "copy failed",
        );
    }
}
