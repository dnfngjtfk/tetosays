mod bubble;
mod render;
mod tetoart;

use std::io::{IsTerminal, Read};

use clap::{Parser, ValueEnum};
use bubble::BubbleStyle;
use render::{render_with_options, Align};
use tetoart::{
    art_count, effective_pool, get_teto_art, pick_from_pool, user_art_name, user_arts_dir,
    BUILTIN_COUNT,
};

#[derive(Clone, Copy, ValueEnum, Debug, Default)]
enum AlignArg {
    /// Center of the terminal.
    #[default]
    Center,
    /// Left edge.
    Left,
    /// Right edge.
    Right,
}
impl From<AlignArg> for Align {
    fn from(value: AlignArg) -> Self {
        match value {
            AlignArg::Center => Align::Center,
            AlignArg::Left => Align::Left,
            AlignArg::Right => Align::Right,
        }
    }
}

/// A cowsay clone with Kasane Teto ASCII art and speech bubbles.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Text in the speech bubble (or stdin pipe). Multiple words are joined with spaces.
    text: Vec<String>,

    /// Art style by number (see --list), random from pool if omitted
    #[arg(short, long)]
    style: Option<usize>,

    /// List all art styles
    #[arg(short, long)]
    list: bool,

    /// Max bubble width (10-200)
    #[arg(short, long)]
    width: Option<usize>,

    /// Do not clear the screen
    #[arg(long)]
    no_clear: bool,

    /// Remove styles from random pool (comma-separated or repeated: --disable 1,2)
    #[arg(long, value_delimiter = ',')]
    disable: Vec<usize>,

    /// Return styles to random pool (comma-separated or repeated: --enable 6)
    #[arg(long, value_delimiter = ',')]
    enable: Vec<usize>,

    /// Horizontal alignment
    #[arg(long, value_enum, default_value_t = AlignArg::Center)]
    align: AlignArg,

    /// Bubble frame style
    #[arg(short = 'b', long = "bubble", value_enum, default_value_t = BubbleStyle::Round)]
    bubble: BubbleStyle,

    /// Print text without any bubble frame (overrides --bubble)
    #[arg(short = 'B', long = "no-bubble")]
    no_bubble: bool,
}

fn read_stdin() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).ok()?;
    // Use lossy conversion so non-UTF-8 input still renders
    // instead of masquerading as "no input".
    let buf = String::from_utf8_lossy(&bytes).into_owned();
    // Preserve interior newlines for multiline bubbles; only strip the
    // leading/trailing blank edges. Normalize \r\n to \n.
    let normalized = buf.replace("\r\n", "\n");
    let trimmed = normalized.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn write_stdout(s: &str) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    // println!/print! panic on EPIPE (e.g. `tetosays -l | head`);
    // exit quietly instead so pipes like `| head` or `| less` work.
    if let Err(e) = out.write_all(s.as_bytes()) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("Error writing to stdout: {e}");
        std::process::exit(1);
    }
}

fn print_line(s: &str) {
    let mut owned = String::with_capacity(s.len() + 1);
    owned.push_str(s);
    owned.push('\n');
    write_stdout(&owned);
}

fn main() {
    let args = Args::parse();

    let total = art_count();
    for n in args.disable.iter().chain(args.enable.iter()) {
        if *n >= total {
            eprintln!(
                "Error: style {n} does not exist; choose a value between 0 and {}.",
                total - 1
            );
            std::process::exit(1);
        }
    }
    let pool = effective_pool(&args.disable, &args.enable);

    if args.list {
        print_line(&format!("Available Teto art styles: {}", art_count()));
        print_line("==========================");
        for i in 0..art_count() {
            let custom = if i >= BUILTIN_COUNT {
                user_art_name(i).map(|n| format!(" (custom: {n})"))
            } else {
                None
            };
            let custom = custom.unwrap_or_default();
            if pool.contains(&i) {
                print_line(&format!("\n--- Style {i}{custom} ---"));
            } else {
                print_line(&format!("\n--- Style {i} (out of pool){custom} ---"));
            }
            for line in get_teto_art(Some(i)) {
                print_line(&line);
            }
        }
        print_line(
            "\nUse --style <number> to select a specific style, or omit for random selection.",
        );
        print_line(&format!(
            "Custom arts: drop .txt files into {}.",
            user_arts_dir().display()
        ));
        return;
    }

    let text = {
        let joined = args.text.join(" ");
        let trimmed = joined.trim();
        if trimmed.is_empty() || trimmed == "-" {
            match read_stdin() {
                Some(s) => s,
                None => {
                    if trimmed == "-" {
                        eprintln!("Error: no input on stdin.");
                    } else {
                        eprintln!("Error: TEXT is required (or use --list). Try --help.");
                    }
                    std::process::exit(1);
                }
            }
        } else {
            trimmed.to_string()
        }
    };

    if let Some(s) = args.style {
        if s >= art_count() {
            eprintln!(
                "Error: style {s} does not exist; choose a value between 0 and {}.",
                art_count() - 1
            );
            std::process::exit(1);
        }
    }

    let width = args.width.unwrap_or(crate::bubble::MAX_BUBBLE_WIDTH);
    if !(10..=200).contains(&width) {
        eprintln!("Error: --width must be between 10 and 200.");
        std::process::exit(1);
    }

    let style = match args.style {
        Some(s) => Some(s),
        None => match pick_from_pool(&pool) {
            Some(s) => Some(s),
            None => {
                eprintln!("Error: pool is empty (all styles disabled).");
                std::process::exit(1);
            }
        },
    };

    if !args.no_clear && std::io::stdout().is_terminal() {
        write_stdout("\x1b[2J\x1b[H");
    }
    write_stdout(&render_with_options(
        &text,
        style,
        width,
        args.align.into(),
        args.bubble,
        args.no_bubble,
    ));
}
