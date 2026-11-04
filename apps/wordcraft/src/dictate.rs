//! Lusk Texte : Accueil › Voix › Dicter. Sous Windows, la dictée passe par la saisie vocale du
//! système (le raccourci Windows + H) : elle écrit dans le document à l'emplacement du curseur, en
//! français si Windows est en français, et le même raccourci l'arrête.

/// Ouvre la saisie vocale de Windows en envoyant Windows + H.
#[cfg(windows)]
pub fn start() -> Result<(), String> {
    use enigo::Direction::{Click, Press, Release};
    use enigo::{Enigo, Key, Keyboard, Settings};
    let fail = |e: &dyn std::fmt::Display| format!("La saisie vocale de Windows n'a pas pu être ouverte ({e}). Essayez Windows + H.");
    let mut keys = Enigo::new(&Settings::default()).map_err(|e| fail(&e))?;
    keys.key(Key::Meta, Press).map_err(|e| fail(&e))?;
    let typed = keys.key(Key::Unicode('h'), Click).map_err(|e| fail(&e));
    // Windows relâchée même si H n'est pas passé, pour ne pas laisser la touche enfoncée.
    keys.key(Key::Meta, Release).map_err(|e| fail(&e))?;
    typed
}
