// SPDX-License-Identifier: MIT OR Apache-2.0

//! Boots the Horus image in QEMU, interactively or as an automated test.
//!
//! The virtual machine matches the laptop's boot path: UEFI firmware (OVMF),
//! and the image attached as USB storage on an xHCI controller.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, thread};

use crate::{Error, Options, Result, bail, image, run_command, target_dir};

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
/// How long the screen may take to show the expected result after the
/// kernel logged it.
const SCREEN_TIMEOUT: Duration = Duration::from_secs(10);

// Colors the kernel draws; keep in sync with `kernel/src/stage.rs`,
// `kernel/src/console.rs`, and `kernel/src/panic.rs`.
const STAGE_READY: u32 = 0xd4a63a;
const STAGE_PANIC: u32 = 0xc0392b;
const CONSOLE_BACKGROUND: u32 = 0x0f1115;
const CONSOLE_TEXT: u32 = 0xe6e6e6;
const PANIC_TEXT: u32 = 0xffffff;
// Console layout; keep in sync with `kernel/src/console.rs` and `font.rs`.
const CONSOLE_TEXT_TOP: usize = 14;
const CONSOLE_MARGIN: usize = 8;
const GLYPH_HEIGHT: usize = 16;

/// The laptop's internal panel (`docs/hardware.md`).
const LAPTOP_RESOLUTION: (usize, usize) = (1366, 768);
/// Small enough that the boot log is longer than the screen, so the boot
/// test exercises console scrolling.
const SCROLL_TEST_RESOLUTION: (usize, usize) = (1024, 600);

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

/// The QEMU command for `image`, without the debug console setting. The
/// UEFI firmware picks `resolution` for the framebuffer.
fn qemu_command(
    image: &Path,
    options: &Options,
    (width, height): (usize, usize),
) -> Result<Command> {
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
    cmd.args(["-vga", "none", "-device"])
        .arg(format!("VGA,xres={width},yres={height}"));
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
    run_command(qemu_command(&image, options, LAPTOP_RESOLUTION)?.args(["-debugcon", "stdio"]))
}

/// What a test boot should end in.
#[derive(Clone, Copy)]
enum Scenario {
    /// Normal boot: the log ends with [`BOOT_OK`] under a gold stage stripe.
    Boot,
    /// Kernel built with `panic-test`: the red panic screen shows.
    Panic,
}

impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Scenario::Boot => "boot",
            Scenario::Panic => "panic",
        }
    }

    fn resolution(self) -> (usize, usize) {
        match self {
            Scenario::Boot => SCROLL_TEST_RESOLUTION,
            Scenario::Panic => LAPTOP_RESOLUTION,
        }
    }

    /// The log line that ends this scenario, and the one that fails it.
    fn markers(self) -> (&'static str, &'static str) {
        match self {
            Scenario::Boot => (BOOT_OK, PANIC),
            Scenario::Panic => (PANIC, BOOT_OK),
        }
    }

    /// Checks a screenshot taken after the expected log line appeared.
    fn check_screen(self, screen: &Screen) -> std::result::Result<(), String> {
        let (w, h) = (screen.width, screen.height);
        match self {
            Scenario::Boot => {
                expect_pixel(screen, w / 2, 2, STAGE_READY, "stage stripe")?;
                expect_pixel(
                    screen,
                    w - 2,
                    h - 2,
                    CONSOLE_BACKGROUND,
                    "console background",
                )?;
                // The log is longer than the screen, so after the final
                // newline scrolls, the last line sits second from the bottom
                // and the bottom row is empty. Without a working scroll and
                // redraw, the bottom row would still hold text.
                let rows = (h - CONSOLE_TEXT_TOP - CONSOLE_MARGIN) / GLYPH_HEIGHT;
                let row_top = |row: usize| CONSOLE_TEXT_TOP + row * GLYPH_HEIGHT;
                if screen.count_rows(CONSOLE_TEXT, row_top(rows - 2), GLYPH_HEIGHT) == 0 {
                    return Err("console: last log line is not second from the bottom".into());
                }
                if screen.count_rows(CONSOLE_TEXT, row_top(rows - 1), GLYPH_HEIGHT) != 0 {
                    return Err("console: bottom row is not empty after scrolling".into());
                }
                Ok(())
            }
            Scenario::Panic => {
                expect_pixel(screen, w - 2, h - 2, STAGE_PANIC, "panic background")?;
                expect_some(screen, PANIC_TEXT, "panic text")
            }
        }
    }
}

fn expect_pixel(
    screen: &Screen,
    x: usize,
    y: usize,
    want: u32,
    what: &str,
) -> std::result::Result<(), String> {
    match screen.pixel(x, y) {
        got if got == want => Ok(()),
        got => Err(format!(
            "{what}: pixel ({x}, {y}) is {got:#08x}, expected {want:#08x}"
        )),
    }
}

fn expect_some(screen: &Screen, color: u32, what: &str) -> std::result::Result<(), String> {
    if screen.count(color) > 0 {
        Ok(())
    } else {
        Err(format!("{what}: no pixels of color {color:#08x}"))
    }
}

/// `cargo xtask test`: boot headless, once normally and once with a forced
/// panic, checking the kernel log and a screenshot each time.
pub fn test(options: &Options) -> Result {
    let start = Instant::now();
    for scenario in [Scenario::Boot, Scenario::Panic] {
        test_scenario(options, scenario)?;
    }
    println!("xtask: all boot tests passed in {:.1?}", start.elapsed());
    Ok(())
}

fn test_scenario(options: &Options, scenario: Scenario) -> Result {
    let name = scenario.name();
    let options = Options {
        headless: true,
        gdb: false,
        panic_test: matches!(scenario, Scenario::Panic),
        ..*options
    };
    let image = image::build(&options)?;
    let log = target_dir().join(format!("test-{name}.log"));
    let screenshot = target_dir().join(format!("test-{name}.ppm"));
    let socket = target_dir().join(format!("test-{name}.qmp"));
    let stderr = target_dir().join(format!("test-{name}.stderr"));
    for path in [&log, &screenshot, &socket, &stderr] {
        let _ = fs::remove_file(path);
    }

    let mut child = qemu_command(&image, &options, scenario.resolution())?
        .arg("-debugcon")
        .arg(format!("file:{}", log.display()))
        .arg("-qmp")
        .arg(format!("unix:{},server=on,wait=off", socket.display()))
        .stdin(Stdio::null())
        .stderr(File::create(&stderr)?)
        .spawn()?;

    let start = Instant::now();
    let outcome = wait_for_log(&mut child, &log, scenario, start)
        .and_then(|()| wait_for_screen(&socket, &screenshot, scenario));
    let _ = child.kill();
    let _ = child.wait();

    println!("----- {name}: kernel log -----");
    print!("{}", fs::read_to_string(&log).unwrap_or_default());
    println!("----- screenshot: {} -----", screenshot.display());
    match outcome {
        Ok(()) => {
            println!("xtask: {name} test passed in {:.1?}", start.elapsed());
            Ok(())
        }
        Err(reason) => {
            let qemu_errors = fs::read_to_string(&stderr).unwrap_or_default();
            if !qemu_errors.trim().is_empty() {
                println!("----- QEMU stderr -----\n{}", qemu_errors.trim_end());
            }
            bail!("{name} test failed: {reason}")
        }
    }
}

/// Waits until the kernel log shows the scenario's expected line.
fn wait_for_log(
    child: &mut Child,
    log: &Path,
    scenario: Scenario,
    start: Instant,
) -> std::result::Result<(), String> {
    let (expected, unexpected) = scenario.markers();
    loop {
        let output = fs::read_to_string(log).unwrap_or_default();
        if output.contains(expected) {
            return Ok(());
        }
        if output.contains(unexpected) {
            return Err(format!("log shows `{unexpected}`"));
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return Err(format!("QEMU exited early ({status})"));
        }
        if start.elapsed() > TEST_TIMEOUT {
            return Err(format!("timed out waiting for `{expected}`"));
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// Takes screenshots until the scenario's screen checks pass. The kernel
/// logs before it draws, so the screen may lag briefly behind the log.
fn wait_for_screen(
    socket: &Path,
    screenshot: &Path,
    scenario: Scenario,
) -> std::result::Result<(), String> {
    let mut qmp = Qmp::connect(socket).map_err(|e| format!("QMP: {e}"))?;
    let start = Instant::now();
    loop {
        qmp.screendump(screenshot)
            .map_err(|e| format!("screendump: {e}"))?;
        let screen = Screen::read_ppm(screenshot).map_err(|e| format!("screenshot: {e}"))?;
        match scenario.check_screen(&screen) {
            Ok(()) => return Ok(()),
            Err(problem) if start.elapsed() > SCREEN_TIMEOUT => return Err(problem),
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
}

/// A minimal client for the QEMU Machine Protocol.
struct Qmp {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Qmp {
    fn connect(socket: &Path) -> Result<Qmp> {
        let stream = UnixStream::connect(socket)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let mut qmp = Qmp {
            reader: BufReader::new(stream.try_clone()?),
            writer: stream,
        };
        let greeting = qmp.read_line()?;
        if !greeting.contains("\"QMP\"") {
            bail!("unexpected greeting: {greeting}");
        }
        qmp.execute(r#"{"execute": "qmp_capabilities"}"#)?;
        Ok(qmp)
    }

    fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            bail!("QEMU closed the connection");
        }
        Ok(line)
    }

    /// Sends a command and waits for its reply, skipping async events.
    fn execute(&mut self, command: &str) -> Result {
        writeln!(self.writer, "{command}")?;
        loop {
            let line = self.read_line()?;
            if line.contains("\"return\"") {
                return Ok(());
            }
            if line.contains("\"error\"") {
                bail!("{}", line.trim());
            }
        }
    }

    fn screendump(&mut self, path: &Path) -> Result {
        let path = path
            .to_str()
            .ok_or_else(|| Error("non-UTF-8 path".into()))?;
        if path.contains(['"', '\\']) {
            bail!("unsupported characters in path {path}");
        }
        self.execute(&format!(
            r#"{{"execute": "screendump", "arguments": {{"filename": "{path}", "format": "ppm"}}}}"#
        ))
    }
}

/// An RGB screenshot.
struct Screen {
    width: usize,
    height: usize,
    /// Three bytes per pixel, row by row.
    rgb: Vec<u8>,
}

impl Screen {
    /// Reads a binary PPM (P6, 8 bits per channel), as written by QEMU.
    fn read_ppm(path: &Path) -> Result<Screen> {
        let data = fs::read(path)?;
        // Header: "P6", width, height, maxval, each followed by whitespace.
        let mut fields = Vec::new();
        let mut pos = 0;
        while fields.len() < 4 {
            while data.get(pos).is_some_and(u8::is_ascii_whitespace) {
                pos += 1;
            }
            let start = pos;
            while data.get(pos).is_some_and(|b| !b.is_ascii_whitespace()) {
                pos += 1;
            }
            if start == pos {
                bail!("truncated PPM header");
            }
            fields.push(String::from_utf8_lossy(&data[start..pos]).into_owned());
        }
        let number = |s: &str| {
            s.parse::<usize>()
                .map_err(|_| Error(format!("bad PPM number {s}")))
        };
        if fields[0] != "P6" || fields[3] != "255" {
            bail!("unsupported PPM ({} with maxval {})", fields[0], fields[3]);
        }
        let (width, height) = (number(&fields[1])?, number(&fields[2])?);
        let rgb = data.get(pos + 1..).unwrap_or_default().to_vec();
        if rgb.len() < width * height * 3 {
            bail!("truncated PPM pixel data");
        }
        Ok(Screen { width, height, rgb })
    }

    fn pixel(&self, x: usize, y: usize) -> u32 {
        let i = (y * self.width + x) * 3;
        u32::from_be_bytes([0, self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]])
    }

    fn count(&self, color: u32) -> usize {
        self.count_rows(color, 0, self.height)
    }

    /// Counts pixels of `color` in `height` pixel rows starting at `top`.
    fn count_rows(&self, color: u32, top: usize, height: usize) -> usize {
        let bottom = (top + height).min(self.height);
        (top.min(bottom)..bottom)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&(x, y)| self.pixel(x, y) == color)
            .count()
    }
}
