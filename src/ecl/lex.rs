// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tokeniser for the supported ECL subset. See `spec/ecl.md` §5.
//!
//! Whitespace is skipped. `|term|` annotations after a concept id are consumed
//! and discarded (they carry no semantics). Constructs outside the slice 1
//! grammar (cardinality `[`, dotted `.`, etc.) produce a clear error rather
//! than being silently mis-tokenised.

use anyhow::{bail, Result};

/// A lexical token, paired with the source character offset where it starts
/// (for error messages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    DescOrSelf, // <<
    Desc,       // <
    AncOrSelf,  // >>
    Anc,        // >
    Child,      // <!
    Parent,     // >!
    Member,     // ^
    Star,       // *
    Colon,      // :
    Comma,      // ,
    Eq,         // =
    NotEq,      // !=
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    Sctid(String),
    And,
    Or,
    Minus,
}

/// A token plus its starting character position.
#[derive(Debug, Clone)]
pub struct Spanned {
    pub tok: Tok,
    pub pos: usize,
}

/// Tokenise an ECL string.
pub fn lex(input: &str) -> Result<Vec<Spanned>> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let peek = |o: usize| chars.get(i + o).copied();

        macro_rules! push {
            ($t:expr, $adv:expr) => {{
                out.push(Spanned {
                    tok: $t,
                    pos: start,
                });
                i += $adv;
            }};
        }

        match c {
            '<' => match peek(1) {
                Some('<') => push!(Tok::DescOrSelf, 2),
                Some('!') => push!(Tok::Child, 2),
                _ => push!(Tok::Desc, 1),
            },
            '>' => match peek(1) {
                Some('>') => push!(Tok::AncOrSelf, 2),
                Some('!') => push!(Tok::Parent, 2),
                _ => push!(Tok::Anc, 1),
            },
            '^' => push!(Tok::Member, 1),
            '*' => push!(Tok::Star, 1),
            ':' => push!(Tok::Colon, 1),
            ',' => push!(Tok::Comma, 1),
            '=' => push!(Tok::Eq, 1),
            '!' => match peek(1) {
                Some('=') => push!(Tok::NotEq, 2),
                _ => bail!("unexpected '!' at position {start} (expected '!=')"),
            },
            '(' => push!(Tok::LParen, 1),
            ')' => push!(Tok::RParen, 1),
            '{' => push!(Tok::LBrace, 1),
            '}' => push!(Tok::RBrace, 1),
            '|' => {
                // Consume a `|term|` annotation and discard it.
                i += 1;
                while i < chars.len() && chars[i] != '|' {
                    i += 1;
                }
                if i >= chars.len() {
                    bail!("unterminated |term| annotation starting at position {start}");
                }
                i += 1; // closing '|'
            }
            '0'..='9' => {
                let mut j = i;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let s: String = chars[i..j].iter().collect();
                out.push(Spanned {
                    tok: Tok::Sctid(s),
                    pos: start,
                });
                i = j;
            }
            c if c.is_ascii_alphabetic() => {
                let mut j = i;
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                let word: String = chars[i..j].iter().collect();
                let upper = word.to_ascii_uppercase();
                let tok = match upper.as_str() {
                    "AND" => Tok::And,
                    "OR" => Tok::Or,
                    "MINUS" => Tok::Minus,
                    other => bail!(
                        "unsupported ECL keyword {other:?} at position {start} \
                         (supported: AND, OR, MINUS; reverse/dotted attributes are not yet implemented)"
                    ),
                };
                out.push(Spanned { tok, pos: start });
                i = j;
            }
            '[' | ']' => {
                bail!("attribute cardinality ('[..]') is not yet supported (position {start})")
            }
            '.' => bail!("dotted attributes ('.') are not yet supported (position {start})"),
            other => bail!("unexpected character {other:?} at position {start}"),
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        lex(s).unwrap().into_iter().map(|s| s.tok).collect()
    }

    #[test]
    fn operators_longest_match() {
        assert_eq!(
            toks("<<73211009"),
            vec![Tok::DescOrSelf, Tok::Sctid("73211009".into())]
        );
        assert_eq!(toks("<! 1"), vec![Tok::Child, Tok::Sctid("1".into())]);
        assert_eq!(toks(">>2"), vec![Tok::AncOrSelf, Tok::Sctid("2".into())]);
        assert_eq!(toks(">!2"), vec![Tok::Parent, Tok::Sctid("2".into())]);
        assert_eq!(
            toks("^447562003"),
            vec![Tok::Member, Tok::Sctid("447562003".into())]
        );
    }

    #[test]
    fn discards_term_annotation() {
        assert_eq!(
            toks("73211009 |Diabetes mellitus|"),
            vec![Tok::Sctid("73211009".into())]
        );
    }

    #[test]
    fn keywords_case_insensitive() {
        assert_eq!(
            toks("1 or 2"),
            vec![Tok::Sctid("1".into()), Tok::Or, Tok::Sctid("2".into())]
        );
        assert_eq!(
            toks("1 MINUS 2"),
            vec![Tok::Sctid("1".into()), Tok::Minus, Tok::Sctid("2".into())]
        );
    }

    #[test]
    fn refinement_tokens() {
        assert_eq!(
            toks("1:2=<<3"),
            vec![
                Tok::Sctid("1".into()),
                Tok::Colon,
                Tok::Sctid("2".into()),
                Tok::Eq,
                Tok::DescOrSelf,
                Tok::Sctid("3".into()),
            ]
        );
    }

    #[test]
    fn unsupported_constructs_error() {
        assert!(lex("1:2=3 [0..1]").is_err()); // cardinality
        assert!(lex("1.2").is_err()); // dotted
        assert!(lex("R 1").is_err()); // reverse keyword
        assert!(lex("73211009 |unterminated").is_err());
    }
}
