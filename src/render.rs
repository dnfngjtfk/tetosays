use std::io::IsTerminal;
use terminal_size::{terminal_size, Height, Width};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::bubble::get_speech_bubble_lines;
use crate::tetoart::{get_teto_art, is_blank};

fn term_size() -> (usize, usize) {
    let (w, h) = terminal_size()
        .map(|(Width(w), Height(h))| (w as usize, h as usize))
        .unwrap_or((80, 24));
    (if w > 0 { w } else { 80 }, if h > 0 { h } else { 24 })
}

fn ink_center(art: &[String]) -> usize {
    let mut min_start = usize::MAX;
    let mut max_end = 0;
    let mut found = false;
    for line in art {
        let mut col = 0;
        let mut start = None;
        let mut end = 0;
        for c in line.chars() {
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

pub fn render_with_options(text: &str, style: Option<usize>, width: usize, align: Align) -> String {
    let bubble = get_speech_bubble_lines(text, width);
    let art = get_teto_art(style);

    let tails = &bubble[bubble.len().saturating_sub(2)..];

    let max_content_width = bubble
        .iter()
        .chain(art.iter())
        .map(|l| l.width())
        .max()
        .unwrap_or(0);
    let (term_w, term_h) = term_size();
    let overall_left = match align {
        Align::Center => term_w.saturating_sub(max_content_width) / 2,
        Align::Left => 0,
        Align::Right => term_w.saturating_sub(max_content_width),
    };

    let bubble_width = bubble.first().map(|l| l.width()).unwrap_or(0);
    let bubble_center = bubble_width / 2;
    let anchor = ink_center(&art);
    let tail_abs = overall_left + anchor;
    let bubble_left = tail_abs.saturating_sub(bubble_center);

    let mut body = String::new();
    for line in &bubble[..bubble.len().saturating_sub(2)] {
        body.push_str(&" ".repeat(bubble_left));
        body.push_str(line);
        body.push('\n');
    }
    for tail in tails {
        body.push_str(&" ".repeat(overall_left + anchor));
        body.push_str(tail);
        body.push('\n');
    }
    for line in &art {
        body.push_str(&" ".repeat(overall_left));
        body.push_str(line);
        body.push('\n');
    }

    if !std::io::stdout().is_terminal() {
        return body;
    }
    let content_lines = body.lines().count();
    let top_pad = term_h.saturating_sub(content_lines) / 2;
    let bottom_pad = term_h.saturating_sub(content_lines + top_pad);

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

    #[test]
    fn render_contains_text_and_art() {
        let out = render_with_options(
            "hello",
            Some(3),
            crate::bubble::MAX_BUBBLE_WIDTH,
            Align::Center,
        );
        assert!(out.contains("hello"));
        assert!(out.chars().any(|c| ('⠀'..='⣿').contains(&c)));
    }

    #[test]
    fn render_left_has_no_padding() {
        let left = render_with_options("hi", Some(3), 50, Align::Left);
        let center = render_with_options("hi", Some(3), 50, Align::Center);
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
        let wide = render_with_options("hello world foo bar baz qux", Some(3), 60, Align::Left);
        let narrow = render_with_options("hello world foo bar baz qux", Some(3), 24, Align::Left);
        assert!(narrow.lines().count() >= wide.lines().count());
    }

    #[test]
    fn render_right_pads_more_than_center() {
        let center = render_with_options("hi", Some(3), 50, Align::Center);
        let right = render_with_options("hi", Some(3), 50, Align::Right);
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
}
