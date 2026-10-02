mod bubble;
mod render;
mod tetoart;

use std::io::{IsTerminal, Read};

use clap::{Parser, ValueEnum};
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
    /// Text in the speech bubble (or stdin pipe)
    text: Option<String>,

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

    /// Remove styles from random pool
    #[arg(long, value_delimiter = ',')]
    disable: Vec<usize>,

    /// Return styles to random pool
    #[arg(long, value_delimiter = ',')]
    enable: Vec<usize>,

    /// Horizontal alignment
    #[arg(long, value_enum, default_value_t = AlignArg::Center)]
    align: AlignArg,
}

fn read_stdin() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn main() {
    let args = Args::parse();

    for n in args.disable.iter().chain(args.enable.iter()) {
        if *n >= art_count() {
            eprintln!(
                "Error: style {n} does not exist; choose a value between 0 and {}.",
                art_count() - 1
            );
            std::process::exit(1);
        }
    }
    let pool = effective_pool(&args.disable, &args.enable);

    if args.list {
        println!("Available Teto art styles: {}", art_count());
        println!("==========================");
        for i in 0..art_count() {
            let custom = if i >= BUILTIN_COUNT {
                user_art_name(i).map(|n| format!(" (custom: {n})"))
            } else {
                None
            };
            if pool.contains(&i) {
                println!("\n--- Style {i}{} ---", custom.unwrap_or_default());
            } else {
                println!("\n--- Style {i} (вне пула) ---");
            }
            for line in get_teto_art(Some(i)) {
                println!("{line}");
            }
        }
        println!(
            "\nUse --style <number> to select a specific style, or omit for random selection."
        );
        println!(
            "Custom arts: drop .txt files into {}.",
            user_arts_dir().display()
        );
        return;
    }

    let text = match args.text {
        Some(ref t) if t == "-" => match read_stdin() {
            Some(s) => s,
            None => {
                eprintln!("Error: no input on stdin.");
                std::process::exit(1);
            }
        },
        Some(t) => t,
        None => match read_stdin() {
            Some(s) => s,
            None => {
                eprintln!("Error: TEXT is required (or use --list). Try --help.");
                std::process::exit(1);
            }
        },
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
        print!("\x1b[2J\x1b[H");
    }
    print!(
        "{}",
        render_with_options(&text, style, width, args.align.into())
    );
}
