use std::io::IsTerminal;
use terminal_size::{terminal_size, Height, Width};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::bubble::{get_plain_text_lines, get_speech_bubble_lines, tail_count, BubbleStyle};
use crate::tetoart::{get_teto_art, is_blank};

pub(crate) fn term_size() -> (usize, usize) {
    let (w, h) = terminal_size()
        .map(|(Width(w), Height(h))| (w as usize, h as usize))
        .unwrap_or((80, 24));
    (if w > 0 { w } else { 80 }, if h > 0 { h } else { 24 })
}

pub(crate) fn fits_term(body: &str, term: (usize, usize), reserve: usize) -> bool {
    let (w, h) = term;
    let mut lines = 0usize;
    for line in body.lines() {
        if visible_width(line) > w {
            return false;
        }
        lines += 1;
    }
    lines <= h.saturating_sub(reserve)
}

pub(crate) fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for esc in chars.by_ref() {
                    if ('@'..='~').contains(&esc) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn visible_width(s: &str) -> usize {
    strip_ansi(s).width()
}

fn ink_center(art: &[String]) -> usize {
    let mut min_start = usize::MAX;
    let mut max_end = 0;
    let mut found = false;
    for line in art {
        let visible = strip_ansi(line);
        let mut col = 0;
        let mut start = None;
        let mut end = 0;
        for c in visible.chars() {
            let w = c.width().unwrap_or(0);
            if !is_blank(c) {
                if start.is_none() {
                    start = Some(col);
                }
                end = col + w;
            }
            col += w;
        }
        if let Some(s) = start {
            found = true;
            min_start = min_start.min(s);
            max_end = max_end.max(end);
        }
    }
    if !found {
        return 0;
    }
    (min_start + max_end) / 2
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Align {
    #[default]
    Center,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub width: usize,
    pub auto_width: bool,
    pub align: Align,
    pub bubble: BubbleStyle,
    pub no_bubble: bool,
    pub no_wrap: bool,
    pub flip: bool,
    pub bold: bool,
    pub rainbow: bool,
    pub hue_offset: f32,
    pub reserve: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: crate::bubble::MAX_BUBBLE_WIDTH,
            auto_width: true,
            align: Align::Center,
            bubble: BubbleStyle::Round,
            no_bubble: false,
            no_wrap: false,
            flip: false,
            bold: false,
            rainbow: false,
            hue_offset: 0.0,
            reserve: 3,
        }
    }
}

fn mirror_char(c: char) -> char {
    if ('\u{2800}'..='\u{28FF}').contains(&c) {
        let bits = c as u32 - 0x2800;
        let bit = |n: u32| (bits >> n) & 1;
        let mirrored = bit(3) | bit(4) << 1 | bit(5) << 2 | bit(0) << 3
            | bit(1) << 4 | bit(2) << 5 | bit(7) << 6 | bit(6) << 7;
        return char::from_u32(0x2800 + mirrored).unwrap_or(c);
    }
    match c {
        '/' => '\\',
        '\\' => '/',
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '╭' => '╮',
        '╮' => '╭',
        '╰' => '╯',
        '╯' => '╰',
        '┌' => '┐',
        '┐' => '┌',
        '└' => '┘',
        '┘' => '└',
        '├' => '┤',
        '┤' => '├',
        _ => c,
    }
}

fn flip_line(line: &str) -> String {
    if !line.contains('\x1b') {
        return line.chars().map(mirror_char).rev().collect();
    }
    let mut items: Vec<(String, char)> = Vec::new();
    let mut pending = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                pending.push(c);
                pending.push(chars.next().unwrap());
                for esc in chars.by_ref() {
                    pending.push(esc);
                    if ('@'..='~').contains(&esc) {
                        break;
                    }
                }
                continue;
            }
            continue;
        }
        items.push((std::mem::take(&mut pending), c));
    }
    let suffix = std::mem::take(&mut pending);
    let mut out = String::with_capacity(line.len());
    for (codes, ch) in items.into_iter().rev() {
        out.push_str(&codes);
        out.push(mirror_char(ch));
    }
    out.push_str(&suffix);
    out
}

fn flip_art(art: &[String]) -> Vec<String> {
    let width = art.iter().map(|l| visible_width(l)).max().unwrap_or(0);
    art.iter()
        .map(|l| {
            let padded = if visible_width(l) < width {
                format!("{l}{}", " ".repeat(width - visible_width(l)))
            } else {
                l.clone()
            };
            flip_line(&padded).trim_end_matches(is_blank).to_string()
        })
        .collect()
}

fn hue_to_rgb(hue: f32) -> (u8, u8, u8) {
    let h = (hue.fract() * 6.0).clamp(0.0, 5.999);
    let sector = h as u8;
    let f = h - h.floor();
    let up = (f * 255.0) as u8;
    let down = ((1.0 - f) * 255.0) as u8;
    let (r, g, b) = match sector {
        0 => (255, up, 0),
        1 => (down, 255, 0),
        2 => (0, 255, up),
        3 => (0, down, 255),
        4 => (up, 0, 255),
        _ => (255, 0, down),
    };
    (r, g, b)
}

fn rainbow_lines(lines: &[String], phase: f32, row_base: usize) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            if line.contains('\x1b') {
                return line.clone();
            }
            let mut out = String::with_capacity(line.len() + 16);
            let mut col: usize = 0;
            let mut painted = false;
            for c in line.chars() {
                if is_blank(c) {
                    out.push(c);
                } else {
                    let hue = (phase + (row_base + row) as f32 * 0.06 + col as f32 * 0.045) % 1.0;
                    let (r, g, b) = hue_to_rgb(hue);
                    out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{c}"));
                    painted = true;
                }
                col += c.width().unwrap_or(0);
            }
            if painted {
                out.push_str("\x1b[0m");
            }
            out
        })
        .collect()
}

fn bold_body(body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 32);
    for line in body.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str(&format!("\x1b[1m{line}\x1b[0m\n"));
        }
    }
    out
}

pub fn render_with_options(text: &str, style: Option<usize>, opts: &RenderOptions) -> String {
    let (term_w, term_h) = term_size();
    let is_tty = std::io::stdout().is_terminal();
    render_with_term(text, style, opts, (term_w, term_h), is_tty)
}

pub(crate) fn render_with_term(
    text: &str,
    style: Option<usize>,
    opts: &RenderOptions,
    term: (usize, usize),
    is_tty: bool,
) -> String {
    let (term_w, term_h) = term;
    let width = opts.width;
    let width = if is_tty {
        width.min(term_w.clamp(10, 200))
    } else {
        width
    };
    let color_ok = std::env::var_os("NO_COLOR").is_none();
    let mut art = get_teto_art(style);
    if opts.flip {
        art = flip_art(&art);
    }
    let bold = opts.bold && color_ok;

    if opts.no_bubble {
        let mut text_lines = get_plain_text_lines(text, width, opts.no_wrap, opts.auto_width);
        if opts.rainbow && color_ok {
            text_lines = rainbow_lines(&text_lines, opts.hue_offset, 0);
            art = rainbow_lines(&art, opts.hue_offset, text_lines.len());
        }
        let max_content_width = text_lines
            .iter()
            .chain(art.iter())
            .map(|l| visible_width(l))
            .max()
            .unwrap_or(0);
        let overall_left = if !is_tty {
            0
        } else {
            match opts.align {
                Align::Center => term_w.saturating_sub(max_content_width) / 2,
                Align::Left => 0,
                Align::Right => term_w.saturating_sub(max_content_width),
            }
        };
        let text_width = text_lines
            .iter()
            .map(|l| visible_width(l))
            .max()
            .unwrap_or(0);
        let text_center = text_width / 2;
        let anchor = ink_center(&art);
        let art_width = art.iter().map(|l| visible_width(l)).max().unwrap_or(0);
        let mut art_left = overall_left;
        if is_tty && text_width > art_width {
            art_left = overall_left + (max_content_width.saturating_sub(art_width)) / 2;
        }
        let mut text_left = (overall_left + anchor).saturating_sub(text_center);
        if is_tty {
            text_left = text_left.min(term_w.saturating_sub(text_width.max(1)));
            if text_width > art_width {
                text_left = overall_left;
            }
        }

        let mut body = String::new();
        for line in &text_lines {
            let pad = text_width.saturating_sub(visible_width(line)) / 2;
            body.push_str(&" ".repeat(text_left + pad));
            body.push_str(line);
            body.push('\n');
        }
        for line in &art {
            body.push_str(&" ".repeat(art_left));
            body.push_str(line);
            body.push('\n');
        }
        if bold {
            body = bold_body(&body);
        }
        if !is_tty {
            return body;
        }
        return pad_vertical(body, term_h, opts.reserve);
    }

    let height_budget = if is_tty && opts.auto_width {
        Some(
            term_h
                .saturating_sub(opts.reserve)
                .saturating_sub(art.len()),
        )
    } else {
        None
    };
    let raw_bubble = get_speech_bubble_lines(
        text,
        width,
        opts.bubble,
        opts.no_wrap,
        opts.auto_width,
        height_budget,
    );
    let bubble = if opts.rainbow && color_ok {
        let painted = rainbow_lines(&raw_bubble, opts.hue_offset, 0);
        art = rainbow_lines(&art, opts.hue_offset, painted.len());
        painted
    } else {
        raw_bubble
    };

    let ntails = tail_count(opts.bubble);
    let tails = &bubble[bubble.len().saturating_sub(ntails)..];

    let max_content_width = bubble
        .iter()
        .chain(art.iter())
        .map(|l| visible_width(l))
        .max()
        .unwrap_or(0);
    let overall_left = if !is_tty {
        0
    } else {
        match opts.align {
            Align::Center => term_w.saturating_sub(max_content_width) / 2,
            Align::Left => 0,
            Align::Right => term_w.saturating_sub(max_content_width),
        }
    };

    let bubble_width = bubble[..bubble.len().saturating_sub(ntails)]
        .iter()
        .map(|l| visible_width(l))
        .max()
        .unwrap_or(0);
    let bubble_center = bubble_width / 2;
    let anchor = ink_center(&art);
    let art_width = art.iter().map(|l| visible_width(l)).max().unwrap_or(0);
    let mut art_left = overall_left;
    if is_tty && bubble_width > art_width {
        art_left = overall_left + (max_content_width.saturating_sub(art_width)) / 2;
    }
    let tail_abs = art_left + anchor;
    let mut bubble_left = tail_abs.saturating_sub(bubble_center);
    let mut tail_left = art_left + anchor;
    if is_tty {
        bubble_left = bubble_left.min(term_w.saturating_sub(bubble_width));
        tail_left = tail_left.min(term_w.saturating_sub(1));
        if bubble_width > art_width {
            bubble_left = overall_left;
        }
    }

    let mut body = String::new();
    for line in &bubble[..bubble.len().saturating_sub(ntails)] {
        body.push_str(&" ".repeat(bubble_left));
        body.push_str(line);
        body.push('\n');
    }
    for tail in tails {
        body.push_str(&" ".repeat(tail_left));
        body.push_str(tail);
        body.push('\n');
    }
    for line in &art {
        body.push_str(&" ".repeat(art_left));
        body.push_str(line);
        body.push('\n');
    }

    if bold {
        body = bold_body(&body);
    }
    if !is_tty {
        return body;
    }
    pad_vertical(body, term_h, opts.reserve)
}

fn pad_vertical(body: String, term_h: usize, reserve: usize) -> String {
    let avail = term_h.saturating_sub(reserve);
    let content_lines = body.lines().count();
    let top_pad = avail.saturating_sub(content_lines) / 2;
    let bottom_pad = avail.saturating_sub(content_lines + top_pad);

    let mut out = String::new();
    for _ in 0..top_pad {
        out.push('\n');
    }
    out.push_str(&body);
    for _ in 0..bottom_pad {
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(width: usize) -> RenderOptions {
        RenderOptions {
            width,
            auto_width: false,
            ..RenderOptions::default()
        }
    }

    fn tty(
        term: (usize, usize),
        align: Align,
        text: &str,
        style: Option<usize>,
        width: usize,
    ) -> String {
        let o = RenderOptions {
            width,
            align,
            auto_width: false,
            ..RenderOptions::default()
        };
        render_with_term(text, style, &o, term, true)
    }

    fn pipe(
        text: &str,
        style: Option<usize>,
        width: usize,
        bubble: BubbleStyle,
        no_bubble: bool,
    ) -> String {
        let o = RenderOptions {
            width,
            align: Align::Left,
            bubble,
            no_bubble,
            auto_width: false,
            ..RenderOptions::default()
        };
        render_with_term(text, style, &o, (80, 24), false)
    }

    fn pipe_opts(text: &str, style: Option<usize>, o: &RenderOptions) -> String {
        render_with_term(text, style, o, (80, 24), false)
    }

    #[test]
    fn render_contains_text_and_art() {
        let out = pipe("hello", Some(3), crate::bubble::MAX_BUBBLE_WIDTH, BubbleStyle::Round, false);
        assert!(out.contains("hello"));
        assert!(out.chars().any(|c| ('⠀'..='⣿').contains(&c)));
    }

    #[test]
    fn render_left_has_no_padding() {
        let left = tty((80, 24), Align::Left, "hi", Some(3), 50);
        let center = tty((80, 24), Align::Center, "hi", Some(3), 50);
        let leading = |s: &str| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.chars().take_while(|c| *c == ' ').count())
                .min()
                .unwrap_or(0)
        };
        assert!(leading(&left) <= leading(&center));
    }

    #[test]
    fn render_narrow_width_wraps() {
        let wide = pipe(
            "hello world foo bar baz qux",
            Some(3),
            60,
            BubbleStyle::Round,
            false,
        );
        let narrow = pipe(
            "hello world foo bar baz qux",
            Some(3),
            24,
            BubbleStyle::Round,
            false,
        );
        assert!(narrow.lines().count() >= wide.lines().count());
    }

    #[test]
    fn render_right_pads_more_than_center() {
        let center = tty((80, 24), Align::Center, "hi", Some(3), 50);
        let right = tty((80, 24), Align::Right, "hi", Some(3), 50);
        let leading = |s: &str| {
            s.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.chars().take_while(|c| *c == ' ').count())
                .min()
                .unwrap_or(0)
        };
        assert!(leading(&right) >= leading(&center));
    }

    #[test]
    fn ink_center_counts_columns() {
        let art = vec!["  テト".to_string()];
        assert_eq!(ink_center(&art), 4);
    }

    #[test]
    fn ink_center_ignores_ansi() {
        let plain = ink_center(&["  XX".to_string()]);
        let colored = ink_center(&["  \x1b[31mXX\x1b[0m".to_string()]);
        assert_eq!(plain, colored);
    }

    #[test]
    fn tty_output_reserves_prompt_row() {
        let out = tty((120, 100), Align::Center, "hi", Some(4), 50);
        assert_eq!(out.lines().count(), 97);
    }

    #[test]
    fn reserve_zero_restores_old_behavior() {
        let mut o = RenderOptions {
            width: 50,
            align: Align::Center,
            auto_width: false,
            ..RenderOptions::default()
        };
        o.reserve = 0;
        let out = render_with_term("hi", Some(4), &o, (120, 100), true);
        assert_eq!(out.lines().count(), 100);
    }

    #[test]
    fn bubble_clamped_to_narrow_term() {
        let out = tty((30, 24), Align::Left, "hi", Some(3), 50);
        for line in out
            .lines()
            .filter(|l| l.contains('╭') || l.contains('│') || l.contains('╰'))
        {
            assert!(visible_width(line) <= 30, "bubble line too wide: {line}");
        }
    }

    #[test]
    fn bubble_styles_render() {
        for bubble in BubbleStyle::ALL {
            let out = pipe("hi", Some(3), 50, bubble, false);
            assert!(out.contains("hi"), "{bubble:?} lost text");
        }
        let think = pipe("hi", Some(3), 50, BubbleStyle::Think, false);
        assert!(think.lines().any(|l| l.trim() == "o"));
        let cowsay = pipe("hi", Some(3), 50, BubbleStyle::Cowsay, false);
        assert!(cowsay.contains('<') && cowsay.contains('>'));
    }

    #[test]
    fn no_bubble_has_no_frame() {
        let out = pipe("hello", Some(3), 50, BubbleStyle::Round, true);
        assert!(out.contains("hello"));
        for ch in ['╭', '╮', '╰', '╯', '│', '─', '┌', '┐', '└', '┘', '<', '>'] {
            assert!(!out.lines().any(|l| l.contains(ch) && l.contains("hello")), "frame leak: {ch}");
        }
        assert!(out.chars().any(|c| ('⠀'..='⣿').contains(&c)));
    }

    #[test]
    fn flip_twice_is_identity() {
        let art = crate::tetoart::get_teto_art(Some(3));
        let once = flip_art(&art);
        let twice = flip_art(&once);
        let canon = |l: &String| {
            l.chars()
                .map(|c| if is_blank(c) { ' ' } else { c })
                .collect::<String>()
        };
        assert_eq!(
            twice.iter().map(canon).collect::<Vec<_>>(),
            art.iter().map(canon).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flip_mirrors_around_frame() {
        let art = vec!["  XX".to_string(), "YYYY".to_string()];
        let flipped = flip_art(&art);
        assert_eq!(flipped, vec!["XX".to_string(), "YYYY".to_string()]);
        let max_plain = art.iter().map(|l| visible_width(l)).max().unwrap();
        let max_flipped = flipped.iter().map(|l| visible_width(l)).max().unwrap();
        assert_eq!(max_plain, max_flipped);
    }

    #[test]
    fn flip_changes_output() {
        let plain = pipe("hi", Some(3), 50, BubbleStyle::Round, false);
        let o = RenderOptions {
            width: 50,
            align: Align::Left,
            flip: true,
            ..RenderOptions::default()
        };
        let flipped = pipe_opts("hi", Some(3), &o);
        assert_ne!(plain, flipped);
        assert!(flipped.contains("hi"));
    }

    #[test]
    fn flip_keeps_colored_art_intact() {
        let art = vec!["\x1b[31mAB\x1b[0m".to_string()];
        let flipped = flip_art(&art);
        assert_eq!(flipped.len(), 1);
        assert!(flipped[0].contains('A') && flipped[0].contains('B'));
        assert!(flipped[0].ends_with("\x1b[0m"));
        assert_eq!(visible_width(&flipped[0]), 2);
    }

    #[test]
    fn mirror_char_pairs() {
        assert_eq!(mirror_char('/'), '\\');
        assert_eq!(mirror_char('\\'), '/');
        assert_eq!(mirror_char('('), ')');
        assert_eq!(mirror_char('╭'), '╮');
        for c in ['⠋', '⣿', '⢿'] {
            assert_eq!(mirror_char(mirror_char(c)), c);
        }
    }

    #[test]
    fn bold_wraps_lines() {
        let o = RenderOptions {
            width: 50,
            align: Align::Left,
            bold: true,
            ..RenderOptions::default()
        };
        let out = pipe_opts("hello", Some(3), &o);
        assert!(out.contains("\x1b[1m"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn rainbow_paints_art() {
        let o = RenderOptions {
            width: 50,
            align: Align::Left,
            rainbow: true,
            ..RenderOptions::default()
        };
        let out = pipe_opts("hello", Some(3), &o);
        assert!(out.contains("\x1b[38;2;"));
        assert!(strip_ansi(&out).contains("hello"));
    }

    #[test]
    fn rainbow_paints_bubble_text() {
        let o = RenderOptions {
            width: 50,
            align: Align::Left,
            rainbow: true,
            ..RenderOptions::default()
        };
        let out = pipe_opts("hello", Some(3), &o);
        let text_line = out
            .lines()
            .find(|l| strip_ansi(l).contains("hello"))
            .unwrap();
        assert!(text_line.contains("\x1b[38;2;"));
        let frame = out.lines().find(|l| l.contains('╭')).unwrap();
        assert!(frame.contains("\x1b[38;2;"));
    }

    #[test]
    fn hue_offset_shifts_rainbow_and_cycles() {
        let base = RenderOptions {
            width: 50,
            align: Align::Left,
            rainbow: true,
            ..RenderOptions::default()
        };
        let mut shifted = base;
        shifted.hue_offset = 0.37;
        let a = pipe_opts("hello", Some(3), &base);
        let b = pipe_opts("hello", Some(3), &shifted);
        assert_ne!(a, b);
        assert_eq!(hue_to_rgb(0.25), hue_to_rgb(1.25));
    }

    #[test]
    fn fits_term_rejects_wide_and_tall() {
        assert!(fits_term("hi\nbye\n", (80, 24), 1));
        assert!(!fits_term("hi\nbye\n", (2, 24), 1));
        assert!(!fits_term("hi\nbye\n", (80, 2), 1));
        assert!(fits_term("", (80, 24), 1));
        assert!(!fits_term("hi\nbye\n", (80, 3), 2));
    }

    #[test]
    fn no_wrap_keeps_table_in_render() {
        let o = RenderOptions {
            width: 50,
            align: Align::Left,
            no_wrap: true,
            ..RenderOptions::default()
        };
        let out = pipe_opts("a  b", Some(3), &o);
        assert!(out.contains("a  b"));
    }

    #[test]
    fn auto_width_is_default_and_explicit_wins() {
        let auto_out = pipe_opts("hi", Some(3), &RenderOptions::default());
        let fixed_out = pipe("hi", Some(3), 50, BubbleStyle::Round, false);
        let outer = |s: &str| {
            s.lines()
                .find(|l| l.contains('╭'))
                .map(|l| {
                    let lead = l.chars().take_while(|c| *c == ' ').count();
                    visible_width(l) - lead
                })
                .unwrap_or(0)
        };
        assert!(outer(&auto_out) < outer(&fixed_out));
        assert_eq!(outer(&auto_out), "hi".width() + 6);
    }

    #[test]
    fn auto_width_caps_at_term_on_tty() {
        let long = "word ".repeat(100);
        let o = RenderOptions {
            width: usize::MAX,
            auto_width: true,
            align: Align::Left,
            ..RenderOptions::default()
        };
        let out = render_with_term(&long, Some(3), &o, (80, 24), true);
        for line in out.lines().filter(|l| l.contains('╭') || l.contains('│')) {
            assert!(visible_width(line) <= 80, "too wide: {line}");
        }
    }

    #[test]
    fn wide_bubble_centers_art_inside() {
        let text = "Ullamco do culpa proident proident amet. Do incididunt vocaloid, laborum id \
            nostrud cupidatat. Do do ea sunt proident utauloid bread. Occaecat ex enim bread \
            mollit id eiusmod vocaloid cupidatat et.";
        let out = tty((120, 40), Align::Center, text, Some(5), 200);
        let leading = |l: &str| l.chars().take_while(|c| *c == ' ').count();
        let top = out.lines().find(|l| l.contains('╭')).unwrap();
        let bubble_lead = leading(top);
        let bubble_outer = visible_width(top).saturating_sub(bubble_lead);
        let art_w = crate::tetoart::get_teto_art(Some(5))
            .iter()
            .map(|l| visible_width(l))
            .max()
            .unwrap();
        let art_lead = out
            .lines()
            .filter(|l| l.chars().any(|c| ('⠀'..='⣿').contains(&c)))
            .map(leading)
            .min()
            .unwrap();
        assert_eq!(bubble_lead, (120 - bubble_outer) / 2);
        assert_eq!(art_lead, bubble_lead + (bubble_outer - art_w) / 2);
        assert!(art_lead > bubble_lead);
    }

    #[test]
    fn short_term_widens_bubble_to_fit_height() {
        let text = "word ".repeat(120);
        let render_auto = |term| {
            let o = RenderOptions {
                width: 200,
                auto_width: true,
                align: Align::Center,
                ..RenderOptions::default()
            };
            render_with_term(&text, Some(4), &o, term, true)
        };
        let outer = |s: &str| {
            s.lines()
                .find(|l| l.contains('╭'))
                .map(|l| {
                    let lead = l.chars().take_while(|c| *c == ' ').count();
                    visible_width(l) - lead
                })
                .unwrap_or(0)
        };
        let tall = render_auto((120, 100));
        let short = render_auto((120, 40));
        assert!(outer(&short) > outer(&tall));
        assert!(short.lines().count() <= 37);
    }
}
