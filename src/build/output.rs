use std::env;
use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::process::Output;

const STATUS_WIDTH: usize = 12;

pub fn status(label: &str, message: impl Display) {
    print_status(io::stdout(), label, message);
}

pub fn print_error(error: &anyhow::Error) {
    let mut stderr = io::stderr();
    let _ = writeln!(stderr, "{} {}", style("error:", Color::Red, true), error);

    let mut causes = error.chain().skip(1).peekable();
    if causes.peek().is_none() {
        return;
    }

    let _ = writeln!(stderr);
    let _ = writeln!(stderr, "Caused by:");
    for cause in causes {
        let _ = writeln!(stderr, "  {cause}");
    }
}

pub fn print_command_output(output: &Output) -> io::Result<()> {
    let mut stderr = io::stderr();
    if !output.stderr.is_empty() {
        stderr.write_all(&output.stderr)?;
        if !output.stderr.ends_with(b"\n") {
            stderr.write_all(b"\n")?;
        }
    }

    if !output.stdout.is_empty() {
        stderr.write_all(&output.stdout)?;
        if !output.stdout.ends_with(b"\n") {
            stderr.write_all(b"\n")?;
        }
    }

    stderr.flush()
}

fn print_status(mut stream: impl Write, label: &str, message: impl Display) {
    let padding = " ".repeat(STATUS_WIDTH.saturating_sub(label.len()));
    let _ = writeln!(
        stream,
        "{padding}{} {}",
        style(label, Color::Green, true),
        message,
    );
}

fn style(text: &str, color: Color, bold: bool) -> String {
    if !color_enabled() {
        return text.to_string();
    }

    let color_code = match color {
        Color::Green => 32,
        Color::Red => 31,
    };
    let weight = if bold { "1;" } else { "" };
    format!("\u{1b}[{weight}{color_code}m{text}\u{1b}[0m")
}

fn color_enabled() -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }

    if matches!(env::var("CLICOLOR_FORCE").as_deref(), Ok("1")) {
        return true;
    }

    if matches!(env::var("CLICOLOR").as_deref(), Ok("0")) {
        return false;
    }

    if matches!(env::var("TERM").as_deref(), Ok("dumb")) {
        return false;
    }

    io::stdout().is_terminal() || io::stderr().is_terminal()
}

#[derive(Clone, Copy)]
enum Color {
    Green,
    Red,
}
