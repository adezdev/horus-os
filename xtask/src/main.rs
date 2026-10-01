// SPDX-License-Identifier: MIT OR Apache-2.0

//! Build orchestration for Horus: `cargo xtask <command>`.
//!
//! See `docs/development/setup.md` for the full command list.

mod image;
mod qemu;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fmt};

const KERNEL_TARGET: &str = "x86_64-unknown-none";

const USAGE: &str = "\
usage: cargo xtask <command> [options]

commands:
  build    build the kernel
  image    build target/horus.img (GPT + FAT32 ESP + Limine + kernel)
  run      boot the image in QEMU
  test     boot the image in headless QEMU and check the kernel log

options:
  --release   optimized kernel build (build, image, run, test)
  --gdb       start paused with a GDB stub on :1234 (run)
  --no-kvm    use TCG emulation instead of KVM (run, test)
  --headless  no QEMU window (run)

environment:
  LIMINE_DIR  Limine binaries (default /usr/share/limine)
  OVMF_CODE   UEFI firmware code image (searched in common locations)
  OVMF_VARS   UEFI variable store template (searched in common locations)";

/// Error type for all tasks: a message for the user.
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error(err.to_string())
    }
}

pub type Result<T = ()> = std::result::Result<T, Error>;

/// Returns an [`Error`] with a formatted message.
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::Error(format!($($arg)*)))
    };
}
pub(crate) use bail;

/// Command-line options shared by the tasks.
#[derive(Default)]
pub struct Options {
    pub release: bool,
    pub gdb: bool,
    pub no_kvm: bool,
    pub headless: bool,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next();
    let mut options = Options::default();
    for arg in args {
        match arg.as_str() {
            "--release" => options.release = true,
            "--gdb" => options.gdb = true,
            "--no-kvm" => options.no_kvm = true,
            "--headless" => options.headless = true,
            _ => return usage_error(&format!("unknown option `{arg}`")),
        }
    }

    let result = match command.as_deref() {
        Some("build") => build_kernel(&options).map(drop),
        Some("image") => image::build(&options).map(drop),
        Some("run") => qemu::run(&options),
        Some("test") => qemu::test(&options),
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => return usage_error(&format!("unknown command `{other}`")),
        None => return usage_error("missing command"),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask: error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("xtask: {message}\n\n{USAGE}");
    ExitCode::FAILURE
}

/// The workspace root directory.
pub fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
}

/// `target/` in the workspace.
pub fn target_dir() -> PathBuf {
    root().join("target")
}

/// A `cargo` command run from the workspace root.
fn cargo() -> Command {
    let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.current_dir(root());
    cmd
}

/// Runs `cmd` and fails if it doesn't exit successfully.
pub fn run_command(cmd: &mut Command) -> Result {
    let program = cmd.get_program().to_string_lossy().into_owned();
    let status = cmd
        .status()
        .map_err(|err| Error(format!("failed to start `{program}`: {err}")))?;
    if !status.success() {
        bail!("`{program}` failed ({status})");
    }
    Ok(())
}

/// Builds the kernel and returns the path of its ELF file.
pub fn build_kernel(options: &Options) -> Result<PathBuf> {
    let mut cmd = cargo();
    cmd.args([
        "build",
        "--package",
        "horus-kernel",
        "--target",
        KERNEL_TARGET,
    ]);
    if options.release {
        cmd.arg("--release");
    }
    run_command(&mut cmd)?;
    let profile = if options.release { "release" } else { "debug" };
    Ok(target_dir()
        .join(KERNEL_TARGET)
        .join(profile)
        .join("horus-kernel"))
}
