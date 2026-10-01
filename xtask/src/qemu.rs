// SPDX-License-Identifier: MIT OR Apache-2.0

//! Boots the Horus image in QEMU, interactively or as an automated test.
//!
//! The virtual machine matches the laptop's boot path: UEFI firmware (OVMF),
//! and the image attached as USB storage on an xHCI controller.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, thread};

use crate::{Options, Result, bail, image, run_command, target_dir};

const OVMF_CODE_PATHS: &[&str] = &[
    "/usr/share/edk2/x64/OVMF_CODE.4m.fd", // Arch
    "/usr/share/OVMF/OVMF_CODE_4M.fd",     // Debian, Ubuntu
];
const OVMF_VARS_PATHS: &[&str] = &[
    "/usr/share/edk2/x64/OVMF_VARS.4m.fd",
    "/usr/share/OVMF/OVMF_VARS_4M.fd",
];

/// Printed by the kernel when early boot succeeds.
const BOOT_OK: &str = "horus: boot complete";
/// Printed by the kernel's panic handler.
const PANIC: &str = "horus: PANIC";
/// TCG emulation in CI is slow; KVM boots in a few seconds.
const TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn find_firmware(var: &str, candidates: &[&str]) -> Result<PathBuf> {
    if let Some(path) = env::var_os(var) {
        return Ok(PathBuf::from(path));
    }
    match candidates.iter().map(Path::new).find(|path| path.is_file()) {
        Some(path) => Ok(path.to_path_buf()),
        None => bail!("UEFI firmware not found; install OVMF (edk2-ovmf) or set {var}"),
    }
}

/// Whether KVM can be used by this user.
fn kvm_available() -> bool {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/kvm")
        .is_ok()
}

/// The QEMU command for `image`, without the debug console setting.
fn qemu_command(image: &Path, options: &Options) -> Result<Command> {
    let code = find_firmware("OVMF_CODE", OVMF_CODE_PATHS)?;
    // The firmware writes to its variable store (boot entries, display
    // settings), so every run starts from a fresh copy of the template.
    // Otherwise one run's state could change how the next one boots.
    let vars = target_dir().join("OVMF_VARS.fd");
    fs::copy(find_firmware("OVMF_VARS", OVMF_VARS_PATHS)?, &vars)?;

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(["-machine", "q35", "-smp", "8", "-m", "4G", "-no-reboot"]);
    if !options.no_kvm && kvm_available() {
        cmd.args(["-enable-kvm", "-cpu", "host"]);
    } else {
        cmd.args(["-cpu", "max"]);
    }
    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,readonly=on,file={}",
        code.display()
    ));
    cmd.arg("-drive")
        .arg(format!("if=pflash,format=raw,file={}", vars.display()));
    cmd.arg("-drive").arg(format!(
        "if=none,id=stick,format=raw,file={}",
        image.display()
    ));
    cmd.args([
        "-device",
        "qemu-xhci,id=xhci",
        "-device",
        "usb-storage,bus=xhci.0,drive=stick,bootindex=0",
    ]);
    if options.headless {
        cmd.args(["-display", "none"]);
    }
    if options.gdb {
        cmd.args(["-s", "-S"]);
    }
    Ok(cmd)
}

/// `cargo xtask run`: boot interactively, kernel log on stdout.
pub fn run(options: &Options) -> Result {
    let image = image::build(options)?;
    if options.gdb {
        println!("xtask: QEMU is paused; attach with `gdb -ex 'target remote :1234'`");
    }
    run_command(qemu_command(&image, options)?.args(["-debugcon", "stdio"]))
}

/// `cargo xtask test`: boot headless and wait for the kernel's success line.
pub fn test(options: &Options) -> Result {
    let image = image::build(options)?;
    let log = target_dir().join("test-debugcon.log");
    let _ = fs::remove_file(&log);

    let headless = Options {
        headless: true,
        gdb: false,
        ..*options
    };
    let mut child = qemu_command(&image, &headless)?
        .arg("-debugcon")
        .arg(format!("file:{}", log.display()))
        .stdin(Stdio::null())
        .spawn()?;

    let start = Instant::now();
    let outcome = loop {
        let output = fs::read_to_string(&log).unwrap_or_default();
        if output.contains(BOOT_OK) {
            break Ok(());
        }
        if output.contains(PANIC) {
            break Err("kernel panicked");
        }
        if child.try_wait()?.is_some() {
            break Err("QEMU exited before boot completed");
        }
        if start.elapsed() > TEST_TIMEOUT {
            break Err("timed out waiting for boot to complete");
        }
        thread::sleep(Duration::from_millis(200));
    };
    let _ = child.kill();
    let _ = child.wait();

    println!("----- kernel log -----");
    print!("{}", fs::read_to_string(&log).unwrap_or_default());
    println!("----------------------");
    match outcome {
        Ok(()) => {
            println!("xtask: boot test passed in {:.1?}", start.elapsed());
            Ok(())
        }
        Err(reason) => bail!("boot test failed: {reason}"),
    }
}
