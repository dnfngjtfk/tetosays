use std::io::IsTerminal;
use terminal_size::{terminal_size, Height, Width};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::bubble::{get_plain_text_lines, get_speech_bubble_lines, BubbleStyle};
use crate::tetoart::{get_teto_art, is_blank};

fn term_size() -> (usize, usize) {
    let (w, h) = terminal_size()
        .map(|(Width(w), Height(h))| (w as usize, h as usize))
        .unwrap_or((80, 24));
    (if w > 0 { w } else { 80 }, if h > 0 { h } else { 24 })
}

fn strip_ansi(s: &str) -> String {
    // Minimal CSI/SGR skipper: ESC [ ... final-byte, plus lone ESC chars.
    // Enough for colored custom arts; keeps normal text untouched.
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

pub fn render_with_options(
    text: &str,
    style: Option<usize>,
    width: usize,
    align: Align,
    bubble: BubbleStyle,
    no_bubble: bool,
) -> String {
    let (term_w, term_h) = term_size();
    let is_tty = std::io::stdout().is_terminal();
    render_with_term(text, style, width, align, bubble, no_bubble, (term_w, term_h), is_tty)
}

pub(crate) fn render_with_term(
    text: &str,
    style: Option<usize>,
    width: usize,
    align: Align,
    bubble_style: BubbleStyle,
    no_bubble: bool,
    term: (usize, usize),
    is_tty: bool,
) -> String {
    let (term_w, term_h) = term;
    // On a real tty, never request a bubble wider than the terminal itself,
    // otherwise bubble + wide art wraps and shreds the picture.
    let width = if is_tty {
        width.min(term_w.clamp(10, 200))
    } else {
        width
    };
    let art = get_teto_art(style);

    if no_bubble {
        let text_lines = get_plain_text_lines(text, width);
        let max_content_width = text_lines
            .iter()
            .chain(art.iter())
            .map(|l| visible_width(l))
            .max()
            .unwrap_or(0);
        let overall_left = if !is_tty {
            0
        } else {
            match align {
                Align::Center => term_w.saturating_sub(max_content_width) / 2,
                Align::Left => 0,
                Align::Right => term_w.saturating_sub(max_content_width),
            }
        };
        // Center the text block over the art's ink center, like the bubble.
        let text_width = text_lines
            .iter()
            .map(|l| visible_width(l))
            .max()
            .unwrap_or(0);
        let text_center = text_width / 2;
        let anchor = ink_center(&art);
        let mut text_left = (overall_left + anchor).saturating_sub(text_center);
        if is_tty {
            text_left = text_left.min(term_w.saturating_sub(text_width.max(1)));
        }

        let mut body = String::new();
        for line in &text_lines {
            // Center each line within the text block so short lines
            // don't hug the left edge.
            let pad = text_width.saturating_sub(visible_width(line)) / 2;
            body.push_str(&" ".repeat(text_left + pad));
            body.push_str(line);
            body.push('\n');
        }
        for line in &art {
            body.push_str(&" ".repeat(overall_left));
            body.push_str(line);
            body.push('\n');
        }
        if !is_tty {
            return body;
        }
        return pad_vertical(body, term_h);
    }

    let bubble = get_speech_bubble_lines(text, width, bubble_style);

    let tails = &bubble[bubble.len().saturating_sub(2)..];

    let max_content_width = bubble
        .iter()
        .chain(art.iter())
        .map(|l| visible_width(l))
        .max()
        .unwrap_or(0);
    // Piped output has no real terminal width; centering against a fake
    // 80-column fallback only adds noise, so align to the left edge.
    let overall_left = if !is_tty {
        0
    } else {
        match align {
            Align::Center => term_w.saturating_sub(max_content_width) / 2,
            Align::Left => 0,
            Align::Right => term_w.saturating_sub(max_content_width),
        }
    };

    let bubble_width = bubble.first().map(|l| visible_width(l)).unwrap_or(0);
    let bubble_center = bubble_width / 2;
    let anchor = ink_center(&art);
    let tail_abs = overall_left + anchor;
    // Keep the bubble inside the right edge on tty: it is anchored to the
    // art's ink center, which can otherwise push it past the margin
    // (notably with --align right and asymmetric arts).
    let mut bubble_left = tail_abs.saturating_sub(bubble_center);
    let mut tail_left = overall_left + anchor;
    if is_tty {
        bubble_left = bubble_left.min(term_w.saturating_sub(bubble_width));
        tail_left = tail_left.min(term_w.saturating_sub(1));
    }

    let mut body = String::new();
    for line in &bubble[..bubble.len().saturating_sub(2)] {
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
        body.push_str(&" ".repeat(overall_left));
        body.push_str(line);
        body.push('\n');
    }

    if !is_tty {
        return body;
    }
    pad_vertical(body, term_h)
}

fn pad_vertical(body: String, term_h: usize) -> String {
    // Reserve one row for the shell prompt: filling exactly term_h rows
    // scrolls the terminal by one line and pushes the top off-screen.
    let avail = term_h.saturating_sub(1);
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

    fn tty(
        term: (usize, usize),
        align: Align,
        text: &str,
        style: Option<usize>,
        width: usize,
    ) -> String {
        render_with_term(
            text,
            style,
            width,
            align,
            BubbleStyle::Round,
            false,
            term,
            true,
        )
    }

    fn pipe(
        text: &str,
        style: Option<usize>,
        width: usize,
        bubble: BubbleStyle,
        no_bubble: bool,
    ) -> String {
        render_with_term(
            text,
            style,
            width,
            Align::Left,
            bubble,
            no_bubble,
            (80, 24),
            false,
        )
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
        // Tall terminal so content fits: total must be term_h - 1, not term_h.
        let out = tty((120, 100), Align::Center, "hi", Some(4), 50);
        assert_eq!(out.lines().count(), 99);
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
        for bubble in [
            BubbleStyle::Round,
            BubbleStyle::Sharp,
            BubbleStyle::Ascii,
            BubbleStyle::Cowsay,
            BubbleStyle::Think,
        ] {
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
}
