//! Generic repeated-word and spacing checks with French messages. No French-specific grammar
//! rules are inferred. French writes a (non-breaking) space before `:`, `;`, `!` and `?`, so
//! those are never flagged; a space before a comma or a full stop is.

use crate::{Issue, IssueKind, ProofLang, words};

/// Words that can legitimately appear twice in a row (`nous nous sommes`, `vous vous`, `très très`).
const DOUBLES: &[&str] = &["nous", "vous", "si", "plus", "bien", "très", "tout", "loin", "vite", "coup", "pas", "peu", "ça"];

fn issue(start: usize, end: usize, message: String, suggestions: Vec<String>) -> Issue {
    Issue { start, end, kind: IssueKind::Grammar, message, suggestions, lang: ProofLang::Fr }
}

pub fn check(text: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    repeated_words(text, &words(text), &mut issues);
    spaces(text, &mut issues);
    issues.sort_by_key(|i| (i.start, i.end));
    issues
}

fn repeated_words(text: &str, ws: &[(usize, usize)], v: &mut Vec<Issue>) {
    for pair in ws.windows(2) {
        let [(a0, b0), (a1, b1)] = pair else { continue };
        let (Some(w0), Some(w1), Some(gap)) = (text.get(*a0..*b0), text.get(*a1..*b1), text.get(*b0..*a1)) else { continue };
        let lower = w0.to_lowercase();
        if !gap.is_empty()
            && gap.chars().all(char::is_whitespace)
            && w0.chars().count() >= 2
            && w0.chars().all(char::is_alphabetic)
            && lower == w1.to_lowercase()
            && !DOUBLES.contains(&lower.as_str())
        {
            v.push(issue(*a0, *b1, format!("Mot répété : « {w1} »"), vec![w0.to_string()]));
        }
    }
}

fn spaces(text: &str, v: &mut Vec<Issue>) {
    for (i, c) in text.char_indices() {
        if c != ' ' {
            continue;
        }
        let rest = text.get(i + 1..).unwrap_or("");
        let mut it = rest.chars();
        let Some(p) = it.next() else { continue };
        let after = it.next();
        let flag = match p {
            ',' => true,
            // ` ...` is an ellipsis, ` .5` a number.
            '.' => !matches!(after, Some('.')) && !after.is_some_and(|a| a.is_ascii_digit()),
            _ => false,
        };
        if flag {
            v.push(issue(i, i + 1 + p.len_utf8(), "Espace avant un signe de ponctuation".into(), vec![p.to_string()]));
        }
    }
    for (i, _) in text.match_indices("  ") {
        if i > 0 && text.get(..i).is_some_and(|t| t.ends_with(' ')) {
            continue;
        }
        v.push(issue(i, i + 2, "Espace en double".into(), vec![" ".into()]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_words_and_spaces_have_french_messages() {
        let issues = check("Le le chat ,  dort.");
        assert!(issues.iter().any(|i| i.message == "Mot répété : « le »"));
        assert!(issues.iter().any(|i| i.message == "Espace avant un signe de ponctuation"));
        assert!(issues.iter().any(|i| i.message == "Espace en double"));
        for text in ["Nous nous sommes vus.", "Quoi ? Rien : tout !", "Il dit : « Bonjour ; salut ».", "Et puis... voilà", "3 .5"] {
            assert!(check(text).iter().all(|i| i.message != "Mot répété : « nous »"), "{text}");
        }
        assert!(check("Quoi ? Rien : tout !").is_empty());
    }
}
