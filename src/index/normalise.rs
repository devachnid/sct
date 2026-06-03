//! Term normalisation and semantic-tag handling for the FST index.
//!
//! Normalisation is deliberately *lossless* with respect to accents and
//! punctuation — see `specs/fst.md` §7. We accept a larger index in exchange
//! for not collapsing clinically distinct terms. The transform is fixed and
//! MUST stay stable across releases: changing it silently would invalidate any
//! stored queries downstream.
//!
//! The steps are:
//!   1. NFC-normalise (composes equivalent encodings; does NOT strip accents)
//!   2. Unicode lowercase
//!   3. Collapse internal whitespace to a single space, trim
//!
//! Stripping the trailing semantic tag from an FSN is a *separate* step
//! ([`split_semantic_tag`]); the caller does it before normalising, because the
//! tag is recorded in the FST value rather than the key.

use unicode_normalization::UnicodeNormalization;

/// Normalise a term for indexing or lookup. Idempotent: `normalise(normalise(x)) == normalise(x)`.
pub fn normalise(term: &str) -> String {
    let lowered = term.nfc().collect::<String>().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut pending_space = false;
    for ch in lowered.chars() {
        if ch.is_whitespace() {
            // Defer emitting a separator until we know a non-space follows, so
            // leading/trailing/runs of whitespace all collapse cleanly.
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(ch);
        }
    }
    out
}

/// Split an FSN into `(term_without_tag, Some(tag))`.
///
/// `"Myocardial infarction (disorder)"` → `("Myocardial infarction", Some("disorder"))`.
/// Returns `(input, None)` when there is no trailing parenthesised tag.
///
/// Only the *final* parenthesised group is treated as the tag, matching how
/// `builder::strip_semantic_tag` already works elsewhere in `sct`.
pub fn split_semantic_tag(fsn: &str) -> (&str, Option<&str>) {
    let trimmed = fsn.trim_end();
    if !trimmed.ends_with(')') {
        return (fsn, None);
    }
    match trimmed.rfind(" (") {
        Some(open) => {
            let tag = &trimmed[open + 2..trimmed.len() - 1];
            let term = trimmed[..open].trim_end();
            (term, Some(tag))
        }
        None => (fsn, None),
    }
}

/// Split a normalised term into word tokens for the word-level index.
///
/// Whitespace-tokenised only. Consistent with the no-punctuation-stripping
/// decision: a token may carry adjacent punctuation (rare once FSN tags are
/// removed). Empty tokens are skipped.
pub fn tokenise(normalised: &str) -> impl Iterator<Item = &str> {
    normalised.split(' ').filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_collapses_whitespace() {
        assert_eq!(normalise("  Heart   Attack \t"), "heart attack");
        assert_eq!(normalise("Myocardial\nInfarction"), "myocardial infarction");
    }

    #[test]
    fn preserves_accents_and_punctuation() {
        // No diacritic folding, no punctuation stripping.
        assert_eq!(normalise("Ménière's disease"), "ménière's disease");
        assert_eq!(normalise("type-2 (x), y"), "type-2 (x), y");
    }

    #[test]
    fn idempotent() {
        let once = normalise("  Côte  d'Azur  ");
        assert_eq!(once, normalise(&once));
    }

    #[test]
    fn splits_trailing_semantic_tag() {
        assert_eq!(
            split_semantic_tag("Myocardial infarction (disorder)"),
            ("Myocardial infarction", Some("disorder"))
        );
        assert_eq!(
            split_semantic_tag("Entire left femur (body structure)"),
            ("Entire left femur", Some("body structure"))
        );
    }

    #[test]
    fn no_tag_returns_input_untouched() {
        assert_eq!(split_semantic_tag("Heart attack"), ("Heart attack", None));
        // A trailing paren that is part of the term, not a tag-style suffix,
        // is still treated as the final group — acceptable; FSNs always end
        // in a real tag, and synonyms are never passed through this function.
        assert_eq!(split_semantic_tag("foo"), ("foo", None));
    }

    #[test]
    fn tokenises_on_whitespace() {
        let toks: Vec<_> = tokenise("fracture of left femur").collect();
        assert_eq!(toks, vec!["fracture", "of", "left", "femur"]);
    }
}
