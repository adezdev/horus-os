// SPDX-License-Identifier: MIT OR Apache-2.0

//! Boots the Horus image in QEMU, interactively or as an automated test.
//!
//! The virtual machine matches the laptop's boot path: UEFI firmware (OVMF),
//! and the image attached as USB storage on an xHCI controller.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, thread};

use crate::font::{self, Font, UNREADABLE};
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
/// Printed by the kernel's panic handler, followed by `at <location>: `.
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
// Layout; keep in sync with `kernel/src/console.rs` and `kernel/src/panic.rs`.
const CONSOLE_TEXT_TOP: usize = 14;
const CONSOLE_MARGIN: usize = 8;
const PANIC_MARGIN: usize = 24;
/// Lines printed by the kernel's `console-test` feature.
const CONSOLE_TEST_LINES: usize = 100;

/// The laptop panel is 1366×768, but QEMU's standard VGA only displays
/// widths that are a multiple of 8. Asked for 1366, it shows 1360 while the
/// firmware still reports 1366, so every row is drawn at the wrong offset
/// and text shears diagonally. 1360 is the closest width it can show.
const LAPTOP_RESOLUTION: (usize, usize) = (1360, 768);
/// Smaller than the laptop, to test a second resolution.
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

/// A file that is deleted when dropped.
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// The QEMU command for `image`, without the debug console setting. The
/// UEFI firmware picks `resolution` for the framebuffer. Keep the returned
/// [`TempFile`] (the firmware variable store) alive until QEMU exits.
fn qemu_command(
    image: &Path,
    options: &Options,
    (width, height): (usize, usize),
) -> Result<(Command, TempFile)> {
    if width % 8 != 0 {
        bail!("QEMU's VGA needs a width that is a multiple of 8, not {width}");
    }
    let code = find_firmware("OVMF_CODE", OVMF_CODE_PATHS)?;
    // The firmware writes to its variable store (boot entries, display
    // settings), so every QEMU gets a fresh copy of the template. A copy per
    // invocation also keeps concurrent runs from sharing one.
    let vars = TempFile(target_dir().join(format!("ovmf-vars-{}.fd", process::id())));
    fs::copy(find_firmware("OVMF_VARS", OVMF_VARS_PATHS)?, &vars.0)?;

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
        .arg(format!("if=pflash,format=raw,file={}", vars.0.display()));
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
    Ok((cmd, vars))
}

/// `cargo xtask run`: boot interactively, kernel log on stdout.
pub fn run(options: &Options) -> Result {
    let image = image::build(options)?;
    if options.gdb {
        println!("xtask: QEMU is paused; attach with `gdb -ex 'target remote :1234'`");
    }
    let (mut cmd, _vars) = qemu_command(&image, options, LAPTOP_RESOLUTION)?;
    run_command(cmd.args(["-debugcon", "stdio"]))
}

/// What a test boot should end in.
#[derive(Clone, Copy)]
enum Scenario {
    /// The normal kernel: the console's last line is [`BOOT_OK`] under a
    /// gold stage stripe.
    Boot,
    /// Kernel with `console-test`: 100 numbered lines scroll through the
    /// console, and every visible row shows the expected line.
    Scroll,
    /// Kernel with `panic-test`: the red panic screen shows the message and
    /// location from the debug log.
    Panic,
}

impl Scenario {
    const ALL: [Scenario; 3] = [Scenario::Boot, Scenario::Scroll, Scenario::Panic];

    fn name(self) -> &'static str {
        match self {
            Scenario::Boot => "boot",
            Scenario::Scroll => "scroll",
            Scenario::Panic => "panic",
        }
    }

    fn kernel_features(self) -> &'static [&'static str] {
        match self {
            Scenario::Boot => &[],
            Scenario::Scroll => &["console-test"],
            Scenario::Panic => &["panic-test"],
        }
    }

    fn resolution(self) -> (usize, usize) {
        match self {
            Scenario::Boot | Scenario::Panic => LAPTOP_RESOLUTION,
            Scenario::Scroll => SCROLL_TEST_RESOLUTION,
        }
    }

    /// The log line that ends this scenario, and the one that fails it.
    fn markers(self) -> (&'static str, &'static str) {
        match self {
            Scenario::Boot | Scenario::Scroll => (BOOT_OK, PANIC),
            Scenario::Panic => (PANIC, BOOT_OK),
        }
    }

    /// Checks a screenshot taken after the expected log line appeared.
    fn check_screen(self, screen: &Screen, font: &Font, log: &str) -> CheckResult {
        let (w, h) = (screen.width, screen.height);
        match self {
            Scenario::Boot => {
                expect_pixel(screen, w / 2, 2, STAGE_READY, "stage stripe")?;
                let lines = console_lines(screen, font)?;
                expect_last_line(&lines, BOOT_OK)
            }
            Scenario::Scroll => {
                expect_pixel(screen, w / 2, 2, STAGE_READY, "stage stripe")?;
                let lines = console_lines(screen, font)?;
                // The final newline scrolls once more, so the last line sits
                // second from the bottom and the bottom row is empty.
                let rows = lines.len();
                if !lines[rows - 1].is_empty() {
                    return Err(format!("bottom row should be empty: {:?}", lines[rows - 1]));
                }
                expect_line(&lines, rows - 2, BOOT_OK)?;
                for (k, row) in (0..rows - 2).rev().enumerate() {
                    let n = CONSOLE_TEST_LINES - 1 - k;
                    expect_line(&lines, row, &format!("console test line {n:03}"))?;
                }
                Ok(())
            }
            Scenario::Panic => {
                expect_pixel(screen, w - 2, h - 2, STAGE_PANIC, "panic background")?;
                let (location, message) = panic_from_log(log)?;
                let expected = [
                    "HORUS KERNEL PANIC".to_string(),
                    String::new(),
                    message,
                    String::new(),
                    format!("at {location}"),
                    String::new(),
                    format!("kernel {}", env!("CARGO_PKG_VERSION")),
                    String::new(),
                    "The system has stopped. Take a photo of this screen, then reboot.".into(),
                ];
                let cols = (w - 2 * PANIC_MARGIN) / font::WIDTH;
                for (i, want) in expected.iter().enumerate() {
                    let y = PANIC_MARGIN + i * font::HEIGHT;
                    let got = font.read(
                        |x, y| screen.pixel(x, y),
                        (PANIC_MARGIN, y),
                        cols,
                        (PANIC_TEXT, STAGE_PANIC),
                    );
                    if &got != want {
                        return Err(format!("panic screen line {i}: {got:?}, expected {want:?}"));
                    }
                }
                Ok(())
            }
        }
    }
}

type CheckResult = std::result::Result<(), String>;

fn expect_pixel(screen: &Screen, x: usize, y: usize, want: u32, what: &str) -> CheckResult {
    match screen.pixel(x, y) {
        got if got == want => Ok(()),
        got => Err(format!(
            "{what}: pixel ({x}, {y}) is {got:#08x}, expected {want:#08x}"
        )),
    }
}

/// Reads every console row. Fails if any row has pixels that aren't
/// console text, e.g. because rows are drawn at the wrong offset.
fn console_lines(screen: &Screen, font: &Font) -> std::result::Result<Vec<String>, String> {
    let rows = (screen.height - CONSOLE_TEXT_TOP - CONSOLE_MARGIN) / font::HEIGHT;
    let cols = (screen.width - 2 * CONSOLE_MARGIN) / font::WIDTH;
    let lines: Vec<String> = (0..rows)
        .map(|row| {
            font.read(
                |x, y| screen.pixel(x, y),
                (CONSOLE_MARGIN, CONSOLE_TEXT_TOP + row * font::HEIGHT),
                cols,
                (CONSOLE_TEXT, CONSOLE_BACKGROUND),
            )
        })
        .collect();
    match lines.iter().position(|line| line.contains(UNREADABLE)) {
        Some(row) => Err(format!("console row {row} is unreadable: {:?}", lines[row])),
        None => Ok(lines),
    }
}

fn expect_line(lines: &[String], row: usize, want: &str) -> CheckResult {
    match &lines[row] {
        got if got == want => Ok(()),
        got => Err(format!("console row {row}: {got:?}, expected {want:?}")),
    }
}

/// Checks that the last non-empty console row reads `want`.
fn expect_last_line(lines: &[String], want: &str) -> CheckResult {
    match lines.iter().rposition(|line| !line.is_empty()) {
        Some(row) => expect_line(lines, row, want),
        None => Err("console is empty".into()),
    }
}

/// The location and message of the panic in the debug log.
fn panic_from_log(log: &str) -> std::result::Result<(String, String), String> {
    let prefix = format!("{PANIC} at ");
    log.lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|rest| rest.split_once(": "))
        .map(|(location, message)| (location.to_string(), message.trim_end().to_string()))
        .ok_or_else(|| "no panic location in the debug log".into())
}

/// `cargo xtask test`: boot headless in each [`Scenario`], checking the
/// kernel log and the text on screen each time.
pub fn test(options: &Options) -> Result {
    let font = Font::load()?;
    let start = Instant::now();
    for scenario in Scenario::ALL {
        test_scenario(options, scenario, &font)?;
    }
    println!("xtask: all boot tests passed in {:.1?}", start.elapsed());
    Ok(())
}

fn test_scenario(options: &Options, scenario: Scenario, font: &Font) -> Result {
    let name = scenario.name();
    let options = Options {
        headless: true,
        gdb: false,
        kernel_features: scenario.kernel_features(),
        ..*options
    };
    let image = image::build(&options)?;
    let log = target_dir().join(format!("test-{name}.log"));
    let screenshot = target_dir().join(format!("test-{name}.ppm"));
    let stderr = target_dir().join(format!("test-{name}.stderr"));
    let socket = TempFile(target_dir().join(format!("qmp-{}.sock", process::id())));
    for path in [&log, &screenshot, &stderr, &socket.0] {
        let _ = fs::remove_file(path);
    }

    let (mut cmd, _vars) = qemu_command(&image, &options, scenario.resolution())?;
    let mut child = cmd
        .arg("-debugcon")
        .arg(format!("file:{}", log.display()))
        .arg("-qmp")
        .arg(format!("unix:{},server=on,wait=off", socket.0.display()))
        .stdin(Stdio::null())
        .stderr(File::create(&stderr)?)
        .spawn()?;

    let start = Instant::now();
    let outcome = wait_for_log(&mut child, &log, scenario, start).and_then(|()| {
        let log = fs::read_to_string(&log).unwrap_or_default();
        wait_for_screen(&socket.0, &screenshot, |screen| {
            scenario.check_screen(screen, font, &log)
        })
    });
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
fn wait_for_log(child: &mut Child, log: &Path, scenario: Scenario, start: Instant) -> CheckResult {
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

/// Takes screenshots until `check` passes. The kernel logs before it draws,
/// so the screen may lag briefly behind the log.
fn wait_for_screen(
    socket: &Path,
    screenshot: &Path,
    check: impl Fn(&Screen) -> CheckResult,
) -> CheckResult {
    let mut qmp = Qmp::connect(socket).map_err(|e| format!("QMP: {e}"))?;
    let start = Instant::now();
    loop {
        qmp.screendump(screenshot)
            .map_err(|e| format!("screendump: {e}"))?;
        let screen = Screen::read_ppm(screenshot).map_err(|e| format!("screenshot: {e}"))?;
        match check(&screen) {
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

    /// The pixel at `(x, y)` as `0xRRGGBB`; black outside the screen.
    fn pixel(&self, x: usize, y: usize) -> u32 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        let i = (y * self.width + x) * 3;
        u32::from_be_bytes([0, self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]])
    }
}
