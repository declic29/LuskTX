//! French proofing using the Hunspell `fr_FR` dictionary of Dicollecte (MPL 2.0), built into the
//! program (`assets/dictionaries/fr`); an installed one (`hunspell::search_dirs`) serves only if
//! the built-in one can't be read. Elisions (`l'homme`, `qu'il`, `aujourd'hui`) are accepted whole or as an elided
//! prefix plus a known word. Suggestions are single edits (accents first) kept when the
//! dictionary knows the result. Grammar is limited to generic repeated-word and spacing checks
//! with French messages; hyphenation uses the bundled French patterns ([`crate::hyphen::Patterns::fr`]).

pub mod grammar;
mod suggest;

use std::sync::{Arc, RwLock};

use crate::affix::AffixDict;

pub use suggest::suggest;

/// The French dictionary: `None` until first use, then the installed one (or an empty one).
static DICT: RwLock<Option<Arc<AffixDict>>> = RwLock::new(None);

/// File names of a French Hunspell dictionary, most specific first (`fr_FR` is what Linux
/// packages and LibreOffice call it; the others are Dicollecte's own file names).
const NAMES: &[&str] = &["fr_FR", "fr-FR", "fr", "fr-toutesvariantes", "fr-classique", "fr-reforme1990"];

/// Elided prefixes that are not dictionary words on their own (`qu'`, `jusqu'`…); a single
/// letter followed by an apostrophe (`l'`, `d'`, `j'`…) is always accepted.
const ELISIONS: &[&str] = &["qu'", "jusqu'", "lorsqu'", "puisqu'", "quoiqu'", "presqu'", "quelqu'", "entr'", "prud'"];

/// The French dictionary: the installed Hunspell `fr_FR` (read on first use), one given with
/// [`install_dictionary`], or an empty one when none is available (French spelling is off then;
/// see [`has_dictionary`]).
pub fn dictionary() -> Arc<AffixDict> {
    if let Some(d) = DICT.read().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
        return d.clone();
    }
    let loaded = Arc::new(load_installed());
    let mut slot = DICT.write().unwrap_or_else(std::sync::PoisonError::into_inner);
    slot.get_or_insert(loaded).clone()
}

/// Lusk Texte : the Dicollecte dictionary (MPL 2.0) ships inside the program, adapted to this
/// reader (`assets/dictionaries/fr/MODIFICATIONS.txt`), so French spelling works on every
/// computer and on the web without installing anything.
const EMBEDDED_AFF: &str = include_str!("../../../../assets/dictionaries/fr/fr_FR.aff");
const EMBEDDED_DIC: &str = include_str!("../../../../assets/dictionaries/fr/fr_FR.dic");

/// The dictionary shipped with the program.
pub fn embedded() -> Result<AffixDict, String> {
    parse_tolerant(EMBEDDED_AFF.to_string(), EMBEDDED_DIC)
}

fn load_installed() -> AffixDict {
    match embedded() {
        Ok(d) if !d.is_empty() => return d,
        Ok(_) => log::warn!("French spelling: the built-in dictionary is empty"),
        Err(e) => log::warn!("French spelling: the built-in dictionary could not be read: {e}"),
    }
    let Some((aff, dic)) = crate::hunspell::find(NAMES) else {
        crate::hunspell::clear_load_error(NAMES[0]);
        log::info!("French spelling: no Hunspell fr_FR dictionary installed");
        return AffixDict::default();
    };
    crate::hunspell::note_load(NAMES[0], &dic, load_tolerant(&aff, &dic)).unwrap_or_else(|e| {
        log::warn!("French spelling: {e}");
        AffixDict::default()
    })
}

/// Load the dictionary; when the loader refuses a directive it doesn't implement (`ICONV` in
/// Dicollecte's file: input conversions such as `’` → `'`, which [`straight`] does itself), read
/// the files again without that directive, as many times as needed. The directives that define
/// the file's format (`SET`, `FLAG`, `PFX`, `SFX`) are never dropped.
fn load_tolerant(aff: &std::path::Path, dic: &std::path::Path) -> Result<AffixDict, String> {
    match crate::hunspell::load(aff, dic) {
        Ok(d) => return Ok(d),
        Err(e) if !e.contains(" is not supported") => return Err(e),
        Err(_) => {}
    }
    let aff_text = std::fs::read_to_string(aff).map_err(|e| format!("{}: {e}", aff.display()))?;
    let dic_text = std::fs::read_to_string(dic).map_err(|e| format!("{}: {e}", dic.display()))?;
    parse_tolerant(aff_text, &dic_text)
}

/// [`load_tolerant`] on the files' text.
fn parse_tolerant(mut aff_text: String, dic_text: &str) -> Result<AffixDict, String> {
    let mut dropped: Vec<String> = Vec::new();
    let mut last = String::new();
    for _ in 0..40 {
        let e = match AffixDict::from_hunspell(&aff_text, dic_text, &[]) {
            Ok(d) => return Ok(d),
            Err(e) => e,
        };
        last = e.clone();
        let directive = e
            .split(" is not supported")
            .next()
            .filter(|_| e.contains(" is not supported"))
            .and_then(|head| head.rsplit(':').next())
            .and_then(|part| part.split_whitespace().next())
            .map(str::to_string);
        let Some(directive) = directive else { return Err(e) };
        if matches!(directive.as_str(), "SET" | "FLAG" | "SFX" | "PFX") {
            return Err(e);
        }
        log::info!("French spelling: ignoring the unsupported {directive} directive of the dictionary");
        dropped.push(directive.clone());
        let cleaned = aff_text.lines().filter(|l| l.split_whitespace().next() != Some(directive.as_str())).collect::<Vec<_>>().join("\n") + "\n";
        if cleaned.len() == aff_text.len() {
            return Err(e); // nothing to drop: the problem isn't a directive line
        }
        aff_text = cleaned;
    }
    Err(format!("too many unsupported directives in the dictionary (ignored: {}; last error: {last})", dropped.join(" ")))
}

/// Use this French dictionary from now on (Hunspell `.aff` and `.dic` text, decoded to UTF-8).
/// On error the current dictionary stays.
pub fn install_dictionary(aff: &str, dic: &str) -> Result<(), String> {
    let d = AffixDict::from_hunspell(aff, dic, &[])?;
    *DICT.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(d));
    Ok(())
}

/// Forget the loaded dictionary, so the next check looks for an installed one again.
pub fn reload_dictionary() {
    *DICT.write().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// The `.dic` file of the French dictionary found in the dictionaries folders now; `None` when
/// there is none.
pub fn installed_path() -> Option<std::path::PathBuf> {
    crate::hunspell::find(NAMES).map(|(_, dic)| dic)
}

/// Is a French dictionary available (spelling is checked)?
pub fn has_dictionary() -> bool {
    !dictionary().is_empty()
}

/// Is `c` a letter of the Latin script?
fn is_latin_letter(c: char) -> bool {
    c.is_alphabetic() && (c < '\u{0250}' || ('\u{1E00}'..='\u{1EFF}').contains(&c))
}

/// The typographic apostrophes written as the straight one the dictionary uses.
fn straight(word: &str) -> String {
    word.chars().map(|c| if c == '’' || c == 'ʼ' { '\'' } else { c }).collect()
}

/// Is `word` spelled correctly in French? Numbers, single letters, short words in capitals,
/// words with digits and words in other scripts are accepted, and so is every word when no
/// French dictionary is installed.
pub fn is_correct(word: &str) -> bool {
    is_correct_with(&dictionary(), word)
}

/// [`is_correct`] with the File › Options › Proofing choices.
pub fn is_correct_with_options(word: &str, opts: &crate::Options) -> bool {
    check(&dictionary(), word, opts)
}

/// [`is_correct`] against a given dictionary (an empty one accepts everything).
pub fn is_correct_with(d: &AffixDict, word: &str) -> bool {
    check(d, word, &crate::Options::default())
}

fn check(d: &AffixDict, word: &str, opts: &crate::Options) -> bool {
    if d.is_empty() {
        return true; // no French dictionary installed: no spelling marks
    }
    let s = straight(word);
    let w = s.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'').trim_matches('\'');
    let n = w.chars().count();
    if n <= 1 || (opts.ignore_numbers && w.chars().any(|c| c.is_ascii_digit())) || !w.chars().any(char::is_alphabetic) {
        return true;
    }
    if opts.ignore_uppercase && w.chars().all(|c| !c.is_lowercase()) && n <= 6 {
        return true;
    }
    if !w.chars().any(is_latin_letter) {
        return true; // other scripts: no dictionary
    }
    if known(d, w) {
        return true;
    }
    // `peut-être`, `grand-mère` are entries; `a-t-il`, `dis-moi`, `vingt-et-un` are checked by parts.
    w.contains('-')
        && w.split('-').all(|p| {
            p.chars().count() <= 1 || (opts.ignore_uppercase && p.chars().all(|c| !c.is_lowercase()) && p.chars().count() <= 6) || known(d, p)
        })
}

/// Is one word in the dictionary, the user dictionary, or an elided prefix and a known word?
fn known(d: &AffixDict, w: &str) -> bool {
    if d.check(w) || crate::in_user_dictionary(&w.to_lowercase()) {
        return true;
    }
    let Some(i) = w.find('\'') else { return false };
    let (left, right) = (w.get(..=i).unwrap_or(""), w.get(i + 1..).unwrap_or(""));
    if right.is_empty() {
        return false;
    }
    let left_ok = left.chars().count() == 2 || ELISIONS.contains(&left.to_lowercase().as_str()) || d.check(left);
    left_ok && (right.chars().count() <= 1 || known(d, right))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lusk Texte : the built-in dictionary reads, knows everyday words in all their forms and
    /// flags misspellings, with nothing installed.
    #[test]
    fn the_built_in_dictionary_checks_french() {
        let d = embedded().unwrap();
        assert!(!d.is_empty());
        for w in ["maison", "maisons", "écrivez", "appartient", "Bretagne", "l'association", "aujourd'hui", "chevaux", "livres"] {
            assert!(is_correct_with(&d, w), "{w}");
        }
        for w in ["maisonn", "écrivé", "apartient", "chevals"] {
            assert!(!is_correct_with(&d, w), "{w}");
        }
    }

    const AFF: &str = "SET UTF-8\n";
    const DIC: &str = "6\nbonjour\nmonde\nhomme\naujourd'hui\npeut\nêtre\n";

    fn dict() -> AffixDict {
        AffixDict::from_hunspell(AFF, DIC, &[]).expect("test dictionary")
    }

    #[test]
    fn french_words_elisions_and_hyphens() {
        let d = dict();
        for w in ["bonjour", "Bonjour", "l'homme", "l’homme", "aujourd'hui", "qu'homme", "peut-être", "a", "2026", "RI"] {
            assert!(is_correct_with(&d, w), "{w}");
        }
        for w in ["bonjuor", "l'hommme", "mondde"] {
            assert!(!is_correct_with(&d, w), "{w}");
        }
    }

    #[test]
    fn no_dictionary_accepts_everything() {
        assert!(is_correct_with(&AffixDict::default(), "bonjuor"));
    }
}
