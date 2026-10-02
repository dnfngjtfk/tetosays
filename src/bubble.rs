use clap::ValueEnum;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub const MAX_BUBBLE_WIDTH: usize = 50;
const AUTO_SINGLE_LINE_LIMIT: usize = 64;
const AUTO_WIDTH_RATIO: f64 = 4.0;

fn auto_wrap_width(text: &str, cap: usize) -> usize {
    auto_wrap_width_ratio(text, cap, AUTO_WIDTH_RATIO)
}

fn auto_wrap_width_ratio(text: &str, cap: usize, ratio: f64) -> usize {
    let cap = cap.max(6);
    let mut single = 0usize;
    let mut total = 0usize;
    let mut words = 0usize;
    for paragraph in text.lines() {
        single = single.max(paragraph.width());
        for w in paragraph.split_whitespace() {
            total += w.width();
            words += 1;
        }
    }
    total += words.saturating_sub(1);
    if single <= AUTO_SINGLE_LINE_LIMIT {
        return single.max(1).min(cap);
    }
    ((total as f64 * ratio).sqrt() as usize)
        .max(6)
        .min(cap)
}

const BAGUETTE_WIDTH_RATIO: f64 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, ValueEnum)]
pub enum BubbleStyle {
    /// Rounded unicode bubble (default): ╭─╮ │ ╰─╯
    #[default]
    Round,
    /// Square unicode bubble: ┌─┐ │ └─┘
    Sharp,
    /// Double-line unicode bubble: ╔═╗ ║ ╚═╝
    Double,
    /// Heavy unicode bubble: ┏━┓ ┃ ┗━┛
    Heavy,
    /// Star bubble: *** * *
    Star,
    /// Dotted bubble: ·┄· ┆
    Dots,
    /// Baguette bubble: paren body with slashed crust
    Baguette,
    /// Plain ASCII bubble: +---+ | |
    Ascii,
    /// Classic cowsay bubble: _____ < > / \ | | \ /
    Cowsay,
    /// Thought bubble: (---) ( ) with o tails
    Think,
}

impl BubbleStyle {
    #[cfg(test)]
    pub const ALL: [BubbleStyle; 10] = [
        BubbleStyle::Round,
        BubbleStyle::Sharp,
        BubbleStyle::Double,
        BubbleStyle::Heavy,
        BubbleStyle::Star,
        BubbleStyle::Dots,
        BubbleStyle::Ascii,
        BubbleStyle::Cowsay,
        BubbleStyle::Think,
        BubbleStyle::Baguette,
    ];
}

/// Number of trailing pointer lines a style appends after the bubble body.
pub fn tail_count(style: BubbleStyle) -> usize {
    match style {
        BubbleStyle::Baguette => 3,
        _ => 2,
    }
}

pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![text.to_string()]
        };
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let word_width = word.width();
        let current_width = current_line.width();
        let separator_width = usize::from(!current_line.is_empty());

        if word_width <= max_width && current_width + separator_width + word_width <= max_width {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
            continue;
        }

        if !current_line.is_empty() {
            lines.push(std::mem::take(&mut current_line));
        }

        if word_width <= max_width {
            current_line.push_str(word);
            continue;
        }

        let chunks = split_word_by_width(word, max_width);
        let last_index = chunks.len().saturating_sub(1);
        for (index, chunk) in chunks.into_iter().enumerate() {
            if index == last_index && chunk.width() < max_width {
                current_line = chunk;
            } else {
                lines.push(chunk);
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn split_word_by_width(word: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut chunk = String::new();
    let mut chunk_width = 0;

    for character in word.chars() {
        let character_width = character.width().unwrap_or(0);
        if !chunk.is_empty() && chunk_width + character_width > max_width {
            chunks.push(std::mem::take(&mut chunk));
            chunk_width = 0;
        }
        chunk.push(character);
        chunk_width += character_width;
    }

    if !chunk.is_empty() {
        chunks.push(chunk);
    }

    chunks
}

fn expand_tab_stops(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut col: usize = 0;
    for c in line.chars() {
        if c == '\t' {
            let spaces = 8 - (col % 8);
            for _ in 0..spaces {
                out.push(' ');
            }
            col += spaces;
        } else {
            out.push(c);
            col += c.width().unwrap_or(0);
        }
    }
    out
}

fn cut_line_by_width(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![line.to_string()];
    }
    let mut chunks = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for c in line.chars() {
        let w = c.width().unwrap_or(0);
        if !cur.is_empty() && cur_w + w > width {
            chunks.push(std::mem::take(&mut cur));
            cur_w = 0;
        }
        cur.push(c);
        cur_w += w;
    }
    if !cur.is_empty() || chunks.is_empty() {
        chunks.push(cur);
    }
    chunks
}

pub fn wrap_multiline(text: &str, width: usize, no_wrap: bool) -> Vec<String> {
    let mut out = Vec::new();
    for paragraph in text.lines() {
        if no_wrap {
            let expanded = expand_tab_stops(paragraph);
            let kept = expanded.trim_end().to_string();
            if kept.is_empty() {
                out.push(String::new());
            } else if kept.width() <= width {
                out.push(kept);
            } else {
                out.extend(cut_line_by_width(&kept, width));
            }
        } else if paragraph.trim().is_empty() {
            out.push(String::new());
        } else {
            out.extend(wrap_text(paragraph, width));
        }
    }
    if out.is_empty() && !text.is_empty() {
        if no_wrap {
            let kept = expand_tab_stops(text).trim_end().to_string();
            if kept.width() <= width {
                out.push(kept);
            } else {
                out.extend(cut_line_by_width(&kept, width));
            }
        } else {
            out.extend(wrap_text(text, width));
        }
    }
    while out.first().is_some_and(|l| l.is_empty()) && out.len() > 1 {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.is_empty()) && out.len() > 1 {
        out.pop();
    }
    out
}

fn bubble_content_width(text: &str, inner_max: usize, no_wrap: bool) -> (Vec<String>, usize) {
    let temp_lines = wrap_multiline(text, inner_max.saturating_sub(4), no_wrap);
    let content_width = temp_lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(0);
    (temp_lines, content_width)
}

pub fn get_plain_text_lines(
    text: &str,
    max_width: usize,
    no_wrap: bool,
    auto_width: bool,
) -> Vec<String> {
    let effective_max = if auto_width {
        if no_wrap {
            max_width.max(1)
        } else {
            auto_wrap_width(text, max_width)
        }
    } else {
        max_width.clamp(10, 200)
    };
    let lines = wrap_multiline(text, effective_max, no_wrap);
    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

pub fn get_speech_bubble_lines(
    text: &str,
    max_width: usize,
    style: BubbleStyle,
    no_wrap: bool,
    auto_width: bool,
    height_budget: Option<usize>,
) -> Vec<String> {
    let (inner_max, lower) = if auto_width {
        let cap = max_width.saturating_sub(2).max(6);
        let mut content_w = if no_wrap {
            cap
        } else {
            auto_wrap_width(text, cap)
        };
        if let Some(budget) = height_budget {
            while wrap_multiline(text, content_w, no_wrap).len().max(1) + 4 > budget
                && content_w < cap
            {
                content_w = (content_w * 5 / 4 + 1).min(cap);
            }
        }
        (content_w.saturating_add(4).min(cap), 6)
    } else {
        let effective_max = max_width.clamp(10, 200);
        let inner = effective_max.saturating_sub(2).max(6);
        (inner, inner)
    };

    let (_temp, content_width) = bubble_content_width(text, inner_max, no_wrap);
    let bubble_width = (content_width + 4).clamp(lower, inner_max);

    let lines = wrap_multiline(text, bubble_width.saturating_sub(4), no_wrap);
    let padded: Vec<String> = if lines.is_empty() {
        vec![" ".repeat(bubble_width.saturating_sub(2))]
    } else {
        lines
            .iter()
            .map(|line| {
                let line_display_width = line.width();
                let padding_needed = (bubble_width - 2).saturating_sub(line_display_width);
                let left_padding = padding_needed / 2;
                let right_padding = padding_needed - left_padding;
                format!(
                    "{}{}{}",
                    " ".repeat(left_padding),
                    line,
                    " ".repeat(right_padding)
                )
            })
            .collect()
    };
    let mut bubble_lines = Vec::new();

    match style {
        BubbleStyle::Round => {
            bubble_lines.push(format!("╭{}╮", "─".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("│ {p} │"));
            }
            bubble_lines.push(format!("╰{}╯", "─".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Sharp => {
            bubble_lines.push(format!("┌{}┐", "─".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("│ {p} │"));
            }
            bubble_lines.push(format!("└{}┘", "─".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Double => {
            bubble_lines.push(format!("╔{}╗", "═".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("║ {p} ║"));
            }
            bubble_lines.push(format!("╚{}╝", "═".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Heavy => {
            bubble_lines.push(format!("┏{}┓", "━".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("┃ {p} ┃"));
            }
            bubble_lines.push(format!("┗{}┛", "━".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Star => {
            bubble_lines.push(format!("*{}*", "*".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("* {p} *"));
            }
            bubble_lines.push(format!("*{}*", "*".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Dots => {
            bubble_lines.push(format!("·{}·", "┄".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("┆ {p} ┆"));
            }
            bubble_lines.push(format!("·{}·", "┄".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Ascii => {
            bubble_lines.push(format!("+{}+", "-".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("| {p} |"));
            }
            bubble_lines.push(format!("+{}+", "-".repeat(bubble_width)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Think => {
            bubble_lines.push(format!("({})", "─".repeat(bubble_width)));
            for p in &padded {
                bubble_lines.push(format!("( {p} )"));
            }
            bubble_lines.push(format!("({})", "─".repeat(bubble_width)));
            bubble_lines.push("o".to_string());
            bubble_lines.push(" o".to_string());
        }
        BubbleStyle::Cowsay => {
            bubble_lines.push(format!(" {}", "_".repeat(bubble_width + 1)));
            if padded.len() == 1 {
                bubble_lines.push(format!("< {} >", padded[0]));
            } else {
                for (i, p) in padded.iter().enumerate() {
                    if i == 0 {
                        bubble_lines.push(format!("/ {p} \\"));
                    } else if i == padded.len() - 1 {
                        bubble_lines.push(format!("\\ {p} /"));
                    } else {
                        bubble_lines.push(format!("| {p} |"));
                    }
                }
            }
            bubble_lines.push(format!(" {}", "-".repeat(bubble_width + 1)));
            bubble_lines.push("\\".to_string());
            bubble_lines.push(" \\".to_string());
        }
        BubbleStyle::Baguette => {
            bubble_lines.extend(baguette_lines(text, max_width, no_wrap, auto_width, height_budget));
        }
    }

    bubble_lines
}

fn baguette_slash_run(width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let n = ((width + 3) / 4).max(1);
    if n == 1 {
        let left = (width - 1) / 2;
        return format!("{}{}{}", " ".repeat(left), "/", " ".repeat(width - 1 - left));
    }
    let gaps = n - 1;
    let spaces = width - n;
    let base = spaces / gaps;
    let extra = spaces % gaps;
    let mid = (gaps - 1) / 2;
    let mut run = String::with_capacity(width);
    for i in 0..n {
        run.push('/');
        if i < gaps {
            let mut gap = base;
            if i >= mid && i < mid + extra {
                gap += 1;
            }
            run.push_str(&" ".repeat(gap));
        }
    }
    run
}

fn baguette_lines(
    text: &str,
    max_width: usize,
    no_wrap: bool,
    auto_width: bool,
    height_budget: Option<usize>,
) -> Vec<String> {
    let cap = if auto_width {
        max_width
    } else {
        max_width.clamp(10, 200)
    };
    let tw_max = cap.saturating_sub(6).max(1);
    let mut tw = if !auto_width || no_wrap {
        tw_max
    } else {
        auto_wrap_width_ratio(text, tw_max, BAGUETTE_WIDTH_RATIO).min(tw_max)
    };
    if auto_width {
        if let Some(budget) = height_budget {
            let total = |n: usize| if n == 0 { 6 } else { n + 7 };
            while total(wrap_multiline(text, tw, no_wrap).len()) > budget && tw < tw_max {
                tw = (tw * 5 / 4 + 1).min(tw_max);
            }
        }
    }
    let blines = wrap_multiline(text, tw, no_wrap);
    let content = blines.iter().map(|l| l.width()).max().unwrap_or(0);
    let eff = if auto_width {
        content
    } else {
        content.max(cap.saturating_sub(6))
    };
    let inner = eff + 4;
    let outer = inner + 2;
    let mut rows = Vec::new();
    if outer >= 12 {
        let slashes = baguette_slash_run(outer - 12);
        rows.push(format!(" .-'  {slashes}  '-."));
    } else {
        rows.push("_".repeat(outer.saturating_sub(2)));
    }
    if blines.is_empty() {
        rows.push(format!("({})", " ".repeat(inner)));
    } else {
        rows.push(format!("({})", " ".repeat(inner)));
        for line in &blines {
            let pad = inner.saturating_sub(line.width());
            let left = pad / 2;
            rows.push(format!(
                "({}{}{})",
                " ".repeat(left),
                line,
                " ".repeat(pad - left)
            ));
        }
        rows.push(format!("({})", " ".repeat(inner)));
    }
    if outer >= 9 {
        rows.push(format!(" '-.{}.-'", "_".repeat(outer - 8)));
    } else {
        rows.push("-".repeat(outer));
    }
    rows.push(".".to_string());
    rows.push("o".to_string());
    rows.push(" O".to_string());
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_basic() {
        assert_eq!(
            wrap_text("Hello world this is a test", 15),
            vec!["Hello world", "this is a test"]
        );
    }

    #[test]
    fn wrap_empty() {
        assert_eq!(wrap_text("", 10), Vec::<String>::new());
    }

    #[test]
    fn wrap_long_word() {
        let wrapped = wrap_text("ThisIsAVeryLongWordThatExceedsMaxWidth", 10);
        assert_eq!(wrapped.concat(), "ThisIsAVeryLongWordThatExceedsMaxWidth");
        assert!(wrapped.iter().all(|l| l.width() <= 10));
    }

    #[test]
    fn wrap_unicode_by_display_width() {
        let wrapped = wrap_text("テトテトテトテトテト", 8);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|l| l.width() <= 8));
    }

    #[test]
    fn bubble_widths_consistent() {
        for style in BubbleStyle::ALL {
            if style == BubbleStyle::Baguette {
                continue;
            }
            let bubble = get_speech_bubble_lines("Hello Teto!", MAX_BUBBLE_WIDTH, style, false, false, None);
            let border_width = bubble[0].width();
            for line in &bubble[..bubble.len() - 2] {
                assert_eq!(line.width(), border_width, "{style:?}: {line}");
            }
        }
    }

    #[test]
    fn bubble_empty() {
        let bubble = get_speech_bubble_lines("", MAX_BUBBLE_WIDTH, BubbleStyle::Round, false, false, None);
        assert_eq!(bubble.len(), 5);
    }

    #[test]
    fn bubble_respects_custom_width() {
        let text = "Hello world this is a width test for the bubble";
        let narrow = get_speech_bubble_lines(text, 30, BubbleStyle::Round, false, false, None);
        let wide = get_speech_bubble_lines(text, 60, BubbleStyle::Round, false, false, None);
        assert!(narrow[0].width() <= 30);
        assert!(wide[0].width() > narrow[0].width());
        assert!(wide[0].width() <= 60);
    }

    #[test]
    fn fixed_width_is_exact() {
        let bubble = get_speech_bubble_lines("hi", 30, BubbleStyle::Round, false, false, None);
        assert_eq!(bubble[0].width(), 30);
        for line in &bubble[..bubble.len() - 2] {
            assert_eq!(line.width(), 30);
        }
    }

    #[test]
    fn bubble_preserves_newlines() {
        let bubble = get_speech_bubble_lines("line1\nline2\nline3", 50, BubbleStyle::Round, false, false, None);
        let joined = bubble.join("\n");
        assert!(joined.contains("line1"));
        assert!(joined.contains("line2"));
        assert!(joined.contains("line3"));
        let rows: Vec<&String> = bubble.iter().filter(|l| l.contains("line")).collect();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn bubble_outer_width_bounded() {
        for w in [10usize, 20, 50, 200] {
            for style in BubbleStyle::ALL {
                let bubble = get_speech_bubble_lines("Hello world this is a test", w, style, false, false, None);
                assert!(
                    bubble[0].width() <= w,
                    "{style:?} outer width {} exceeds max {w}",
                    bubble[0].width()
                );
            }
        }
    }

    #[test]
    fn bubble_styles_have_distinct_borders() {
        let round = get_speech_bubble_lines("hi", 50, BubbleStyle::Round, false, false, None);
        let sharp = get_speech_bubble_lines("hi", 50, BubbleStyle::Sharp, false, false, None);
        let double = get_speech_bubble_lines("hi", 50, BubbleStyle::Double, false, false, None);
        let heavy = get_speech_bubble_lines("hi", 50, BubbleStyle::Heavy, false, false, None);
        let star = get_speech_bubble_lines("hi", 50, BubbleStyle::Star, false, false, None);
        let dots = get_speech_bubble_lines("hi", 50, BubbleStyle::Dots, false, false, None);
        let ascii = get_speech_bubble_lines("hi", 50, BubbleStyle::Ascii, false, false, None);
        let cowsay = get_speech_bubble_lines("hi", 50, BubbleStyle::Cowsay, false, false, None);
        let think = get_speech_bubble_lines("hi", 50, BubbleStyle::Think, false, false, None);
        let baguette = get_speech_bubble_lines("hi", 50, BubbleStyle::Baguette, false, false, None);
        assert!(round[0].starts_with('╭'));
        assert!(sharp[0].starts_with('┌'));
        assert!(double[0].starts_with('╔'));
        assert!(heavy[0].starts_with('┏'));
        assert!(star[0].starts_with('*'));
        assert!(dots[0].starts_with('·'));
        assert!(ascii[0].starts_with('+'));
        assert!(cowsay[0].contains('_') && cowsay[1].starts_with('<'));
        assert!(think[0].starts_with('(') && think[think.len() - 2] == "o");
        assert!(baguette.iter().any(|l| l.contains(".-'")));
        assert!(baguette.iter().any(|l| l.contains('(') && l.contains(')')));
        assert_eq!(round[round.len() - 2], "\\");
        assert_eq!(think[think.len() - 1], " o");
    }

    #[test]
    fn cowsay_multiline_borders() {
        let bubble = get_speech_bubble_lines(
            "line1 line2 line3 line4 line5 line6",
            20,
            BubbleStyle::Cowsay,
            false,
            false,
            None,
        );
        assert!(bubble[1].starts_with('/'));
        assert!(bubble[bubble.len() - 4].starts_with('\\'));
    }

    #[test]
    fn auto_width_hugs_short_text() {
        let fixed = get_speech_bubble_lines("hi", 50, BubbleStyle::Round, false, false, None);
        let auto =
            get_speech_bubble_lines("hi", usize::MAX, BubbleStyle::Round, false, true, None);
        assert!(auto[0].width() < fixed[0].width());
        assert_eq!(auto[0].width(), "hi".width() + 6);
    }

    #[test]
    fn auto_width_has_no_upper_clamp() {
        let long = "word ".repeat(100);
        let fixed = get_speech_bubble_lines(&long, 50, BubbleStyle::Round, false, false, None);
        assert!(fixed[0].width() <= 50);
        let auto =
            get_speech_bubble_lines(&long, usize::MAX, BubbleStyle::Round, false, true, None);
        let outer = auto[0].width();
        let body_lines = auto.len() - 4;
        assert!(outer < 100, "auto should wrap, got {outer}");
        assert!(body_lines >= 6, "expected several lines, got {body_lines}");
        let ratio = outer as f64 / body_lines as f64;
        assert!((2.0..=7.0).contains(&ratio), "ratio {ratio}");
    }

    #[test]
    fn auto_medium_sentence_stays_single_line() {
        let text = "Pack my box with five dozen liquor jugs and a lime.";
        assert!(text.width() <= AUTO_SINGLE_LINE_LIMIT);
        let auto =
            get_speech_bubble_lines(text, usize::MAX, BubbleStyle::Round, false, true, None);
        assert_eq!(auto.len(), 5);
        assert_eq!(auto[0].width(), text.width() + 6);
    }

    #[test]
    fn auto_plain_text_unbounded() {
        let long = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod";
        let auto = get_plain_text_lines(long, usize::MAX, false, true);
        assert!(auto.len() > 1);
        assert_eq!(auto.join(" "), long);
    }

    #[test]
    fn height_budget_widens_bubble_to_fit() {
        let long = "word ".repeat(100);
        let free =
            get_speech_bubble_lines(&long, usize::MAX, BubbleStyle::Round, false, true, None);
        let tight = get_speech_bubble_lines(
            &long,
            usize::MAX,
            BubbleStyle::Round,
            false,
            true,
            Some(8),
        );
        assert!(tight.len() <= 8);
        assert!(tight[0].width() > free[0].width());
    }

    #[test]
    fn height_budget_best_effort_at_cap() {
        let long = "word ".repeat(100);
        let tight = get_speech_bubble_lines(&long, 50, BubbleStyle::Round, false, true, Some(2));
        assert!(tight[0].width() <= 50);
    }

    #[test]
    fn plain_text_has_no_frame() {
        let lines = get_plain_text_lines("hello world", 50, false, false);
        assert_eq!(lines, vec!["hello world".to_string()]);
        let joined = lines.join("\n");
        assert!(!joined.contains('╭') && !joined.contains('+'));
    }

    #[test]
    fn no_wrap_keeps_spacing() {
        let wrapped = wrap_multiline("a   b    c", 50, false);
        assert_eq!(wrapped, vec!["a b c".to_string()]);
        let verbatim = wrap_multiline("a   b    c", 50, true);
        assert_eq!(verbatim, vec!["a   b    c".to_string()]);
    }

    #[test]
    fn no_wrap_keeps_indent() {
        let verbatim = wrap_multiline("  indented\n    deeper", 50, true);
        assert_eq!(verbatim, vec!["  indented".to_string(), "    deeper".to_string()]);
    }

    #[test]
    fn no_wrap_cuts_long_lines() {
        let verbatim = wrap_multiline("abcdefghij", 4, true);
        assert!(verbatim.iter().all(|l| l.width() <= 4));
        assert_eq!(verbatim.concat(), "abcdefghij");
    }

    #[test]
    fn no_wrap_bubble_keeps_table() {
        let bubble = get_speech_bubble_lines("a  b\nc   d", 50, BubbleStyle::Round, true, false, None);
        let joined = bubble.join("\n");
        assert!(joined.contains("a  b"));
        assert!(joined.contains("c   d"));
    }
}

#[cfg(test)]
mod baguette_tests {
    use super::*;

    fn outer(lines: &[String]) -> usize {
        lines
            .iter()
            .take(lines.len().saturating_sub(3))
            .map(|l| l.width())
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn baguette_matches_user_art() {
        let b = get_speech_bubble_lines(
            "Ut enim tempor twintail\nvocaloid. Nulla qui teto.",
            38,
            BubbleStyle::Baguette,
            false,
            false,
            None,
        );
        let body = &b[..b.len() - 3];
        assert_eq!(
            body,
            &[
                " .-'  /   /   /    /   /   /   /  '-.".to_string(),
                "(                                    )".to_string(),
                "(      Ut enim tempor twintail       )".to_string(),
                "(     vocaloid. Nulla qui teto.      )".to_string(),
                "(                                    )".to_string(),
                " '-.______________________________.-'".to_string(),
            ]
        );
        assert_eq!(&b[b.len() - 3..], &[".".to_string(), "o".to_string(), " O".to_string()]);
    }

    #[test]
    fn baguette_fixed_width_is_exact() {
        let b = get_speech_bubble_lines("hi", 60, BubbleStyle::Baguette, false, false, None);
        assert_eq!(outer(&b), 60);
    }

    #[test]
    fn baguette_bounded_everywhere() {
        for w in [10usize, 20, 50, 200] {
            let b = get_speech_bubble_lines(
                "Hello world this is a test",
                w,
                BubbleStyle::Baguette,
                false,
                false,
                None,
            );
            assert!(outer(&b) <= w, "w={w} outer={}", outer(&b));
        }
    }

    #[test]
    fn baguette_height_budget_fits() {
        let long = "word ".repeat(100);
        let b = get_speech_bubble_lines(
            &long,
            usize::MAX,
            BubbleStyle::Baguette,
            false,
            true,
            Some(8),
        );
        assert!(b.len() <= 8, "got {} lines", b.len());
    }

    #[test]
    fn baguette_auto_is_wider_than_round() {
        let text = "Kasem tetum dolor sit amet consectetur adipiscing elit teto kasane vocaloid utauloid drill twintail chimera mesmerizer tetoris territory miku hatsune baguette french fukkireta override triple baka";
        let baguette = get_speech_bubble_lines(text, usize::MAX, BubbleStyle::Baguette, false, true, None);
        let round = get_speech_bubble_lines(text, usize::MAX, BubbleStyle::Round, false, true, None);
        let bw = outer(&baguette);
        let rw = round.iter().map(|l| l.width()).max().unwrap_or(0);
        assert!(bw > rw, "baguette={bw} round={rw}");
    }
}
