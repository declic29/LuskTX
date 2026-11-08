//! Lusk Texte : generic checks for Breton text, with French messages (the interface's
//! language): a word written twice, and a double space. Breton follows French typography (a
//! space before `:` `;` `?` `!`), so spaces before punctuation are not flagged.

use crate::{Issue, IssueKind, ProofLang, words};

fn issue(start: usize, end: usize, message: String, suggestions: Vec<String>) -> Issue {
    Issue { start, end, kind: IssueKind::Grammar, message, suggestions, lang: ProofLang::Br }
}

/// The grammar issues of a Breton paragraph.
pub fn check(text: &str) -> Vec<Issue> {
    let mut v = Vec::new();
    for pair in words(text).windows(2) {
        let [(a0, b0), (a1, b1)] = pair else { continue };
        let (Some(w0), Some(w1), Some(gap)) = (text.get(*a0..*b0), text.get(*a1..*b1), text.get(*b0..*a1)) else { continue };
        if !gap.is_empty() && gap.chars().all(char::is_whitespace) && w0.chars().all(char::is_alphabetic) && w0.to_lowercase() == w1.to_lowercase() {
            v.push(issue(*a0, *b1, format!("Mot répété : « {w1} »"), vec![w0.to_string()]));
        }
    }
    for (i, _) in text.match_indices("  ") {
        if i > 0 && text.get(..i).is_some_and(|t| t.ends_with(' ')) {
            continue;
        }
        v.push(issue(i, i + 2, "Double espace".into(), vec![" ".into()]));
    }
    v.sort_by_key(|i| (i.start, i.end));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_words_and_double_spaces() {
        let text = "Demat demat d'an  holl : penaos ?";
        let issues = check(text);
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert_eq!(issues[0].message, "Mot répété : « demat »");
        assert_eq!(issues[1].message, "Double espace");
        assert!(issues.iter().all(|i| i.lang == ProofLang::Br));
        assert!(check("Mont a ran d'ar gêr : ha te ?").is_empty());
    }
}
