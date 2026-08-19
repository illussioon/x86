use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{cursor, execute, terminal};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use x86::{Machine, Resource, RunOptions, X86Error, copy_resource_to};

const ANSI_CGA: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (0, 0, 170),
    (0, 170, 0),
    (0, 170, 170),
    (170, 0, 0),
    (170, 0, 170),
    (170, 85, 0),
    (170, 170, 170),
    (85, 85, 85),
    (85, 85, 255),
    (85, 255, 85),
    (85, 255, 255),
    (255, 85, 85),
    (255, 85, 255),
    (255, 255, 85),
    (255, 255, 255),
];

fn printable_guest_char(byte: u8) -> char {
    match byte {
        0x20..=0x7E => byte as char,
        _ => ' ',
    }
}

fn terminal_dimensions() -> (usize, usize) {
    let detected = terminal::size()
        .ok()
        .map(|(cols, rows)| (cols as usize, rows as usize));
    let (detected_cols, detected_rows) = detected.unwrap_or((80, 24));
    let cols = std::env::var("X86_TERM_COLS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(detected_cols)
        .clamp(40, 240);
    let rows = std::env::var("X86_TERM_ROWS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(detected_rows.saturating_sub(1).max(12))
        .clamp(12, 120);
    (cols, rows)
}

fn ansi_clear_home() -> &'static str {
    "\x1b[2J\x1b[H"
}

fn text_frame(machine: &Machine) -> Option<String> {
    let Some((cols, rows, bytes)) = machine.vga_text_snapshot() else {
        return None;
    };
    let cells = cols as usize * rows as usize;
    if bytes.len() < cells * 2 {
        return None;
    }
    let mut output = String::with_capacity(cells * 20 + rows as usize * 8);
    output.push_str(ansi_clear_home());
    for row in 0..rows as usize {
        for col in 0..cols as usize {
            let index = (row * cols as usize + col) * 2;
            let ch = printable_guest_char(bytes[index]);
            let attr = bytes[index + 1];
            let fg = ANSI_CGA[(attr & 0x0F) as usize];
            let bg = ANSI_CGA[((attr >> 4) & 0x07) as usize];
            output.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}",
                fg.0, fg.1, fg.2, bg.0, bg.1, bg.2, ch
            ));
        }
        output.push_str("\x1b[0m\r\n");
    }
    output.push_str("\x1b[0m");
    Some(output)
}

fn sample_rgb(
    pixels: &[u8],
    width: usize,
    height: usize,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
) -> (u8, u8, u8) {
    let x0 = x0.min(width.saturating_sub(1));
    let x1 = x1.max(x0 + 1).min(width);
    let y0 = y0.min(height.saturating_sub(1));
    let y1 = y1.max(y0 + 1).min(height);
    let mut sum = [0u64; 3];
    let mut count = 0u64;
    for y in y0..y1 {
        for x in x0..x1 {
            let offset = (y * width + x) * 3;
            if offset + 2 < pixels.len() {
                sum[0] += pixels[offset] as u64;
                sum[1] += pixels[offset + 1] as u64;
                sum[2] += pixels[offset + 2] as u64;
                count += 1;
            }
        }
    }
    if count == 0 {
        return (0, 0, 0);
    }
    (
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    )
}

fn luminance(rgb: (u8, u8, u8)) -> u16 {
    (rgb.0 as u16 * 54 + rgb.1 as u16 * 183 + rgb.2 as u16 * 19) / 256
}

fn braille_frame(machine: &Machine) -> Option<String> {
    let Some((width, height, pixels)) = machine.vga_framebuffer_rgb() else {
        return None;
    };
    let source_width = width as usize;
    let source_height = height as usize;
    if source_width == 0 || source_height == 0 || pixels.len() != source_width * source_height * 3 {
        return None;
    }
    let (out_cols, out_rows) = terminal_dimensions();
    let mut output = String::with_capacity(out_cols * out_rows * 30);
    output.push_str(ansi_clear_home());

    // Braille uses two dots by four rows per terminal cell. Unlike full-cell
    // averaging, this preserves the shape of the guest's text and avoids the
    // horizontal white bands produced by large background-colored blocks.
    const DOTS: [(usize, usize, u8); 8] = [
        (0, 0, 0x01),
        (0, 1, 0x02),
        (0, 2, 0x04),
        (1, 0, 0x08),
        (1, 1, 0x10),
        (1, 2, 0x20),
        (0, 3, 0x40),
        (1, 3, 0x80),
    ];
    for row in 0..out_rows {
        for col in 0..out_cols {
            let mut samples = [(0u8, 0u8, 0u8); 8];
            let mut values = [0u16; 8];
            for (index, (dot_x, dot_y, _)) in DOTS.iter().enumerate() {
                let x0 = (col * 2 + dot_x) * source_width / (out_cols * 2);
                let x1 = (col * 2 + dot_x + 1) * source_width / (out_cols * 2);
                let y0 = (row * 4 + dot_y) * source_height / (out_rows * 4);
                let y1 = (row * 4 + dot_y + 1) * source_height / (out_rows * 4);
                samples[index] = sample_rgb(&pixels, source_width, source_height, x0, x1, y0, y1);
                values[index] = luminance(samples[index]);
            }
            let min_value = *values.iter().min().unwrap_or(&0);
            let max_value = *values.iter().max().unwrap_or(&0);
            let threshold = 24u16.max(min_value + (max_value.saturating_sub(min_value) / 3));
            let mut pattern = 0u8;
            let mut color_sum = [0u32; 3];
            let mut active = 0u32;
            for (index, (_, _, bit)) in DOTS.iter().enumerate() {
                if values[index] >= threshold && values[index] > 24 {
                    pattern |= *bit;
                    color_sum[0] += samples[index].0 as u32;
                    color_sum[1] += samples[index].1 as u32;
                    color_sum[2] += samples[index].2 as u32;
                    active += 1;
                }
            }
            if pattern == 0 {
                output.push(' ');
            } else {
                let color = (
                    (color_sum[0] / active) as u8,
                    (color_sum[1] / active) as u8,
                    (color_sum[2] / active) as u8,
                );
                output.push_str(&format!(
                    "\x1b[38;2;{};{};{}m{}",
                    color.0,
                    color.1,
                    color.2,
                    char::from_u32(0x2800 + pattern as u32).unwrap_or(' ')
                ));
            }
        }
        output.push_str("\x1b[0m\r\n");
    }
    output.push_str("\x1b[0m");
    Some(output)
}

pub fn screen_frame(machine: &Machine) -> Option<String> {
    text_frame(machine).or_else(|| braille_frame(machine))
}

pub fn render_screen(machine: &Machine) {
    match screen_frame(machine) {
        Some(frame) => {
            print!("{frame}");
            let _ = io::stdout().flush();
        }
        None => println!("guest screen is not available yet; run the machine first"),
    }
}

fn run_slice(machine: &mut Machine, steps: u64) -> Result<(), X86Error> {
    let report = machine.run(RunOptions {
        max_steps: Some(steps.max(1)),
        ..Default::default()
    })?;
    println!(
        "run finished: {} steps, halted={}",
        report.steps, report.halted
    );
    render_screen(machine);
    Ok(())
}

pub fn run_background(
    machine: &mut Machine,
    steps: u64,
    seconds: Option<u64>,
) -> Result<(), X86Error> {
    let deadline = seconds.map(|value| Instant::now() + Duration::from_secs(value));
    let mut last_render = Instant::now() - Duration::from_secs(1);
    loop {
        if deadline.is_some_and(|value| Instant::now() >= value) {
            break;
        }
        machine.run(RunOptions {
            max_steps: Some(steps.max(1)),
            ..Default::default()
        })?;
        if last_render.elapsed() >= Duration::from_millis(100) {
            render_screen(machine);
            last_render = Instant::now();
        }
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn parse_u64(value: Option<&str>, label: &str) -> Result<u64, X86Error> {
    value
        .ok_or_else(|| X86Error::InvalidImage(format!("missing {label}")))?
        .parse::<u64>()
        .map_err(|error| X86Error::InvalidImage(format!("invalid {label}: {error}")))
}

fn spawn_host(program: &str, args: &[&str]) -> Result<Child, X86Error> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| X86Error::Io {
            path: PathBuf::from(program),
            source: error,
        })
}

fn make_executable(path: &Path) -> Result<(), X86Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|source| X86Error::Io {
                path: path.to_path_buf(),
                source,
            })?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(path, permissions).map_err(|source| X86Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

pub fn run_script(machine: &mut Machine, path: &Path) -> Result<(), X86Error> {
    let source = fs::read_to_string(path).map_err(|source| X86Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut children = Vec::new();
    for (line_number, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let command = parts.next().unwrap_or_default();
        let result = match command {
            "download" => {
                let url = parts.next().ok_or_else(|| {
                    X86Error::InvalidImage("usage: download <url> <path>".to_owned())
                })?;
                let destination = PathBuf::from(parts.next().ok_or_else(|| {
                    X86Error::InvalidImage("usage: download <url> <path>".to_owned())
                })?);
                let destination = if destination.is_absolute() {
                    destination
                } else {
                    base.join(destination)
                };
                copy_resource_to(&Resource::url(url), &destination)?;
                make_executable(&destination)?;
                println!("downloaded {url} -> {}", destination.display());
                Ok(())
            }
            "load" => match parts.next() {
                Some("state") => {
                    let state = parts.next().ok_or_else(|| {
                        X86Error::InvalidImage("usage: load state <path>".to_owned())
                    })?;
                    let state = if Path::new(state).is_absolute() {
                        PathBuf::from(state)
                    } else {
                        base.join(state)
                    };
                    machine.load_saved_state(Resource::file(state))?;
                    println!("saved state loaded");
                    Ok(())
                }
                _ => Err(X86Error::InvalidImage(
                    "script supports: load state <path>".to_owned(),
                )),
            },
            "exec" => {
                let program = parts.next().ok_or_else(|| {
                    X86Error::InvalidImage("usage: exec <program> [args...]".to_owned())
                })?;
                let program_path = if program.contains('/') || program.contains('\\') {
                    let path = Path::new(program);
                    if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        base.join(path)
                    }
                } else {
                    PathBuf::from(program)
                };
                let args = parts.collect::<Vec<_>>();
                let program_display = program_path.display().to_string();
                children.push(spawn_host(&program_path.to_string_lossy(), &args)?);
                println!("started host process in background: {program_display}");
                Ok(())
            }
            "run-state" => run_slice(machine, parse_u64(parts.next(), "steps")?),
            "background" => {
                let mut values = line.split_whitespace();
                let _ = values.next();
                let steps = values
                    .next()
                    .unwrap_or("25000")
                    .parse::<u64>()
                    .map_err(|error| X86Error::InvalidImage(format!("invalid steps: {error}")))?;
                let duration = values
                    .next()
                    .map(|value| {
                        value.parse::<u64>().map_err(|error| {
                            X86Error::InvalidImage(format!("invalid seconds: {error}"))
                        })
                    })
                    .transpose()?;
                run_background(machine, steps, duration)
            }
            "sleep" => {
                thread::sleep(Duration::from_secs(parse_u64(parts.next(), "seconds")?));
                Ok(())
            }
            "type" => {
                let text = parts.collect::<Vec<_>>().join(" ");
                let count = machine.inject_text(&format!("{text}\n"))?;
                println!("keyboard input queued: {count} characters");
                Ok(())
            }
            "screen" => {
                render_screen(machine);
                Ok(())
            }
            "wait" => {
                for child in &mut children {
                    let _ = child.wait();
                }
                children.clear();
                Ok(())
            }
            "quit" | "exit" | "stop" => break,
            _ => Err(X86Error::InvalidImage(format!(
                "unknown script command: {command}"
            ))),
        };
        if let Err(error) = result {
            return Err(X86Error::InvalidImage(format!(
                "script line {}: {error}",
                line_number + 1
            )));
        }
    }
    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self, X86Error> {
        #[cfg(feature = "native-runtime")]
        native_v86_core::native_runtime::set_uart_output_enabled(false);
        terminal::enable_raw_mode()
            .map_err(|error| X86Error::BackendUnavailable(error.to_string()))?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(terminal::ClearType::All)
        )
        .map_err(|error| X86Error::BackendUnavailable(error.to_string()))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        #[cfg(feature = "native-runtime")]
        native_v86_core::native_runtime::set_uart_output_enabled(true);
        let _ = terminal::disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = stdout.flush();
    }
}

fn key_to_guest_text(event: KeyEvent) -> Option<String> {
    match event.code {
        KeyCode::Char(_character) if event.modifiers.contains(KeyModifiers::CONTROL) => None,
        KeyCode::Char(character) => Some(character.to_string()),
        KeyCode::Enter => Some("\n".to_owned()),
        KeyCode::Backspace => Some("\x08".to_owned()),
        KeyCode::Tab => Some("\t".to_owned()),
        _ => None,
    }
}

pub fn run_interactive(machine: &mut Machine) -> Result<(), X86Error> {
    let _guard = TerminalGuard::enter()?;
    let mut last_frame = String::new();
    let mut last_render = Instant::now() - Duration::from_secs(1);
    loop {
        while event::poll(Duration::from_millis(0))
            .map_err(|error| X86Error::BackendUnavailable(error.to_string()))?
        {
            if let Event::Key(key) =
                event::read().map_err(|error| X86Error::BackendUnavailable(error.to_string()))?
            {
                if key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    return Ok(());
                }
                if let Some(text) = key_to_guest_text(key) {
                    machine.inject_text(&text)?;
                }
            }
        }
        machine.run(RunOptions {
            max_steps: Some(25_000),
            ..Default::default()
        })?;
        if last_render.elapsed() >= Duration::from_millis(75) {
            if let Some(frame) = screen_frame(machine) {
                if frame != last_frame {
                    print!("{frame}");
                    io::stdout().flush().map_err(|source| X86Error::Io {
                        path: PathBuf::from("stdout"),
                        source,
                    })?;
                    last_frame = frame;
                }
            }
            last_render = Instant::now();
        }
    }
}
