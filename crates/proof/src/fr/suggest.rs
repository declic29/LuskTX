//! French spelling suggestions: every single edit of the word (accents first, then a missing
//! space, a swapped pair, a missing, extra or wrong letter) that the dictionary knows.

use crate::affix::AffixDict;

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyzàâçéèêëîïôùûü";

/// The accent variants of a lower-case letter (`e` ↔ `é`, `è`, `ê`, `ë`).
fn accent_variants(c: char) -> &'static [char] {
    match c {
        'a' => &['à', 'â'],
        'à' => &['a', 'â'],
        'â' => &['a', 'à'],
        'e' => &['é', 'è', 'ê', 'ë'],
        'é' => &['e', 'è', 'ê'],
        'è' => &['e', 'é', 'ê'],
        'ê' => &['e', 'é', 'è'],
        'ë' => &['e'],
        'i' => &['î', 'ï'],
        'î' => &['i', 'ï'],
        'ï' => &['i', 'î'],
        'o' => &['ô'],
        'ô' => &['o'],
        'u' => &['ù', 'û', 'ü'],
        'ù' => &['u', 'û'],
        'û' => &['u', 'ù'],
        'ü' => &['u'],
        'c' => &['ç'],
        'ç' => &['c'],
        _ => &[],
    }
}

fn with_case(c: char, upper: bool) -> char {
    if upper { c.to_uppercase().next().unwrap_or(c) } else { c }
}

/// Suggestions for a misspelled French `word`, best first, at most `max`.
pub fn suggest(word: &str, max: usize) -> Vec<String> {
    let d = super::dictionary();
    suggest_with(&d, word, max)
}

fn suggest_with(d: &AffixDict, word: &str, max: usize) -> Vec<String> {
    let w: String = word.chars().map(|c| if c == '’' || c == 'ʼ' { '\'' } else { c }).collect();
    let chars: Vec<char> = w.chars().collect();
    let n = chars.len();
    if d.is_empty() || max == 0 || !(2..=30).contains(&n) {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    let add = |candidate: String, out: &mut Vec<String>| {
        if out.len() < max && candidate != w && !out.contains(&candidate) && super::is_correct_with(d, &candidate) {
            out.push(candidate);
        }
    };
    let build = |v: &[char]| -> String { v.iter().collect() };

    // 1. Accents added, removed or changed.
    for (i, &c) in chars.iter().enumerate() {
        let upper = c.is_uppercase();
        let lower = c.to_lowercase().next().unwrap_or(c);
        for &v in accent_variants(lower) {
            let mut t = chars.clone();
            t[i] = with_case(v, upper);
            add(build(&t), &mut out);
        }
    }
    // 2. Two words (`jesuis` → `je suis`).
    for i in 2..n.saturating_sub(1) {
        let (a, b) = (build(&chars[..i]), build(&chars[i..]));
        if super::is_correct_with(d, &a) && super::is_correct_with(d, &b) && !a.ends_with('\'') {
            add(format!("{a} {b}"), &mut out);
        }
    }
    // 3. Two neighbouring letters swapped.
    for i in 0..n - 1 {
        let mut t = chars.clone();
        t.swap(i, i + 1);
        add(build(&t), &mut out);
    }
    // 4. One letter too many.
    for i in 0..n {
        let mut t = chars.clone();
        t.remove(i);
        add(build(&t), &mut out);
    }
    // 5. One letter missing, and 6. one letter wrong.
    for i in 0..=n {
        let upper = i < n && i == 0 && chars[0].is_uppercase();
        for l in LETTERS.chars() {
            let mut t = chars.clone();
            t.insert(i, with_case(l, upper));
            add(build(&t), &mut out);
            if out.len() >= max {
                return out;
            }
        }
    }
    for i in 0..n {
        let upper = chars[i].is_uppercase() && (i == 0 || chars.iter().all(|c| !c.is_lowercase()));
        for l in LETTERS.chars() {
            let mut t = chars.clone();
            t[i] = with_case(l, upper);
            add(build(&t), &mut out);
            if out.len() >= max {
                return out;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_swaps_and_missing_letters() {
        let d = AffixDict::from_hunspell("SET UTF-8\n", "3\ncafé\nbonjour\nsuis\n", &[]).expect("test dictionary");
        assert!(suggest_with(&d, "cafe", 5).contains(&"café".to_string()));
        assert!(suggest_with(&d, "bonjuor", 5).contains(&"bonjour".to_string()));
        assert!(suggest_with(&d, "bonjou", 5).contains(&"bonjour".to_string()));
    }
}
