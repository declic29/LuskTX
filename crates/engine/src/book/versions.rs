//! Historique des versions d'un livre (Lusk Texte, phase 5) : des instantanés complets du
//! manuscrit, enregistrés sur l'ordinateur seulement, dans un dossier par livre.
//!
//! Chaque version est un fichier de projet `.lusklivre` (le manuscrit et ses réglages) et une
//! fiche `.json` (nom, date, nombre de mots). Ouvrir une version l'ouvre dans un nouveau
//! document : le livre en cours n'est jamais remplacé, rien ne se perd. Le dossier du livre est
//! nommé d'après sa date de création, qui ne change pas quand le titre change.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wordcraft_doc::Document;

use super::{read_project, settings, total_words, write_project};

/// Au plus tant de versions par livre (les plus anciennes versions automatiques partent
/// d'abord ; une version nommée par l'utilisateur n'est jamais supprimée sans lui).
pub const MAX_AUTO: usize = 30;

/// Taille maximale d'une version relue (un manuscrit de livre tient largement dedans).
const MAX_VERSION_BYTES: u64 = 1 << 30;

/// Une version enregistrée.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    /// Nom du fichier sans extension (horodatage) : l'identifiant de la version.
    pub id: String,
    /// Nom donné par l'utilisateur, ou « Enregistrement automatique ».
    pub label: String,
    /// Date ISO 8601 (UTC).
    pub date: String,
    pub title: String,
    pub words: usize,
    /// Enregistrée automatiquement (à l'enregistrement du livre) plutôt que demandée.
    #[serde(default)]
    pub auto: bool,
}

/// Le dossier des versions de `doc` dans `root`.
pub fn book_folder(root: &Path, doc: &Document) -> PathBuf {
    let s = settings(doc).unwrap_or_default();
    let seed = if !doc.core.created.trim().is_empty() { doc.core.created.clone() } else { format!("{}\u{1f}{}", s.title, s.author) };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in seed.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    // Un nom lisible devant l'empreinte, sans caractère interdit dans un nom de fichier.
    let name: String = s
        .title
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
        .take(40)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let name = if name.is_empty() { "livre".to_string() } else { name };
    root.join(format!("{name}-{h:016x}"))
}

/// Un identifiant de version sûr : chiffres, lettres, tirets (jamais de chemin).
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn stamp(now: &str) -> String {
    now.chars().map(|c| if c.is_ascii_digit() { c } else { '-' }).collect::<String>().trim_matches('-').to_string()
}

/// Enregistre une version de `doc`. `label` vide : « Version du … ».
pub fn save(root: &Path, doc: &Document, label: &str, auto: bool) -> Result<Version, String> {
    let folder = book_folder(root, doc);
    crate::io::create_dir_all(&folder)?;
    let existing = crate::io::file_names(&folder, 20_000);
    let date = crate::cmd::now_iso();
    let mut id = stamp(&date);
    let mut n = 1;
    while existing.contains(&format!("{id}.lusklivre")) && n < 100 {
        n += 1;
        id = format!("{}-{n:02}", stamp(&date));
    }
    let label: String = label.trim().chars().filter(|c| !c.is_control()).take(120).collect();
    let label = if !label.is_empty() {
        label
    } else if auto {
        "Enregistrement automatique".into()
    } else {
        format!("Version du {}", date.get(..10).unwrap_or(&date))
    };
    let s = settings(doc).unwrap_or_default();
    let v = Version { id: id.clone(), label, date, title: s.title, words: total_words(doc), auto };
    let bytes = write_project(doc)?;
    let file = folder.join(format!("{id}.lusklivre"));
    crate::io::write_file(&file.to_string_lossy(), &bytes)?;
    let card = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    crate::io::write_file(&folder.join(format!("{id}.json")).to_string_lossy(), &card)?;
    if auto {
        prune(root, doc);
    }
    Ok(v)
}

/// Les versions de `doc`, les plus récentes d'abord.
pub fn list(root: &Path, doc: &Document) -> Vec<Version> {
    let folder = book_folder(root, doc);
    let names = crate::io::file_names(&folder, 20_000);
    let mut out: Vec<Version> = names
        .iter()
        .filter(|n| n.ends_with(".json"))
        .filter_map(|n| crate::io::read_capped(&folder.join(n), 1 << 16).ok())
        .filter_map(|b| serde_json::from_slice::<Version>(&b).ok())
        .filter(|v| valid_id(&v.id) && names.contains(&format!("{}.lusklivre", v.id)))
        .collect();
    out.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
    out
}

/// Le manuscrit de la version `id`.
pub fn load(root: &Path, doc: &Document, id: &str) -> Result<Document, String> {
    if !valid_id(id) {
        return Err("version inconnue".into());
    }
    let file = book_folder(root, doc).join(format!("{id}.lusklivre"));
    let bytes = crate::io::read_capped(&file, MAX_VERSION_BYTES)?;
    read_project(&bytes)
}

/// Supprime la version `id` (sur demande de l'utilisateur, après confirmation).
pub fn delete(root: &Path, doc: &Document, id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("version inconnue".into());
    }
    let folder = book_folder(root, doc);
    crate::io::remove_file(&folder.join(format!("{id}.lusklivre")))?;
    let _ = crate::io::remove_file(&folder.join(format!("{id}.json")));
    Ok(())
}

/// Ne garde que les [`MAX_AUTO`] versions automatiques les plus récentes.
fn prune(root: &Path, doc: &Document) {
    for v in list(root, doc).into_iter().filter(|v| v.auto).skip(MAX_AUTO) {
        let _ = delete(root, doc, &v.id);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BookSettings, ElementKind, insert, new_document, structure};
    use super::*;

    fn book() -> Document {
        let s = BookSettings { title: "Les Marées / la mer".into(), author: "Jeanne Martin".into(), ..BookSettings::default() };
        let mut doc = new_document(Document::new(), &s);
        doc.core.created = "2026-11-06T08:00:00Z".into();
        insert(&mut doc, ElementKind::Chapter, "La traversée", None, None).unwrap();
        doc
    }

    #[test]
    fn versions_are_saved_listed_reopened_and_deleted() {
        let root = std::env::temp_dir().join(format!("lusk-versions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut doc = book();
        assert!(list(&root, &doc).is_empty());
        let v1 = save(&root, &doc, "Avant relecture", false).unwrap();
        insert(&mut doc, ElementKind::Chapter, "Le retour", None, None).unwrap();
        let v2 = save(&root, &doc, "", false).unwrap();
        assert_ne!(v1.id, v2.id);
        assert!(v2.label.starts_with("Version du "));
        // Le dossier ne change pas avec le titre, et ne contient pas de « / ».
        let folder = book_folder(&root, &doc);
        assert!(folder.file_name().unwrap().to_string_lossy().starts_with("Les-Marées-la-mer-"));
        let vs = list(&root, &doc);
        assert_eq!(vs.len(), 2);
        assert_eq!(vs[0].id, v2.id, "newest first");
        // L'ancienne version a un chapitre de moins que le livre.
        let old = load(&root, &doc, &v1.id).unwrap();
        assert_eq!(structure(&old).len() + 1, structure(&doc).len());
        assert_eq!(settings(&old).unwrap().title, "Les Marées / la mer");
        // Identifiants hostiles.
        assert!(load(&root, &doc, "../../etc/passwd").is_err());
        assert!(delete(&root, &doc, "..").is_err());
        assert!(load(&root, &doc, "").is_err());
        delete(&root, &doc, &v1.id).unwrap();
        assert_eq!(list(&root, &doc).len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Through the commands: opening a version adds a document and leaves the book alone; a
    /// relative folder is refused.
    #[test]
    fn version_commands_open_a_copy_and_refuse_relative_folders() {
        let root = std::env::temp_dir().join(format!("lusk-versions-cmd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.to_string_lossy().to_string();
        let mut s = crate::Session::new(book());
        let v = s.run("book.versionSave", &serde_json::json!({"dir": dir, "label": "Premier jet"})).unwrap();
        let id = v["id"].as_str().unwrap().to_string();
        let list = s.run("book.versionList", &serde_json::json!({"dir": dir})).unwrap();
        assert_eq!(list["versions"][0]["label"], "Premier jet");
        let before = s.doc.plain_text(wordcraft_doc::StoryRef::Body);
        let docs = s.documents().len();
        s.run("book.versionOpen", &serde_json::json!({"dir": dir, "id": id})).unwrap();
        assert_eq!(s.documents().len(), docs + 1);
        assert!(s.doc.core.title.contains("Premier jet"), "{}", s.doc.core.title);
        assert_eq!(s.doc.plain_text(wordcraft_doc::StoryRef::Body), before);
        assert!(s.run("book.versionList", &serde_json::json!({"dir": "relatif"})).is_err());
        assert!(s.run("book.versionSave", &serde_json::json!({"dir": ""})).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn only_the_latest_automatic_versions_are_kept() {
        let root = std::env::temp_dir().join(format!("lusk-versions-auto-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let doc = book();
        let named = save(&root, &doc, "Ma version", false).unwrap();
        for _ in 0..MAX_AUTO + 3 {
            save(&root, &doc, "", true).unwrap();
        }
        let vs = list(&root, &doc);
        assert_eq!(vs.iter().filter(|v| v.auto).count(), MAX_AUTO);
        assert!(vs.iter().any(|v| v.id == named.id), "a named version is never pruned");
        let _ = std::fs::remove_dir_all(&root);
    }
}
