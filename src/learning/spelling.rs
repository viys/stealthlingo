//! Answer checking for spelling and listening-spelling questions.

/// Normalizes an answer: trims, collapses inner whitespace, ignores case and
/// treats typographic apostrophes as ASCII. Hyphens and apostrophes are kept
/// because they are part of the spelling.
pub fn normalize_answer(answer: &str) -> String {
    answer
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['\u{2019}', '\u{2018}'], "'")
        .to_lowercase()
}

pub fn is_correct_spelling(expected: &str, submitted: &str) -> bool {
    let submitted = normalize_answer(submitted);
    !submitted.is_empty() && submitted == normalize_answer(expected)
}

/// Characters paired with whether they match the other spelling.
pub type Marked = Vec<(char, bool)>;

/// Aligns a submitted spelling with the expected one, ignoring case. Returns
/// both as `(char, matched)` pairs: unmatched expected letters were missed,
/// unmatched submitted letters are extra or wrong.
pub fn diff_chars(expected: &str, submitted: &str) -> (Marked, Marked) {
    let a: Vec<char> = expected.trim().chars().collect();
    let b: Vec<char> = submitted.trim().chars().collect();
    let same = |x: char, y: char| x.to_lowercase().eq(y.to_lowercase());
    // Longest common subsequence table, filled from the end.
    let mut lcs = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if same(a[i], b[j]) {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut matched_a = vec![false; a.len()];
    let mut matched_b = vec![false; b.len()];
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if same(a[i], b[j]) {
            matched_a[i] = true;
            matched_b[j] = true;
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    (
        a.into_iter().zip(matched_a).collect(),
        b.into_iter().zip(matched_b).collect(),
    )
}

/// Replaces occurrences of `word` in `text` (case-insensitively) with a blank,
/// so definitions and examples do not give the answer away.
pub fn mask_word(text: &str, word: &str) -> String {
    let word = word.trim();
    if word.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_word = word.to_lowercase();
    // Byte offsets are only comparable when lowercasing keeps lengths intact.
    if lower_text.len() != text.len() || lower_word.len() != word.len() {
        return text.to_string();
    }
    let blank = "_".repeat(word.chars().count().clamp(3, 12));
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (start, _) in lower_text.match_indices(&lower_word) {
        if start < last {
            continue;
        }
        out.push_str(&text[last..start]);
        out.push_str(&blank);
        last = start + word.len();
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_case_and_surrounding_whitespace() {
        assert!(is_correct_spelling("Ephemeral", "  ephemeral \n"));
        assert!(is_correct_spelling("ephemeral", "EPHEMERAL"));
    }

    #[test]
    fn collapses_inner_whitespace() {
        assert!(is_correct_spelling("ice cream", "ice   cream"));
        assert!(!is_correct_spelling("ice cream", "icecream"));
    }

    #[test]
    fn keeps_apostrophes_and_hyphens_significant() {
        assert!(is_correct_spelling("o'clock", "O'clock"));
        assert!(is_correct_spelling("o'clock", "o\u{2019}clock"));
        assert!(!is_correct_spelling("o'clock", "oclock"));
        assert!(is_correct_spelling("well-being", "Well-Being"));
        assert!(!is_correct_spelling("well-being", "wellbeing"));
        assert!(!is_correct_spelling("well-being", "well being"));
    }

    #[test]
    fn empty_answer_is_never_correct() {
        assert!(!is_correct_spelling("a", "   "));
    }

    #[test]
    fn rejects_misspellings() {
        assert!(!is_correct_spelling("ephemeral", "ephemerall"));
        assert!(!is_correct_spelling("ephemeral", "ephemera"));
    }

    fn unmatched(pairs: &[(char, bool)]) -> String {
        pairs.iter().filter(|(_, m)| !m).map(|(c, _)| *c).collect()
    }

    #[test]
    fn diff_marks_missing_and_extra_letters() {
        let (expected, submitted) = diff_chars("ephemeral", "Ephemrall");
        assert_eq!(unmatched(&expected), "e");
        assert_eq!(unmatched(&submitted), "l");
        let (expected, submitted) = diff_chars("cat", "cat");
        assert!(expected.iter().chain(&submitted).all(|(_, m)| *m));
        let (expected, submitted) = diff_chars("cat", "");
        assert_eq!(unmatched(&expected), "cat");
        assert!(submitted.is_empty());
    }

    #[test]
    fn masks_word_case_insensitively() {
        assert_eq!(
            mask_word("Ephemeral things are ephemeral.", "ephemeral"),
            "_________ things are _________."
        );
        assert_eq!(mask_word("nothing here", "word"), "nothing here");
    }
}
