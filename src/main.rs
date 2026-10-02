mod bubble;
mod kasem;
mod render;
mod tetoart;

use std::io::{IsTerminal, Read};

use clap::{Parser, ValueEnum};
use bubble::BubbleStyle;
use render::{fits_term, render_with_options, render_with_term, term_size, Align, RenderOptions};
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

    /// Bubble width (10-200, exact) or auto-fit to text (default: auto)
    #[arg(short, long, value_parser = parse_width)]
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

    /// Keep spacing/newlines as-is instead of word wrap (for figlet, tables)
    #[arg(short = 'n', long = "no-wrap")]
    no_wrap: bool,

    /// Mirror the art horizontally
    #[arg(short = 'F', long = "flip")]
    flip: bool,

    /// Bold output (-b is taken by --bubble, so long flag only)
    #[arg(long = "bold")]
    bold: bool,

    /// Rainbow-gradient art (respects NO_COLOR)
    #[arg(short = 'r', long = "rainbow")]
    rainbow: bool,

    /// Render the text with every style in the pool, one after another
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Animate the rainbow gradient (optional seconds, 0 = until Ctrl-C; implies --rainbow; skipped if output does not fit the terminal)
    #[arg(short = 'A', long = "animate", num_args = 0..=1, default_missing_value = "5")]
    animate: Option<u64>,

    /// Kasem tetum instead of TEXT (optional word count, default 30)
    #[arg(short = 'k', short_aliases = ['L'], long = "kasem", alias = "lorem", num_args = 0..=1, default_missing_value = "30")]
    kasem: Option<usize>,

    /// Lines reserved at bottom for prompt/status line (overrides TETOSAYS_RESERVE_ROWS, default: 3)
    #[arg(long)]
    reserve: Option<usize>,
}

#[allow(dead_code)]
const TETO: &str = "
╔╦╗ ╔═  ╔╦╗ ╔═╗
 ║  ╠═   ║  ║ ║
 ╩  ╚═   ╩  ╚═╝";

fn parse_width(s: &str) -> Result<usize, String> {
    if s.eq_ignore_ascii_case("auto") {
        return Ok(0);
    }
    let v: usize = s
        .parse()
        .map_err(|_| format!("invalid width '{s}': expected 10-200 or auto"))?;
    if !(10..=200).contains(&v) {
        return Err(format!("invalid width '{v}': expected 10-200 or auto"));
    }
    Ok(v)
}

fn read_stdin() -> Option<String> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).ok()?;
    let buf = String::from_utf8_lossy(&bytes).into_owned();
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
    if let Err(e) = out.write_all(s.as_bytes()) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("Error writing to stdout: {e}");
        std::process::exit(1);
    }
}

fn resolve_reserve(flag: Option<usize>) -> usize {
    if let Some(n) = flag {
        return n;
    }
    if let Some(v) = std::env::var_os("TETOSAYS_RESERVE_ROWS") {
        if let Some(s) = v.to_str() {
            if let Ok(n) = s.trim().parse::<usize>() {
                return n;
            }
        }
    }
    3
}

fn sanitize_text(s: &str) -> String {
    let newlines = s.replace("\r\n", "\n").replace('\r', "\n");
    let visible = crate::render::strip_ansi(&newlines);
    visible
        .chars()
        .filter(|c| {
            if *c == '\n' || *c == '\t' {
                return true;
            }
            if c.is_control() {
                return false;
            }
            if matches!(c, '\u{AD}' | '\u{200B}' | '\u{FEFF}' | '\u{2060}') {
                return false;
            }
            true
        })
        .collect()
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

    let text = if let Some(n) = args.kasem {
        kasem::generate(n.clamp(1, 500))
    } else {
        let joined = args.text.join(" ");
        let trimmed = joined.trim();
        if trimmed.is_empty() || trimmed == "-" {
            match read_stdin() {
                Some(s) => s,
                None => {
                    if trimmed == "-" {
                        eprintln!("Error: no input on stdin.");
                        std::process::exit(1);
                    }
                    kasem::generate_default()
                }
            }
        } else {
            trimmed.to_string()
        }
    };
    let text = sanitize_text(&text);

    if let Some(s) = args.style {
        if s >= art_count() {
            eprintln!(
                "Error: style {s} does not exist; choose a value between 0 and {}.",
                art_count() - 1
            );
            std::process::exit(1);
        }
    }

    let (width, auto_width) = match args.width {
        None | Some(0) => (usize::MAX, true),
        Some(w) => (w, false),
    };

    let opts = RenderOptions {
        width,
        auto_width,
        align: args.align.into(),
        bubble: args.bubble,
        no_bubble: args.no_bubble,
        no_wrap: args.no_wrap,
        flip: args.flip,
        bold: args.bold,
        rainbow: args.rainbow || args.animate.is_some(),
        hue_offset: 0.0,
        reserve: resolve_reserve(args.reserve),
    };

    if args.all {
        if pool.is_empty() {
            eprintln!("Error: pool is empty (all styles disabled).");
            std::process::exit(1);
        }
        for i in &pool {
            print_line(&format!("--- Style {i} ---"));
            write_stdout(&render_with_term(&text, Some(*i), &opts, (80, 24), false));
        }
        return;
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

    if let Some(secs) = args.animate {
        if std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() {
            animate(&text, style, &opts, secs, args.no_clear);
            return;
        }
    }
    if !args.no_clear && std::io::stdout().is_terminal() {
        write_stdout("\x1b[3J\x1b[2J\x1b[H");
    }
    write_stdout(&render_with_options(&text, style, &opts));
}

fn animate(
    text: &str,
    style: Option<usize>,
    opts: &RenderOptions,
    secs: u64,
    no_clear: bool,
) {
    if !no_clear {
        write_stdout("\x1b[3J\x1b[2J\x1b[H");
    }
    let mut o = *opts;
    o.rainbow = true;
    o.hue_offset = 0.0;
    let first = render_with_options(text, style, &o);
    if !fits_term(&first, term_size(), opts.reserve) {
        write_stdout(&first);
        return;
    }
    let start = std::time::Instant::now();
    let limit = std::time::Duration::from_secs(secs);
    let mut frame: u32 = 0;
    loop {
        if secs > 0 && start.elapsed() >= limit {
            break;
        }
        let mut o = *opts;
        o.rainbow = true;
        o.hue_offset = frame as f32 * 0.04;
        if frame == 0 {
            write_stdout(&render_with_options(text, style, &o));
        } else {
            let mut redrawn = String::from("\x1b[H");
            redrawn.push_str(&render_with_options(text, style, &o));
            write_stdout(&redrawn);
        }
        std::thread::sleep(std::time::Duration::from_millis(83));
        frame = frame.wrapping_add(1);
    }
}


#[cfg(test)]
mod tests {
    use super::resolve_reserve;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn reserve_flag_beats_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TETOSAYS_RESERVE_ROWS", "7");
        assert_eq!(resolve_reserve(Some(3)), 3);
        std::env::remove_var("TETOSAYS_RESERVE_ROWS");
    }

    #[test]
    fn reserve_env_beats_default() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("TETOSAYS_RESERVE_ROWS", "4");
        assert_eq!(resolve_reserve(None), 4);
        std::env::remove_var("TETOSAYS_RESERVE_ROWS");
    }

    #[test]
    fn reserve_default_and_garbage() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("TETOSAYS_RESERVE_ROWS");
        assert_eq!(resolve_reserve(None), 3);
        std::env::set_var("TETOSAYS_RESERVE_ROWS", "nope");
        assert_eq!(resolve_reserve(None), 3);
        std::env::remove_var("TETOSAYS_RESERVE_ROWS");
    }
}

#[cfg(test)]
mod sanitize_tests {
    use super::sanitize_text;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn strips_ansi() {
        assert_eq!(sanitize_text("a\x1b[31mb\x1b[0mc"), "abc");
    }

    #[test]
    fn normalizes_carriage_returns() {
        assert_eq!(sanitize_text("a\r\nb"), "a\nb");
        assert_eq!(sanitize_text("a\rb"), "a\nb");
    }

    #[test]
    fn strips_controls_keeps_tab() {
        assert_eq!(sanitize_text("a\x07b\tc"), "ab\tc");
    }

    #[test]
    fn strips_invisible_format_chars() {
        assert_eq!(sanitize_text("a\u{AD}b\u{200B}c\u{FEFF}d\u{2060}e"), "abcde");
    }

    #[test]
    fn keeps_emoji_and_zwj() {
        assert_eq!(sanitize_text("a👨\u{200D}👩b"), "a👨\u{200D}👩b");
    }

    #[test]
    fn bubble_stays_aligned_on_dirty_input() {
        let dirty = "Laborum qui vocaloid,\x1b[31m commodo\u{AD} cupidatat.";
        let clean = sanitize_text(dirty);
        let bubble = crate::bubble::get_speech_bubble_lines(
            &clean,
            50,
            crate::bubble::BubbleStyle::Ascii,
            false,
            false,
            None,
        );
        let w = bubble[0].width();
        for line in &bubble[..bubble.len() - 2] {
            assert_eq!(line.width(), w);
        }
    }
}
