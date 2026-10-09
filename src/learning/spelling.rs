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

    #[test]
    fn masks_word_case_insensitively() {
        assert_eq!(
            mask_word("Ephemeral things are ephemeral.", "ephemeral"),
            "_________ things are _________."
        );
        assert_eq!(mask_word("nothing here", "word"), "nothing here");
    }
}
