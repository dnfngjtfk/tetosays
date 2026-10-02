const WORDS: &[&str] = &[
    "kasem", "tetum", "tetorum", "kasanum", "baguetum", "drillum", "chimerae",

    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",

    "teto", "kasane",
    "vocaloid", "utauloid", "drill", "twintail", "chimera", "mesmerizer",
    "tetoris", "territory", "miku", "hatsune", "baguette", "french",
    "fukkireta", "override", "triple", "baka", "pearto", "obsolete", "meat",
    "synthv", "voicepeak", "vipperloid", "mayo", "larp", "ahoge", "tsundere",
    "igaku", "medicine", "hitomania", "neru", "akita", "machine", "love",
    "twindrill",
];

fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn next_range(state: &mut u64, bound: u64) -> u64 {
    if bound == 0 {
        return 0;
    }
    next_u64(state) % bound
}

fn seed() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x243F6A8885A308D3);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    nanos
        .wrapping_add(count.wrapping_mul(0x9E3779B97F4A7C15))
        .wrapping_add(((std::process::id() as u64) << 32) | 0xD1B54A32)
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn generate_seeded(word_count: usize, mut state: u64) -> String {
    let n = word_count.clamp(1, 500);
    let mut out = String::new();
    let mut remaining = n;
    let mut sentence_left = 4 + next_range(&mut state, 7) as usize;
    let mut first_in_sentence = true;
    if remaining >= 2 && next_range(&mut state, 3) == 0 {
        out.push_str("Kasem tetum");
        remaining -= 2;
        sentence_left -= 2;
        first_in_sentence = false;
        if remaining == 0 {
            out.push('.');
            return out;
        }
        if next_range(&mut state, 100) < 12 {
            out.push(',');
        }
        out.push(' ');
    }
    while remaining > 0 {
        let w = WORDS[next_range(&mut state, WORDS.len() as u64) as usize];
        let word = if first_in_sentence {
            capitalize(w)
        } else {
            w.to_string()
        };
        out.push_str(&word);
        remaining -= 1;
        sentence_left -= 1;
        if remaining == 0 {
            out.push('.');
        } else if sentence_left == 0 {
            out.push_str(". ");
            sentence_left = (4 + next_range(&mut state, 7)) as usize;
            first_in_sentence = true;
        } else {
            if next_range(&mut state, 100) < 12 {
                out.push(',');
            }
            out.push(' ');
            first_in_sentence = false;
        }
    }
    out
}

pub fn generate(word_count: usize) -> String {
    generate_seeded(word_count, seed())
}

pub fn generate_default() -> String {
    let mut state = seed();
    let n = 14 + next_range(&mut state, 22) as usize;
    generate_seeded(n, next_u64(&mut state))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_words(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn word_count_matches() {
        for n in [1usize, 5, 30, 100] {
            assert_eq!(count_words(&generate_seeded(n, 42)), n);
        }
    }

    #[test]
    fn starts_upper_ends_period() {
        let s = generate_seeded(20, 7);
        let first = s.chars().next().unwrap();
        assert!(first.is_uppercase());
        assert!(s.ends_with('.'));
    }

    #[test]
    fn same_seed_same_text() {
        assert_eq!(generate_seeded(25, 123), generate_seeded(25, 123));
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(generate_seeded(25, 1), generate_seeded(25, 2));
    }

    #[test]
    fn opener_hits_about_third() {
        let mut hits = 0;
        for seed in 0..300u64 {
            if generate_seeded(10, seed).starts_with("Kasem tetum") {
                hits += 1;
            }
        }
        assert!((60..140).contains(&hits), "hits={hits}");
    }

    #[test]
    fn opener_keeps_word_count() {
        for seed in 0..50u64 {
            for n in [2usize, 3, 10, 30] {
                assert_eq!(count_words(&generate_seeded(n, seed)), n);
            }
        }
    }
    #[test]
    fn clamped_to_range() {
        assert_eq!(count_words(&generate_seeded(0, 9)), 1);
        assert_eq!(count_words(&generate_seeded(10000, 9)), 500);
    }

    #[test]
    fn default_is_sane() {
        let s = generate_default();
        let n = count_words(&s);
        assert!((14..36).contains(&n));
        assert!(s.ends_with('.'));
    }
}
