//! Project name generation.
//!
//! GitHub-style `adjective-noun` names, but with more variety and taste:
//! several word patterns, larger themed word lists, and a scoring pass
//! that generates a handful of candidates and keeps the best one
//! (favoring alliteration and a comfortable length).

const ADJECTIVES: &[&str] = &[
    "neon", "retro", "hyper", "chrome", "vapor", "turbo", "cosmic", "laser",
    "midnight", "electric", "crystal", "phantom", "analog", "infinite",
    "atomic", "binary", "blazing", "digital", "dreamy", "endless", "feral",
    "frozen", "golden", "gravity", "hidden", "hollow", "iron", "liquid",
    "lucid", "lunar", "magnetic", "mellow", "mystic", "nebula", "nocturnal",
    "obsidian", "parallel", "plasma", "prism", "quantum", "radiant", "rogue",
    "scarlet", "silent", "solar", "sonic", "spectral", "stellar", "stormy",
    "velvet", "violet", "wild", "zero",
];

const NOUNS: &[&str] = &[
    "drive", "wave", "grid", "runner", "horizon", "pulse", "circuit",
    "mirage", "voltage", "sunset", "vector", "doodle", "cascade", "orbit",
    "arcade", "beacon", "canyon", "cipher", "comet", "current", "dynamo",
    "echo", "ember", "engine", "falcon", "flux", "forge", "fragment",
    "glacier", "harbor", "impulse", "jungle", "lagoon", "matrix", "meteor",
    "monolith", "nomad", "oasis", "outpost", "panther", "phoenix", "pixel",
    "prophet", "reactor", "relay", "signal", "specter", "spire", "synth",
    "tempest", "thunder", "traveler", "vertex", "vortex", "wanderer", "zephyr",
];

const GERUNDS: &[&str] = &[
    "blazing", "chasing", "dancing", "drifting", "falling", "floating",
    "gliding", "glowing", "howling", "racing", "rising", "roaming",
    "shifting", "shimmering", "spinning", "surfing", "wandering", "waning",
];

/// Tiny SplitMix64 PRNG: deterministic per seed, no dependencies.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniformly pick one entry from a word list.
    pub fn pick<'a>(&mut self, list: &[&'a str]) -> &'a str {
        list[(self.next() % list.len() as u64) as usize]
    }
}

/// Name pattern selector: 0 = any, 1 = adjective-noun, 2 = gerund-noun,
/// 3 = noun-noun.
pub const STYLE_NAMES: [&str; 4] = ["any", "adj+noun", "verb+noun", "noun+noun"];

/// Word separators selectable in settings.
pub const SEPARATORS: [char; 3] = ['-', '_', ' '];

/// Generate a project name: builds several candidates and returns the
/// highest-scoring one.
pub fn generate_name(seed: u64, style: u8, separator: char) -> String {
    let mut rng = Rng::new(seed);
    let mut best_score = i32::MIN;
    let mut best = ("", "");
    for _ in 0..6 {
        let (a, b) = candidate(&mut rng, style);
        let score = score(a, b);
        if score > best_score {
            best_score = score;
            best = (a, b);
        }
    }
    format!("{}{}{}", best.0, separator, best.1)
}

/// One random word pair in the requested (or a random) pattern.
fn candidate(rng: &mut Rng, style: u8) -> (&'static str, &'static str) {
    let pattern = match style {
        1..=3 => style as u64,
        // any: adjective-noun most of the time.
        _ => match rng.next() % 10 {
            0..=5 => 1,
            6..=7 => 2,
            _ => 3,
        },
    };
    match pattern {
        1 => (rng.pick(ADJECTIVES), rng.pick(NOUNS)),
        2 => (rng.pick(GERUNDS), rng.pick(NOUNS)),
        _ => (rng.pick(NOUNS), rng.pick(NOUNS)),
    }
}

/// Heuristic quality score for a candidate word pair.
fn score(a: &str, b: &str) -> i32 {
    let mut score = 0;

    // Degenerate pairs ("pixel-pixel") are out.
    if a == b {
        return i32::MIN;
    }
    // Alliteration is the charm GitHub's generator is missing.
    if a.as_bytes().first() == b.as_bytes().first() {
        score += 3;
    }
    // Comfortable to read and type: 9..=16 chars total.
    let len = (a.len() + 1 + b.len()) as i32;
    if (9..=16).contains(&len) {
        score += 2;
    } else {
        score -= (len - 12).abs();
    }
    // Matching endings ("blazing-rising") read as accidental rhyme; avoid.
    if a.len() >= 3 && b.len() >= 3 && a[a.len() - 3..] == b[b.len() - 3..] {
        score -= 2;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_wellformed_and_varied() {
        let mut unique = std::collections::HashSet::new();
        for seed in 0..500u64 {
            let style = (seed % 4) as u8;
            let name = generate_name(seed * 0x9E37_79B9, style, '-');
            let (a, b) = name.split_once('-').expect("two dash-joined words");
            assert!(!a.is_empty() && !b.is_empty(), "bad name: {name}");
            assert_ne!(a, b, "degenerate pair: {name}");
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "unexpected chars: {name}"
            );
            unique.insert(name);
        }
        // 500 seeds should produce plenty of distinct names.
        assert!(unique.len() > 250, "only {} unique names", unique.len());
    }
}
