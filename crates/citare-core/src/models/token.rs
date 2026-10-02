use std::collections::HashMap;

use super::edition::Edition;

/// Shared data for all token types.
///
/// Note: `Hash` is not derived because `HashMap` does not implement `Hash`.
/// Manual `Hash` is not required here — tokens are compared by value, not used
/// as hash-map keys directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenData {
    pub data: String,
    pub start: usize,
    pub end: usize,
    pub groups: HashMap<String, Option<String>>,
}

/// Citation-specific token data (extends TokenData with edition info).
#[derive(Debug, Clone)]
pub struct CitationTokenData {
    pub token_data: TokenData,
    pub exact_editions: Vec<Edition>,
    pub variation_editions: Vec<Edition>,
    pub short: bool,
}

/// A token produced by the tokenizer during the citation-finding pipeline.
///
/// The tokenizer scans text with Aho-Corasick and produces a mixed list of
/// `Token` values (for recognized patterns) and raw string spans.  The
/// citation finder then inspects each `Token` to build [`Citation`] values.
///
/// [`Citation`]: crate::models::Citation
#[derive(Debug, Clone)]
pub enum Token {
    /// A reporter, law, or journal token — the heart of a citation.
    Citation(CitationTokenData),
    /// An *id.* / *ibid.* token.
    Id(TokenData),
    /// A *supra* token.
    Supra(TokenData),
    /// A stop-word that delimits citation boundaries.
    StopWord(TokenData),
    /// A section symbol (`§`) that may introduce a statutory citation.
    Section(TokenData),
    /// A paragraph break marker.
    Paragraph(TokenData),
    /// A placeholder inserted during HTML cleaning to preserve offsets.
    Placeholder(TokenData),
    /// A bare party-name reference found after a full case citation.
    CaseReference(TokenData),
}

/// Items in a token list are either parsed tokens or raw string spans.
#[derive(Debug, Clone)]
pub enum TokenOrStr {
    Token(Token),
    Str(String),
}

/// A list of token-or-string items representing a tokenised document.
pub type Tokens = Vec<TokenOrStr>;

impl Token {
    /// Return a reference to the inner `TokenData` regardless of variant.
    pub fn token_data(&self) -> &TokenData {
        match self {
            Token::Citation(c) => &c.token_data,
            Token::Id(d)
            | Token::Supra(d)
            | Token::StopWord(d)
            | Token::Section(d)
            | Token::Paragraph(d)
            | Token::Placeholder(d)
            | Token::CaseReference(d) => d,
        }
    }

    pub fn data(&self) -> &str {
        &self.token_data().data
    }

    pub fn start(&self) -> usize {
        self.token_data().start
    }

    pub fn end(&self) -> usize {
        self.token_data().end
    }

    pub fn groups(&self) -> &HashMap<String, Option<String>> {
        &self.token_data().groups
    }

    /// Attempt to merge `other` into `self`.
    ///
    /// Merge is valid when both tokens occupy the same span and carry the same
    /// capture groups.  For `Citation` tokens the `short` flag must also match;
    /// on success their edition lists are combined and deduplicated.
    ///
    /// Returns `true` if the merge succeeded, `false` otherwise.
    pub fn merge(&mut self, other: &Token) -> bool {
        let self_data = self.token_data();
        let other_data = other.token_data();

        if self_data.start != other_data.start
            || self_data.end != other_data.end
            || self_data.groups != other_data.groups
        {
            return false;
        }

        // For non-citation tokens, the variants must be the same kind.
        // Check this first using shared references (no move).
        let both_same_non_citation = matches!(
            (&*self, other),
            (Token::Id(_), Token::Id(_))
                | (Token::Supra(_), Token::Supra(_))
                | (Token::StopWord(_), Token::StopWord(_))
                | (Token::Section(_), Token::Section(_))
                | (Token::Paragraph(_), Token::Paragraph(_))
                | (Token::Placeholder(_), Token::Placeholder(_))
                | (Token::CaseReference(_), Token::CaseReference(_))
        );
        if both_same_non_citation {
            return true;
        }

        // Citation × Citation: merge edition lists.
        if let (Token::Citation(ref mut sc), Token::Citation(oc)) = (self, other) {
            if sc.short != oc.short {
                return false;
            }
            sc.exact_editions.extend(oc.exact_editions.iter().cloned());
            sc.exact_editions
                .sort_by(|a, b| a.short_name.cmp(&b.short_name));
            sc.exact_editions.dedup();

            sc.variation_editions
                .extend(oc.variation_editions.iter().cloned());
            sc.variation_editions
                .sort_by(|a, b| a.short_name.cmp(&b.short_name));
            sc.variation_editions.dedup();
            return true;
        }

        // Mismatched types (e.g. Citation × Id).
        false
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edition::{Edition, Reporter, ReporterSource};

    fn make_token_data(data: &str, start: usize, end: usize) -> TokenData {
        TokenData {
            data: data.to_string(),
            start,
            end,
            groups: HashMap::new(),
        }
    }

    fn make_reporter() -> Reporter {
        Reporter::new(
            "U.S.".to_string(),
            "United States Reports".to_string(),
            "federal".to_string(),
            ReporterSource::Reporters,
        )
    }

    fn make_edition(short_name: &str) -> Edition {
        Edition {
            reporter: make_reporter(),
            short_name: short_name.to_string(),
            start: None,
            end: None,
            harvard_slug: None,
        }
    }

    fn make_citation_token(
        data: &str,
        start: usize,
        end: usize,
        exact: Vec<Edition>,
        short: bool,
    ) -> Token {
        Token::Citation(CitationTokenData {
            token_data: make_token_data(data, start, end),
            exact_editions: exact,
            variation_editions: vec![],
            short,
        })
    }

    // --- basic accessors ---

    #[test]
    fn test_id_token_accessors() {
        let t = Token::Id(make_token_data("id.", 5, 8));
        assert_eq!(t.data(), "id.");
        assert_eq!(t.start(), 5);
        assert_eq!(t.end(), 8);
        assert!(t.groups().is_empty());
    }

    #[test]
    fn test_display() {
        let t = Token::StopWord(make_token_data("See", 0, 3));
        assert_eq!(t.to_string(), "See");
    }

    // --- merge ---

    #[test]
    fn test_merge_citation_same_position_succeeds() {
        let e1 = make_edition("U.S.");
        let e2 = make_edition("U.S.2d");
        let mut t1 = make_citation_token("1 U.S. 2", 0, 8, vec![e1.clone()], false);
        let t2 = make_citation_token("1 U.S. 2", 0, 8, vec![e2.clone()], false);
        assert!(t1.merge(&t2));
        if let Token::Citation(ref c) = t1 {
            assert_eq!(c.exact_editions.len(), 2);
        } else {
            panic!("expected Citation");
        }
    }

    #[test]
    fn test_merge_citation_different_position_fails() {
        let mut t1 = make_citation_token("1 U.S. 2", 0, 8, vec![], false);
        let t2 = make_citation_token("1 U.S. 2", 10, 18, vec![], false);
        assert!(!t1.merge(&t2));
    }

    #[test]
    fn test_merge_citation_short_mismatch_fails() {
        let mut t1 = make_citation_token("1 U.S. 2", 0, 8, vec![], false);
        let t2 = make_citation_token("1 U.S. 2", 0, 8, vec![], true);
        assert!(!t1.merge(&t2));
    }

    #[test]
    fn test_merge_same_non_citation_type_succeeds() {
        let mut t1 = Token::Id(make_token_data("id.", 0, 3));
        let t2 = Token::Id(make_token_data("id.", 0, 3));
        assert!(t1.merge(&t2));
    }

    #[test]
    fn test_merge_different_non_citation_types_fails() {
        let mut t1 = Token::Id(make_token_data("id.", 0, 3));
        let t2 = Token::Supra(make_token_data("id.", 0, 3));
        assert!(!t1.merge(&t2));
    }

    #[test]
    fn test_merge_citation_deduplicates_editions() {
        let e = make_edition("U.S.");
        let mut t1 = make_citation_token("x", 0, 1, vec![e.clone()], false);
        let t2 = make_citation_token("x", 0, 1, vec![e.clone()], false);
        assert!(t1.merge(&t2));
        if let Token::Citation(ref c) = t1 {
            assert_eq!(c.exact_editions.len(), 1);
        }
    }
}
