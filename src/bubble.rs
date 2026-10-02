use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub const MAX_BUBBLE_WIDTH: usize = 50;
pub const MIN_BUBBLE_WIDTH: usize = 20;

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

pub fn get_speech_bubble_lines(text: &str, max_width: usize) -> Vec<String> {
    // max_width is the total outer width including borders.
    let effective_max = max_width.clamp(10, 200);
    let inner_max = effective_max.saturating_sub(2).max(6);
    let lower = MIN_BUBBLE_WIDTH.min(inner_max);

    fn wrap_multiline(text: &str, width: usize) -> Vec<String> {
        let mut out = Vec::new();
        // "".lines() yields nothing, which correctly maps to an empty bubble.
        for paragraph in text.lines() {
            if paragraph.trim().is_empty() {
                out.push(String::new());
            } else {
                out.extend(wrap_text(paragraph, width));
            }
        }
        // Single-line text without '\n' (the common case): lines() yields
        // exactly one item, but handle text without any newline uniformly.
        if out.is_empty() && !text.is_empty() {
            out.extend(wrap_text(text, width));
        }
        // Drop leading/trailing blank separators, keep interior ones.
        while out.first().is_some_and(|l| l.is_empty()) && out.len() > 1 {
            out.remove(0);
        }
        while out.last().is_some_and(|l| l.is_empty()) && out.len() > 1 {
            out.pop();
        }
        out
    }

    let temp_lines = wrap_multiline(text, inner_max.saturating_sub(4));
    let content_width = temp_lines
        .iter()
        .map(|line| line.width())
        .max()
        .unwrap_or(0);
    let bubble_width = (content_width + 4).clamp(lower, inner_max);

    let lines = wrap_multiline(text, bubble_width.saturating_sub(4));
    let mut bubble_lines = Vec::new();

    bubble_lines.push(format!("╭{}╮", "─".repeat(bubble_width)));

    if lines.is_empty() {
        bubble_lines.push(format!("│{}│", " ".repeat(bubble_width)));
    } else {
        for line in &lines {
            let line_display_width = line.width();
            let padding_needed = (bubble_width - 2).saturating_sub(line_display_width);
            let left_padding = padding_needed / 2;
            let right_padding = padding_needed - left_padding;
            bubble_lines.push(format!(
                "│ {}{}{} │",
                " ".repeat(left_padding),
                line,
                " ".repeat(right_padding)
            ));
        }
    }

    bubble_lines.push(format!("╰{}╯", "─".repeat(bubble_width)));
    bubble_lines.push("\\".to_string());
    bubble_lines.push(" \\".to_string());

    bubble_lines
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
        let bubble = get_speech_bubble_lines("Hello Teto!", MAX_BUBBLE_WIDTH);
        let border_width = bubble[0].width();
        for line in &bubble[..bubble.len() - 2] {
            assert_eq!(line.width(), border_width);
        }
    }

    #[test]
    fn bubble_empty() {
        let bubble = get_speech_bubble_lines("", MAX_BUBBLE_WIDTH);
        assert_eq!(bubble.len(), 5);
    }

    #[test]
    fn bubble_respects_custom_width() {
        let text = "Hello world this is a width test for the bubble";
        let narrow = get_speech_bubble_lines(text, 30);
        let wide = get_speech_bubble_lines(text, 60);
        assert!(narrow[0].width() <= 30);
        assert!(wide[0].width() > narrow[0].width());
        assert!(wide[0].width() <= 60);
    }

    #[test]
    fn bubble_preserves_newlines() {
        let bubble = get_speech_bubble_lines("line1\nline2\nline3", 50);
        let joined = bubble.join("\n");
        assert!(joined.contains("line1"));
        assert!(joined.contains("line2"));
        assert!(joined.contains("line3"));
        // Each source line lands on its own bubble row.
        let rows: Vec<&String> = bubble.iter().filter(|l| l.contains("line")).collect();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn bubble_outer_width_bounded() {
        for w in [10usize, 20, 50, 200] {
            let bubble = get_speech_bubble_lines("Hello world this is a test", w);
            assert!(
                bubble[0].width() <= w,
                "outer width {} exceeds max {w}",
                bubble[0].width()
            );
        }
    }
}
