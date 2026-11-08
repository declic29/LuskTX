//! Lusk Texte : Breton proofing (`br`, `br-FR`) with An Drouizig's Hunspell `br_FR` dictionary
//! (used under MPL 1.1), built into the program from `assets/dictionaries/br` (converted to
//! one-character flags by `outils/convertir-dictionnaire-breton.py`). Words with the `c'h`
//! trigraph and elided forms (`d'ar`, `n'eo`) are dictionary words. Suggestions are the
//! dictionary-driven ones Czech uses; grammar is limited to repeated words and double spaces,
//! with French messages (the interface's language). Hyphenation keeps the fallback patterns.

pub mod grammar;

use std::sync::{Arc, RwLock};

use crate::affix::AffixDict;

/// The Breton dictionary: `None` until first use.
static DICT: RwLock<Option<Arc<AffixDict>>> = RwLock::new(None);

/// File names of a Breton Hunspell dictionary, for one installed in the dictionaries folders.
const NAMES: &[&str] = &["br_FR", "br-FR", "br"];

const EMBEDDED_AFF: &str = include_str!("../../../../assets/dictionaries/br/br_FR.aff");
const EMBEDDED_DIC: &str = include_str!("../../../../assets/dictionaries/br/br_FR.dic");

/// The dictionary shipped with the program.
pub fn embedded() -> Result<AffixDict, String> {
    AffixDict::from_hunspell(EMBEDDED_AFF, EMBEDDED_DIC, &[])
}

/// The Breton dictionary: the built-in one, or an installed `br_FR` if it can't be read.
pub fn dictionary() -> Arc<AffixDict> {
    if let Some(d) = DICT.read().unwrap_or_else(std::sync::PoisonError::into_inner).as_ref() {
        return d.clone();
    }
    let loaded = Arc::new(load());
    let mut slot = DICT.write().unwrap_or_else(std::sync::PoisonError::into_inner);
    slot.get_or_insert(loaded).clone()
}

fn load() -> AffixDict {
    match embedded() {
        Ok(d) if !d.is_empty() => return d,
        Ok(_) => log::warn!("Breton spelling: the built-in dictionary is empty"),
        Err(e) => log::warn!("Breton spelling: the built-in dictionary could not be read: {e}"),
    }
    let Some((aff, dic)) = crate::hunspell::find(NAMES) else {
        crate::hunspell::clear_load_error(NAMES[0]);
        return AffixDict::default();
    };
    crate::hunspell::note_load(NAMES[0], &dic, crate::hunspell::load(&aff, &dic)).unwrap_or_else(|e| {
        log::warn!("Breton spelling: {e}");
        AffixDict::default()
    })
}

/// Use this Breton dictionary from now on (Hunspell `.aff` and `.dic` text).
pub fn install_dictionary(aff: &str, dic: &str) -> Result<(), String> {
    let d = AffixDict::from_hunspell(aff, dic, &[])?;
    *DICT.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(d));
    Ok(())
}

/// Forget the loaded dictionary, so the next check reads it again.
pub fn reload_dictionary() {
    *DICT.write().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
}

/// The `.dic` file of a Breton dictionary installed in the dictionaries folders, if any (the
/// built-in one is used first).
pub fn installed_path() -> Option<std::path::PathBuf> {
    crate::hunspell::find(NAMES).map(|(_, dic)| dic)
}

/// Is a Breton dictionary available (spelling is checked)?
pub fn has_dictionary() -> bool {
    !dictionary().is_empty()
}

/// Is `c` a letter of the Latin script?
fn is_latin_letter(c: char) -> bool {
    c.is_alphabetic() && (c < '\u{0250}' || ('\u{1E00}'..='\u{1EFF}').contains(&c))
}

/// The typographic apostrophes written as the straight one the dictionary uses (`c’h` → `c'h`).
fn straight(word: &str) -> String {
    word.chars().map(|c| if c == '’' || c == 'ʼ' { '\'' } else { c }).collect()
}

/// Is `word` spelled correctly in Breton?
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
        return true;
    }
    let w = straight(word);
    let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'');
    let w = w.trim_end_matches('\'');
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
    // Compounds written with a hyphen (`kreiz-kêr`): each part known.
    w.contains('-') && w.split('-').all(|p| p.chars().count() <= 1 || known(d, p))
}

fn known(d: &AffixDict, w: &str) -> bool {
    d.check(w) || crate::in_user_dictionary(&w.to_lowercase())
}

/// Spelling suggestions for a Breton word, best first, at most `max`.
pub fn suggest(word: &str, max: usize) -> Vec<String> {
    crate::cs::suggest::suggest_in(&dictionary(), &straight(word), max)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in dictionary reads and knows everyday Breton, mutated and inflected forms
    /// included, and flags misspellings.
    #[test]
    fn the_built_in_dictionary_checks_breton() {
        let d = embedded().unwrap();
        assert!(!d.is_empty());
        for w in ["brezhoneg", "Breizh", "ti", "tiez", "c'hoari", "c’hoari", "levr", "levrioù", "skrivañ", "bugale", "kenavo", "demat"] {
            assert!(is_correct_with(&d, w), "{w}");
        }
        for w in ["brezhonegg", "skrivann", "levrouz"] {
            assert!(!is_correct_with(&d, w), "{w}");
        }
    }
}
