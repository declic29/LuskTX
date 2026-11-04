//! Built-in sample documents and templates (original text, generated in code).

use wordcraft_doc::para::InlineObject;
use wordcraft_doc::props::{Align, Border, BorderStyle, Borders, CharProps, HeightRule, NumRef, Rgb, TabAlign, TabLeader, TabStop, TextColor, VAlign};
use wordcraft_doc::{Block, Document, ListKind, Paragraph, PartKind, Table, para_block};

fn para(text: &str) -> Paragraph {
    Paragraph::with_text(text, CharProps::default())
}

fn styled(text: &str, style: &str) -> Paragraph {
    para(text).styled(style)
}

/// Append text with formatting to a paragraph.
fn add(p: &mut Paragraph, text: &str, props: CharProps) {
    let at = p.len();
    let _ = p.insert_text(at, text, &props);
}

fn bold() -> CharProps {
    CharProps { bold: Some(true), ..Default::default() }
}
fn italic() -> CharProps {
    CharProps { italic: Some(true), ..Default::default() }
}

/// The showcase document: a short studio handbook with headings, lists, a table, a quote,
/// a footer with page numbers.
pub fn sample_document() -> Document {
    let mut d = Document::new();
    d.core.title = "The Open Studio Handbook".into();
    d.core.creator = "Lusk Texte".into();
    let mut b: Vec<Block> = Vec::new();
    b.push(Block::Para(styled("The Open Studio Handbook", "Title")));
    b.push(Block::Para(styled("How a small community of makers shares space, tools and time", "Subtitle")));
    let mut intro = para("Every Thursday evening the old print shop on Alder Street fills with people who make things. ");
    add(&mut intro, "Painters", bold());
    add(&mut intro, " set up by the north windows, ", CharProps::default());
    add(&mut intro, "letterpress", italic());
    add(
        &mut intro,
        " regulars claim the back room, and somebody always brings bread. This handbook collects what we have learned about running a shared studio: the routines that keep it pleasant, the tools we pool, and the small agreements that let thirty strangers become a crew.",
        CharProps::default(),
    );
    b.push(Block::Para(intro));
    b.push(Block::Para(styled("Why share a studio?", "Heading1")));
    b.push(Block::Para(para("Rent is the obvious reason, but it is not the best one. A shared room puts your work next to work you would never have seen otherwise. A ceramicist explains glaze chemistry to a comic artist; a photographer borrows a weaver's loom for a still life. The cross-talk is the point.")));
    let mut quote = styled("A studio is a place where unfinished things are welcome.", "Quote");
    quote.props.space_before = Some(12.0);
    b.push(Block::Para(quote));
    b.push(Block::Para(styled("What we agree on", "Heading2")));
    let mut num = d.numbering.clone();
    let bullets = num.add_list(ListKind::Bullet);
    let steps = num.add_list(ListKind::Numbered);
    d.numbering = num;
    for (lead, rest) in [
        ("Clean as you go. ", "Leave a table the way you would like to find it."),
        ("Label what is yours. ", "Unlabelled supplies on the commons shelf are fair game."),
        ("Ask before you borrow. ", "Even from the shelf, a quick note helps everyone keep track."),
        ("Quiet hours ", "run from 9 to 11 on weekday mornings."),
    ] {
        let mut p = Paragraph::new().styled("ListParagraph");
        add(&mut p, lead, bold());
        add(&mut p, rest, CharProps::default());
        p.props.numbering = Some(NumRef { num: bullets, level: 0 });
        b.push(Block::Para(p));
    }
    b.push(Block::Para(styled("Shared equipment", "Heading2")));
    b.push(Block::Para(para("The table below lists what the membership owns together and who keeps each item in working order.")));
    let rows = [
        ["Equipment", "Location", "Steward", "Booking"],
        ["Etching press", "Back room", "Mara O.", "Sign-up sheet"],
        ["Kiln (cone 10)", "Yard shed", "Theo P.", "48 h notice"],
        ["Large-format printer", "Office", "Ines R.", "Calendar"],
        ["Light table", "North windows", "Anyone", "First come"],
    ];
    let mut t = Table::new(rows.len(), 4, 468.0);
    t.props.style = Some("GridTable4AccentBlue".into());
    for (r, row) in rows.iter().enumerate() {
        for (c, txt) in row.iter().enumerate() {
            if let Some(cell) = t.rows.get_mut(r).and_then(|x| x.cells.get_mut(c)) {
                cell.blocks = vec![para_block(para(txt))];
            }
        }
    }
    if let Some(r) = t.rows.first_mut() {
        r.props.header = true;
    }
    b.push(Block::Table(t));
    b.push(Block::Para(styled("Opening the studio", "Heading2")));
    for s in [
        "Switch on the main breaker by the door.",
        "Open both skylights unless it is raining.",
        "Start the kettle. This step is not optional.",
        "Check the booking sheet for the kiln and press.",
    ] {
        let mut p = para(s).styled("ListParagraph");
        p.props.numbering = Some(NumRef { num: steps, level: 0 });
        b.push(Block::Para(p));
    }
    b.push(Block::Para(styled("Membership", "Heading1")));
    let mut m = para("Membership is ");
    add(&mut m, "pay what you can", CharProps { bold: Some(true), color: Some(TextColor::Rgb(Rgb(0x15, 0x60, 0x82))), ..Default::default() });
    add(
        &mut m,
        ", reviewed every season. New members shadow a regular for their first two visits, and everyone takes one cleaning shift a month. If you have never made anything before, you are exactly who we hoped would show up.",
        CharProps::default(),
    );
    b.push(Block::Para(m));
    let mut h = para("Questions go to the front desk, or to the notice board by the sink. ");
    add(&mut h, "Visit the community board", CharProps { style: Some("Hyperlink".into()), link: Some("https://www.lusk.bzh/".into()), ..Default::default() });
    add(&mut h, " for events.", CharProps::default());
    b.push(Block::Para(h));
    d.body = b.into_iter().map(std::sync::Arc::new).collect();
    // Footer: centred page number.
    let mut f = Paragraph::new().styled("Footer");
    f.props.align = Some(Align::Center);
    let _ = f.insert_object(
        0,
        InlineObject::Field { instr: "PAGE".into(), result: "1".into(), locked: false, dirty: false, code: false, form: String::new() },
        &CharProps::default(),
    );
    let fid = d.add_part(PartKind::Footer, vec![para_block(f)]);
    d.last_section.footers.default = Some(fid);
    let mut hd = styled("The Open Studio Handbook\t\tSpring edition", "Header");
    hd.props.borders =
        Some(Borders { bottom: Some(Border { style: BorderStyle::Single, width: 0.5, color: Some(Rgb(0xBF, 0xBF, 0xBF)), space: 4.0 }), ..Default::default() });
    let hid = d.add_part(PartKind::Header, vec![para_block(hd)]);
    d.last_section.headers.default = Some(hid);
    d
}

/// Une lettre (modèle « Lettre ») : expéditeur, destinataire, objet et formule de politesse.
pub fn letter() -> Document {
    let mut d = document_fr("Lettre");
    let b = vec![
        Block::Para(styled("Prénom Nom", "Title")),
        Block::Para(para("Adresse · Code postal Ville · prenom.nom@exemple.fr · 06 00 00 00 00")),
        Block::Para(para("")),
        Block::Para(aligned(para("Nom du destinataire"), Align::Right)),
        Block::Para(aligned(para("Fonction, organisme"), Align::Right)),
        Block::Para(aligned(para("Adresse · Code postal Ville"), Align::Right)),
        Block::Para(para("")),
        Block::Para(aligned(para("À Ville, le jj/mm/aaaa"), Align::Right)),
        Block::Para(para("")),
        Block::Para(bold_para("Objet : objet de la lettre")),
        Block::Para(para("")),
        Block::Para(para("Madame, Monsieur,")),
        Block::Para(aligned(
            para(
                "Commencez par une phrase qui dit pourquoi vous écrivez. Gardez des paragraphes courts et précis, et terminez par ce que vous attendez de votre correspondant.",
            ),
            Align::Justify,
        )),
        Block::Para(aligned(para("Je vous remercie par avance de l’attention que vous porterez à ma demande."), Align::Justify)),
        Block::Para(aligned(para("Veuillez agréer, Madame, Monsieur, l’expression de mes salutations distinguées."), Align::Justify)),
        Block::Para(para("")),
        Block::Para(aligned(para("Prénom Nom"), Align::Right)),
    ];
    blocks(&mut d, b);
    d
}

/// Une ligne de CV : l'intitulé à gauche, la période en italique à droite.
fn cv_line(what: &str, when: &str, width: f32) -> Paragraph {
    let mut p = para(what);
    p.props.tabs = Some(vec![TabStop { pos: width, align: TabAlign::Right, leader: TabLeader::None }]);
    if !when.is_empty() {
        add(&mut p, &format!("\t{when}"), italic());
    }
    p
}

/// Un CV simple sur une page.
pub fn resume() -> Document {
    let mut d = document_fr("CV");
    let mut b: Vec<Block> =
        vec![Block::Para(styled("Prénom Nom", "Title")), Block::Para(para("Métier ou poste recherché · prenom.nom@exemple.fr · 06 00 00 00 00"))];
    for (head, items) in [
        ("Expérience", vec![("Intitulé du poste, Entreprise", "2021 – aujourd’hui"), ("Intitulé du poste, Entreprise", "2016 – 2021")]),
        ("Formation", vec![("Diplôme, École ou université", "2016")]),
        ("Compétences", vec![("Compétence, compétence, logiciel, outil, langue", "")]),
        ("Centres d’intérêt", vec![("Loisirs, engagements associatifs, voyages", "")]),
    ] {
        b.push(Block::Para(styled(head, "Heading1")));
        for (what, when) in items {
            b.push(Block::Para(cv_line(what, when, A4_TEXT)));
        }
    }
    blocks(&mut d, b);
    d
}

/// Un rapport : titre, sous-titre et sections.
pub fn report() -> Document {
    let mut d = document_fr("Rapport");
    let b: Vec<Block> = vec![
        Block::Para(styled("Titre du rapport", "Title")),
        Block::Para(styled("Sous-titre ou date", "Subtitle")),
        Block::Para(styled("Résumé", "Heading1")),
        Block::Para(para("Résumez les conclusions en deux ou trois phrases.")),
        Block::Para(styled("Contexte", "Heading1")),
        Block::Para(para("Décrivez le contexte et la question à laquelle le rapport répond.")),
        Block::Para(styled("Constats", "Heading1")),
        Block::Para(styled("Premier constat", "Heading2")),
        Block::Para(para("Présentez les éléments qui l’appuient.")),
        Block::Para(styled("Deuxième constat", "Heading2")),
        Block::Para(para("Présentez les éléments qui l’appuient.")),
        Block::Para(styled("Recommandations", "Heading1")),
        Block::Para(para("Indiquez ce qui devrait être fait ensuite, par qui et dans quel délai.")),
    ];
    blocks(&mut d, b);
    d
}

// ---------------------------------------------------------------------------------------------
// Modèles français de Lusk Texte : format A4, texte vérifié en français.

/// Largeur d'une page A4 et d'une ligne de texte avec les marges par défaut (2,54 cm), en points.
const A4_W: f32 = 595.3;
const A4_H: f32 = 841.9;
const A4_TEXT: f32 = A4_W - 2.0 * 72.0;

/// Un document A4 en français.
fn document_fr(title: &str) -> Document {
    let mut d = Document::new();
    d.last_section.page_w = A4_W;
    d.last_section.page_h = A4_H;
    d.styles.default_chr.lang = Some("fr-FR".into());
    d.core.title = title.into();
    d.core.creator = "Lusk Texte".into();
    d
}

fn aligned(mut p: Paragraph, align: Align) -> Paragraph {
    p.props.align = Some(align);
    p
}

fn bold_para(text: &str) -> Paragraph {
    let mut p = Paragraph::new();
    add(&mut p, text, bold());
    p
}

/// Un tableau de la largeur de la ligne ; la première ligne est l'en-tête quand `header` est vrai.
fn table(rows: &[&[&str]], header: bool) -> Block {
    let cols = rows.first().map_or(1, |r| r.len());
    let mut t = Table::new(rows.len(), cols, A4_TEXT);
    t.props.style = Some("GridTable4AccentBlue".into());
    for (r, row) in rows.iter().enumerate() {
        for (c, txt) in row.iter().enumerate() {
            if let Some(cell) = t.rows.get_mut(r).and_then(|x| x.cells.get_mut(c)) {
                cell.blocks = vec![para_block(para(txt))];
            }
        }
    }
    if header && let Some(r) = t.rows.first_mut() {
        r.props.header = true;
    }
    Block::Table(t)
}

fn blocks(d: &mut Document, b: Vec<Block>) {
    d.body = b.into_iter().map(std::sync::Arc::new).collect();
}

/// Une lettre de motivation.
pub fn cover_letter_fr() -> Document {
    let mut d = document_fr("Lettre de motivation");
    let b = vec![
        Block::Para(styled("Prénom Nom", "Title")),
        Block::Para(para("Adresse · Code postal Ville")),
        Block::Para(para("06 00 00 00 00 · prenom.nom@exemple.fr")),
        Block::Para(para("")),
        Block::Para(aligned(para("Nom de l’entreprise"), Align::Right)),
        Block::Para(aligned(para("À l’attention de Madame ou Monsieur …"), Align::Right)),
        Block::Para(aligned(para("Adresse · Code postal Ville"), Align::Right)),
        Block::Para(para("")),
        Block::Para(aligned(para("À Ville, le jj/mm/aaaa"), Align::Right)),
        Block::Para(para("")),
        Block::Para(bold_para("Objet : candidature au poste de …")),
        Block::Para(para("")),
        Block::Para(para("Madame, Monsieur,")),
        Block::Para(aligned(
            para("Présentez-vous en une ou deux phrases et dites pourquoi vous écrivez : le poste visé, et comment vous avez connu l’offre ou l’entreprise."),
            Align::Justify,
        )),
        Block::Para(aligned(
            para(
                "Montrez ce que vous apportez : une ou deux expériences précises, les compétences qu’elles prouvent et les résultats obtenus. Faites le lien avec les besoins de l’entreprise.",
            ),
            Align::Justify,
        )),
        Block::Para(aligned(
            para("Expliquez ce qui vous attire dans cette entreprise en particulier, puis proposez une rencontre pour en parler de vive voix."),
            Align::Justify,
        )),
        Block::Para(aligned(
            para("Dans l’attente de votre réponse, je vous prie d’agréer, Madame, Monsieur, l’expression de mes salutations distinguées."),
            Align::Justify,
        )),
        Block::Para(para("")),
        Block::Para(aligned(para("Prénom Nom"), Align::Right)),
    ];
    blocks(&mut d, b);
    d
}

/// Un compte rendu de réunion : informations, ordre du jour, points abordés, décisions et actions.
pub fn minutes_fr() -> Document {
    let mut d = document_fr("Compte rendu de réunion");
    let mut num = d.numbering.clone();
    let bullets = num.add_list(ListKind::Bullet);
    let steps = num.add_list(ListKind::Numbered);
    d.numbering = num;
    let item = |text: &str, list| {
        let mut p = para(text).styled("ListParagraph");
        p.props.numbering = Some(NumRef { num: list, level: 0 });
        Block::Para(p)
    };
    let mut b = vec![
        Block::Para(styled("Compte rendu de réunion", "Title")),
        Block::Para(styled("Objet de la réunion", "Subtitle")),
        table(
            &[
                &["Date", "jj/mm/aaaa, de 10 h à 11 h 30"],
                &["Lieu", "Salle de réunion ou visioconférence"],
                &["Présents", "Prénom Nom, Prénom Nom, Prénom Nom"],
                &["Excusés", "Prénom Nom"],
                &["Rédacteur", "Prénom Nom"],
            ],
            false,
        ),
        Block::Para(styled("Ordre du jour", "Heading1")),
    ];
    for s in ["Premier sujet", "Deuxième sujet", "Questions diverses"] {
        b.push(item(s, steps));
    }
    b.push(Block::Para(styled("Points abordés", "Heading1")));
    b.push(Block::Para(styled("Premier sujet", "Heading2")));
    b.push(Block::Para(para("Résumez les échanges : la situation présentée, les avis exprimés et les points restés en suspens.")));
    b.push(Block::Para(styled("Deuxième sujet", "Heading2")));
    b.push(Block::Para(para("Résumez les échanges sur ce sujet.")));
    b.push(Block::Para(styled("Décisions", "Heading1")));
    for s in ["Décision prise sur le premier sujet.", "Décision prise sur le deuxième sujet."] {
        b.push(item(s, bullets));
    }
    b.push(Block::Para(styled("Actions à mener", "Heading1")));
    b.push(table(
        &[
            &["Action", "Responsable", "Échéance"],
            &["Décrire l’action", "Prénom Nom", "jj/mm/aaaa"],
            &["Décrire l’action", "Prénom Nom", "jj/mm/aaaa"],
            &["Décrire l’action", "Prénom Nom", "jj/mm/aaaa"],
        ],
        true,
    ));
    b.push(Block::Para(styled("Prochaine réunion", "Heading1")));
    b.push(Block::Para(para("Le jj/mm/aaaa à 10 h, même lieu.")));
    blocks(&mut d, b);
    d
}

/// Une facture : émetteur, client, lignes, totaux et conditions de paiement.
pub fn invoice_fr() -> Document {
    let mut d = document_fr("Facture");
    let mut no = para("Facture n° 2026-001 · Date : jj/mm/aaaa");
    no.props.space_after = Some(12.0);
    let b = vec![
        Block::Para(styled("Facture", "Title")),
        Block::Para(no),
        table(
            &[
                &["Émetteur", "Client"],
                &["Nom ou raison sociale", "Nom ou raison sociale"],
                &["Adresse", "Adresse"],
                &["Code postal Ville", "Code postal Ville"],
                &["SIRET : 000 000 000 00000", "N° de client : …"],
                &["TVA intracommunautaire : FR00 000000000", ""],
            ],
            true,
        ),
        Block::Para(para("")),
        table(
            &[
                &["Désignation", "Quantité", "Prix unitaire HT", "Total HT"],
                &["Prestation ou article", "1", "0,00 €", "0,00 €"],
                &["Prestation ou article", "1", "0,00 €", "0,00 €"],
                &["Prestation ou article", "1", "0,00 €", "0,00 €"],
                &["", "", "Total HT", "0,00 €"],
                &["", "", "TVA 20 %", "0,00 €"],
                &["", "", "Total TTC", "0,00 €"],
            ],
            true,
        ),
        Block::Para(para("")),
        Block::Para(bold_para("Conditions de paiement")),
        Block::Para(para("Paiement à 30 jours à réception de facture, par virement : IBAN FR76 0000 0000 0000 0000 0000 000.")),
        Block::Para(para(
            "En cas de retard de paiement, des pénalités au taux légal en vigueur sont dues, ainsi qu’une indemnité forfaitaire de 40 € pour frais de recouvrement. Pas d’escompte pour paiement anticipé.",
        )),
    ];
    blocks(&mut d, b);
    d
}

/// Une note de service.
pub fn memo_fr() -> Document {
    let mut d = document_fr("Note de service");
    let b = vec![
        Block::Para(styled("Note de service", "Title")),
        table(&[&["De", "Prénom Nom, fonction"], &["À", "Destinataires (service, équipe…)"], &["Date", "jj/mm/aaaa"], &["Objet", "Objet de la note"]], false),
        Block::Para(para("")),
        Block::Para(aligned(
            para("Exposez en quelques lignes l’information ou la consigne : ce qui change, à partir de quand, et qui est concerné."),
            Align::Justify,
        )),
        Block::Para(aligned(para("Précisez ce que chacun doit faire et, le cas échéant, la personne à contacter pour toute question."), Align::Justify)),
        Block::Para(para("")),
        Block::Para(aligned(para("Prénom Nom"), Align::Right)),
        Block::Para(aligned(para("Fonction"), Align::Right)),
    ];
    blocks(&mut d, b);
    d
}

/// Une attestation sur l’honneur.
pub fn attestation_fr() -> Document {
    let mut d = document_fr("Attestation sur l’honneur");
    let mut title = aligned(styled("Attestation sur l’honneur", "Title"), Align::Center);
    title.props.space_after = Some(24.0);
    let b = vec![
        Block::Para(title),
        Block::Para(aligned(para("Je soussigné(e) Prénom Nom, né(e) le jj/mm/aaaa à Ville, demeurant Adresse, Code postal Ville,"), Align::Justify)),
        Block::Para(para("")),
        Block::Para(aligned(para("atteste sur l’honneur que : décrivez précisément le fait que vous attestez."), Align::Justify)),
        Block::Para(para("")),
        Block::Para(aligned(
            para("Je suis informé(e) qu’une fausse attestation m’expose aux sanctions prévues par l’article 441-7 du Code pénal."),
            Align::Justify,
        )),
        Block::Para(para("")),
        Block::Para(para("Fait pour servir et valoir ce que de droit.")),
        Block::Para(para("")),
        Block::Para(aligned(para("Fait à Ville, le jj/mm/aaaa"), Align::Right)),
        Block::Para(aligned(para("Signature"), Align::Right)),
    ];
    blocks(&mut d, b);
    d
}

/// Des caractères de couleur `rgb`, à la taille `size`, en gras ou non.
fn colored(rgb: Rgb, size: f32, bold: bool) -> CharProps {
    CharProps { color: Some(TextColor::Rgb(rgb)), size: Some(size), bold: Some(bold), ..Default::default() }
}

fn run_para(text: &str, props: CharProps) -> Paragraph {
    let mut p = Paragraph::new();
    add(&mut p, text, props);
    p
}

/// Un titre de section souligné d'un filet, pour le CV sérieux.
fn ruled_heading(text: &str, rgb: Rgb) -> Paragraph {
    let mut p = run_para(&text.to_uppercase(), CharProps { spacing: Some(1.0), ..colored(rgb, 11.0, true) });
    p.props.space_before = Some(14.0);
    p.props.space_after = Some(6.0);
    p.props.borders = Some(Borders { bottom: Some(Border { style: BorderStyle::Single, width: 0.75, color: Some(rgb), space: 2.0 }), ..Default::default() });
    p
}

/// Un CV sérieux : sections sobres soulignées d'un filet, dates alignées à droite.
pub fn resume_serious_fr() -> Document {
    let mut d = document_fr("CV");
    let navy = Rgb(0x1F, 0x3A, 0x5F);
    let mut name = aligned(run_para("PRÉNOM NOM", colored(navy, 24.0, true)), Align::Center);
    name.props.space_after = Some(2.0);
    let mut b = vec![
        Block::Para(name),
        Block::Para(aligned(run_para("Intitulé du poste recherché", colored(Rgb(0x59, 0x59, 0x59), 12.0, false)), Align::Center)),
        Block::Para(aligned(para("Adresse · Code postal Ville · 06 00 00 00 00 · prenom.nom@exemple.fr"), Align::Center)),
        Block::Para(ruled_heading("Profil", navy)),
        Block::Para(aligned(
            para("En trois lignes : votre métier, vos années d’expérience, vos points forts et le type de poste que vous visez."),
            Align::Justify,
        )),
        Block::Para(ruled_heading("Expérience professionnelle", navy)),
    ];
    for (role, when) in [("Intitulé du poste — Entreprise, Ville", "2021 – aujourd’hui"), ("Intitulé du poste — Entreprise, Ville", "2017 – 2021")]
    {
        let mut p = Paragraph::new();
        add(&mut p, role, bold());
        p.props.tabs = Some(vec![TabStop { pos: A4_TEXT, align: TabAlign::Right, leader: TabLeader::None }]);
        add(&mut p, &format!("\t{when}"), italic());
        p.props.space_before = Some(6.0);
        b.push(Block::Para(p));
        for task in ["Mission ou réalisation, avec un résultat chiffré si possible.", "Mission ou réalisation."] {
            b.push(Block::Para(para(&format!("– {task}"))));
        }
    }
    b.push(Block::Para(ruled_heading("Formation", navy)));
    b.push(Block::Para(cv_line("Diplôme — École ou université, Ville", "2017", A4_TEXT)));
    b.push(Block::Para(cv_line("Baccalauréat — Lycée, Ville", "2012", A4_TEXT)));
    b.push(Block::Para(ruled_heading("Compétences", navy)));
    for (k, v) in
        [("Métier : ", "compétence, compétence, compétence"), ("Logiciels : ", "logiciel, logiciel"), ("Langues : ", "anglais (courant), espagnol (notions)")]
    {
        let mut p = Paragraph::new();
        add(&mut p, k, bold());
        add(&mut p, v, CharProps::default());
        b.push(Block::Para(p));
    }
    b.push(Block::Para(ruled_heading("Centres d’intérêt", navy)));
    b.push(Block::Para(para("Activités, engagements associatifs, sports.")));
    blocks(&mut d, b);
    d
}

/// Une cellule de tableau remplie de `paras`, sur un fond `fill` éventuel.
fn cell_blocks(t: &mut Table, r: usize, c: usize, paras: Vec<Paragraph>, fill: Option<Rgb>) {
    if let Some(cell) = t.rows.get_mut(r).and_then(|x| x.cells.get_mut(c)) {
        cell.blocks = paras.into_iter().map(para_block).collect();
        cell.props.shading = fill;
        cell.props.margins = Some([14.0, 14.0, 14.0, 14.0]);
    }
}

/// Un CV créatif : une colonne colorée à gauche (coordonnées, compétences, langues) et le
/// parcours à droite.
pub fn resume_creative_fr() -> Document {
    let mut d = document_fr("CV");
    for m in [&mut d.last_section.margin_top, &mut d.last_section.margin_bottom, &mut d.last_section.margin_left, &mut d.last_section.margin_right] {
        *m = 36.0;
    }
    let width = A4_W - 72.0;
    let side = 180.0;
    let teal = Rgb(0x13, 0x6F, 0x7A);
    let white = Rgb(0xFF, 0xFF, 0xFF);
    let light = Rgb(0xD9, 0xF0, 0xF2);
    let mut t = Table::new(1, 2, width);
    t.props.style = None;
    t.grid = vec![side, width - side];
    if let Some(row) = t.rows.first_mut() {
        if let Some(c) = row.cells.get_mut(0) {
            c.props.width = Some(side);
        }
        if let Some(c) = row.cells.get_mut(1) {
            c.props.width = Some(width - side);
        }
    }
    let head = |s: &str| {
        let mut p = run_para(&s.to_uppercase(), CharProps { spacing: Some(1.5), ..colored(white, 10.5, true) });
        p.props.space_before = Some(14.0);
        p.props.space_after = Some(4.0);
        p
    };
    let side_text = |s: &str| run_para(s, colored(light, 9.5, false));
    let mut left = vec![
        run_para("Prénom", colored(white, 22.0, true)),
        run_para("Nom", colored(white, 22.0, true)),
        run_para("Graphiste · Illustratrice", colored(light, 11.0, false)),
        head("Contact"),
        side_text("06 00 00 00 00"),
        side_text("prenom.nom@exemple.fr"),
        side_text("portfolio.exemple.fr"),
        side_text("Ville"),
        head("Compétences"),
    ];
    for s in ["Identité visuelle", "Illustration", "Mise en page", "Photographie", "Suite créative"] {
        left.push(side_text(&format!("• {s}")));
    }
    left.push(head("Langues"));
    left.push(side_text("Anglais — courant"));
    left.push(side_text("Espagnol — intermédiaire"));
    left.push(head("Centres d’intérêt"));
    left.push(side_text("Bande dessinée, voyages, céramique"));
    let rhead = |s: &str| {
        let mut p = run_para(s, colored(teal, 15.0, true));
        p.props.space_before = Some(12.0);
        p.props.space_after = Some(4.0);
        p
    };
    let mut right = vec![
        rhead("Profil"),
        aligned(
            para("Présentez-vous en quelques lignes, avec votre style : ce que vous aimez créer, pour qui, et ce qui rend votre travail reconnaissable."),
            Align::Justify,
        ),
        rhead("Expérience"),
    ];
    for (role, place, when) in [
        ("Intitulé du poste", "Studio ou entreprise, Ville", "2022 – aujourd’hui"),
        ("Intitulé du poste", "Studio ou entreprise, Ville", "2019 – 2022"),
        ("Projets indépendants", "Clients ou collaborations", "depuis 2017"),
    ] {
        let mut p = run_para(role, colored(Rgb(0x26, 0x26, 0x26), 11.5, true));
        p.props.space_before = Some(6.0);
        right.push(p);
        right.push(run_para(&format!("{place} · {when}"), CharProps { italic: Some(true), ..colored(teal, 9.5, false) }));
        right.push(para("Une ou deux réalisations marquantes, avec ce qu’elles ont apporté."));
    }
    right.push(rhead("Formation"));
    right.push(run_para("Diplôme en arts appliqués", colored(Rgb(0x26, 0x26, 0x26), 11.5, true)));
    right.push(run_para("École, Ville · 2019", CharProps { italic: Some(true), ..colored(teal, 9.5, false) }));
    cell_blocks(&mut t, 0, 0, left, Some(teal));
    cell_blocks(&mut t, 0, 1, right, None);
    blocks(&mut d, vec![Block::Table(t)]);
    d
}

/// Une planche A4 de dix cartes de visite au format standard (85 × 55 mm), avec des traits
/// pointillés pour la découpe.
pub fn business_cards_fr() -> Document {
    let mut d = document_fr("Cartes de visite");
    let card_w = 85.0 * 72.0 / 25.4;
    let card_h = 55.0 * 72.0 / 25.4;
    let side = (A4_W - 2.0 * card_w) / 2.0;
    let top = (A4_H - 5.0 * card_h) / 2.0;
    let s = &mut d.last_section;
    s.margin_left = side;
    s.margin_right = side;
    s.margin_top = top;
    s.margin_bottom = top - 1.0;
    s.header = 12.0;
    s.footer = 12.0;
    let accent = Rgb(0x13, 0x6F, 0x7A);
    let cut = Border { style: BorderStyle::Dotted, width: 0.5, color: Some(Rgb(0xBF, 0xBF, 0xBF)), space: 0.0 };
    let mut t = Table::new(5, 2, 2.0 * card_w);
    t.props.style = None;
    t.props.borders = Some(Borders { top: Some(cut), left: Some(cut), bottom: Some(cut), right: Some(cut), between: Some(cut), inside_v: Some(cut) });
    t.props.cell_margins = Some([0.0, 0.0, 0.0, 0.0]);
    for r in 0..5 {
        if let Some(row) = t.rows.get_mut(r) {
            row.props.height = Some(card_h);
            row.props.height_rule = HeightRule::Exact;
            row.props.cant_split = true;
        }
        for c in 0..2 {
            let mut name = run_para("Prénom Nom", colored(accent, 13.0, true));
            name.props.space_after = Some(0.0);
            let mut role = run_para("Fonction · Entreprise", CharProps { italic: Some(true), ..colored(Rgb(0x59, 0x59, 0x59), 8.5, false) });
            role.props.space_after = Some(8.0);
            let small = |s: &str| {
                let mut p = run_para(s, colored(Rgb(0x26, 0x26, 0x26), 8.0, false));
                p.props.space_after = Some(0.0);
                p
            };
            let paras =
                vec![name, role, small("06 00 00 00 00"), small("prenom.nom@exemple.fr"), small("www.exemple.fr"), small("Adresse · Code postal Ville")];
            if let Some(cell) = t.rows.get_mut(r).and_then(|x| x.cells.get_mut(c)) {
                cell.blocks = paras.into_iter().map(para_block).collect();
                cell.props.valign = Some(VAlign::Center);
                cell.props.margins = Some([10.0, 16.0, 10.0, 16.0]);
            }
        }
    }
    blocks(&mut d, vec![Block::Table(t)]);
    d
}

/// Une brochure : A4 à l'italienne sur trois colonnes, à plier en trois.
pub fn brochure_fr() -> Document {
    let mut d = document_fr("Brochure");
    let s = &mut d.last_section;
    s.page_w = A4_H;
    s.page_h = A4_W;
    s.landscape = true;
    for m in [&mut s.margin_top, &mut s.margin_bottom, &mut s.margin_left, &mut s.margin_right] {
        *m = 36.0;
    }
    s.columns.count = 3;
    s.columns.space = 72.0;
    let accent = Rgb(0x13, 0x6F, 0x7A);
    let mut num = d.numbering.clone();
    let bullets = num.add_list(ListKind::Bullet);
    d.numbering = num;
    let bullet = |text: &str| {
        let mut p = para(text).styled("ListParagraph");
        p.props.numbering = Some(NumRef { num: bullets, level: 0 });
        Block::Para(p)
    };
    let head = |text: &str| {
        let mut p = run_para(text, colored(accent, 15.0, true));
        p.props.space_before = Some(10.0);
        p.props.space_after = Some(4.0);
        Block::Para(p)
    };
    let text = |t: &str| Block::Para(aligned(para(t), Align::Justify));
    let mut b = vec![
        Block::Para(run_para("Titre de la brochure", colored(accent, 26.0, true))),
        Block::Para(run_para(
            "Une phrase d’accroche qui donne envie d’ouvrir",
            CharProps { italic: Some(true), ..colored(Rgb(0x59, 0x59, 0x59), 12.0, false) },
        )),
        head("Qui sommes-nous ?"),
        text("Présentez en quelques lignes votre association, votre entreprise ou votre événement : ce que vous faites, depuis quand, et pour qui."),
        text("Insérez une image ici (Insertion › Images) pour illustrer votre propos."),
        head("Ce que nous proposons"),
        bullet("Premier service ou activité"),
        bullet("Deuxième service ou activité"),
        bullet("Troisième service ou activité"),
        text("Ajoutez une ou deux phrases sur ce qui vous distingue."),
        head("Pourquoi nous choisir"),
        text("Citez un avis de client ou de participant, des chiffres clés, ou les valeurs qui vous animent."),
        head("Tarifs et horaires"),
        text("Précisez les tarifs, les jours et horaires d’ouverture, ou le programme de l’événement."),
        head("Nous trouver"),
        text("Adresse, accès en transports, parking."),
        head("Nous contacter"),
    ];
    for line in ["06 00 00 00 00", "contact@exemple.fr", "www.exemple.fr"] {
        b.push(Block::Para(run_para(line, colored(accent, 11.0, true))));
    }
    blocks(&mut d, b);
    d
}

/// Un paragraphe avec un espace avant et après, en points.
fn spaced(mut p: Paragraph, before: f32, after: f32) -> Paragraph {
    p.props.space_before = Some(before);
    p.props.space_after = Some(after);
    p
}

/// Un cadre autour de la page entière.
fn page_border(d: &mut Document, rgb: Rgb, style: BorderStyle, width: f32) {
    let b = Border { style, width, color: Some(rgb), space: 24.0 };
    d.last_section.page_borders = Some(Borders { top: Some(b), left: Some(b), bottom: Some(b), right: Some(b), ..Default::default() });
}

/// Une ligne centrée.
fn center(text: &str, props: CharProps) -> Block {
    Block::Para(aligned(run_para(text, props), Align::Center))
}

/// Une ligne centrée avec un espace avant et après.
fn center_spaced(text: &str, props: CharProps, before: f32, after: f32) -> Block {
    Block::Para(aligned(spaced(run_para(text, props), before, after), Align::Center))
}

/// Une invitation : un cadre, le titre de l'événement et les informations pratiques.
pub fn invitation_fr() -> Document {
    let mut d = document_fr("Invitation");
    let rose = Rgb(0xB8, 0x3B, 0x5E);
    let grey = Rgb(0x59, 0x59, 0x59);
    let ink = Rgb(0x26, 0x26, 0x26);
    page_border(&mut d, rose, BorderStyle::Double, 3.0);
    let b = vec![
        center_spaced("VOUS ÊTES INVITÉ(E)", CharProps { spacing: Some(3.0), ..colored(grey, 12.0, true) }, 60.0, 18.0),
        center_spaced("Anniversaire de Prénom", colored(rose, 34.0, true), 0.0, 6.0),
        center("pour fêter ses 40 ans", CharProps { italic: Some(true), ..colored(grey, 15.0, false) }),
        center_spaced("Samedi jj mois aaaa à 19 h", colored(ink, 16.0, true), 48.0, 4.0),
        center("Nom du lieu", colored(ink, 14.0, false)),
        center("Adresse · Code postal Ville", colored(grey, 12.0, false)),
        center_spaced("Au programme : apéritif, dîner et soirée dansante. Tenue de fête conseillée !", colored(grey, 12.0, false), 48.0, 30.0),
        center_spaced("Merci de confirmer votre venue avant le jj/mm", colored(rose, 12.0, true), 24.0, 2.0),
        center("06 00 00 00 00 · prenom.nom@exemple.fr", colored(grey, 12.0, false)),
    ];
    blocks(&mut d, b);
    d
}

/// Un faire-part de naissance.
pub fn birth_announcement_fr() -> Document {
    let mut d = document_fr("Faire-part de naissance");
    let blue = Rgb(0x3A, 0x7C, 0xA5);
    let grey = Rgb(0x59, 0x59, 0x59);
    let ink = Rgb(0x26, 0x26, 0x26);
    page_border(&mut d, blue, BorderStyle::Single, 1.5);
    let mut b = vec![
        center_spaced("Nous avons la joie de vous annoncer la naissance de", CharProps { italic: Some(true), ..colored(grey, 15.0, false) }, 72.0, 20.0),
        center_spaced("Prénom", colored(blue, 44.0, true), 0.0, 24.0),
    ];
    for (k, v) in [("Né(e) le", "jj mois aaaa à hh h mm"), ("à", "Ville"), ("Poids :", "3,2 kg"), ("Taille :", "50 cm")] {
        let mut p = Paragraph::new();
        add(&mut p, &format!("{k} "), colored(grey, 13.0, false));
        add(&mut p, v, colored(ink, 13.0, true));
        b.push(Block::Para(aligned(spaced(p, 0.0, 4.0), Align::Center)));
    }
    b.push(center_spaced("Ses parents, Prénom et Prénom,", colored(grey, 14.0, false), 40.0, 2.0));
    b.push(center("et sa grande sœur Prénom sont aux anges.", colored(grey, 14.0, false)));
    b.push(center_spaced("Adresse · Code postal Ville", colored(grey, 11.0, false), 40.0, 0.0));
    blocks(&mut d, b);
    d
}

/// Un menu de repas ou de restaurant : plats et prix reliés par des points de conduite.
pub fn menu_fr() -> Document {
    let mut d = document_fr("Menu");
    let wine = Rgb(0x7A, 0x2E, 0x3B);
    let grey = Rgb(0x59, 0x59, 0x59);
    let ink = Rgb(0x26, 0x26, 0x26);
    page_border(&mut d, wine, BorderStyle::Double, 2.0);
    let price_tab = Some(vec![TabStop { pos: A4_TEXT, align: TabAlign::Right, leader: TabLeader::Dot }]);
    let mut b = vec![
        center_spaced("Menu", colored(wine, 40.0, true), 24.0, 0.0),
        center("Nom du restaurant ou de l’événement", CharProps { italic: Some(true), ..colored(grey, 14.0, false) }),
    ];
    for (section, dishes) in [
        ("Entrées", [("Velouté de saison, croûtons dorés", "8,00 €"), ("Salade de chèvre chaud au miel", "9,50 €")]),
        ("Plats", [("Filet de poisson, beurre blanc et légumes", "18,00 €"), ("Suprême de volaille, sauce aux morilles", "17,50 €")]),
        ("Desserts", [("Tarte fine aux pommes, crème fraîche", "7,00 €"), ("Moelleux au chocolat, glace vanille", "7,50 €")]),
    ] {
        b.push(center_spaced(&section.to_uppercase(), CharProps { spacing: Some(2.0), ..colored(wine, 14.0, true) }, 26.0, 8.0));
        for (dish, price) in dishes {
            let mut p = Paragraph::new();
            add(&mut p, dish, colored(ink, 12.5, false));
            add(&mut p, &format!("\t{price}"), colored(ink, 12.5, true));
            p.props.tabs = price_tab.clone();
            b.push(Block::Para(spaced(p, 0.0, 6.0)));
        }
    }
    b.push(center_spaced("Menu complet (entrée, plat, dessert) : 29,00 €", colored(wine, 13.0, true), 26.0, 4.0));
    b.push(center("Prix nets, service compris. Liste des allergènes disponible sur demande.", CharProps { italic: Some(true), ..colored(grey, 10.5, false) }));
    blocks(&mut d, b);
    d
}

/// Une attestation employeur (attestation de travail).
pub fn employer_certificate_fr() -> Document {
    let mut d = document_fr("Attestation employeur");
    let navy = Rgb(0x1F, 0x3A, 0x5F);
    let justified = |t: &str| Block::Para(aligned(para(t), Align::Justify));
    let b = vec![
        Block::Para(bold_para("Nom de l’entreprise")),
        Block::Para(para("Adresse · Code postal Ville")),
        Block::Para(para("SIRET : 000 000 000 00000 · Téléphone : 01 00 00 00 00")),
        center_spaced("ATTESTATION EMPLOYEUR", CharProps { spacing: Some(2.0), ..colored(navy, 20.0, true) }, 48.0, 30.0),
        justified(
            "Je soussigné(e) Prénom Nom, agissant en qualité de fonction (gérant, directeur des ressources humaines…) de la société Nom de l’entreprise,",
        ),
        Block::Para(para("")),
        justified(
            "atteste que Madame / Monsieur Prénom Nom, né(e) le jj/mm/aaaa, demeurant Adresse, Code postal Ville, est employé(e) dans notre entreprise depuis le jj/mm/aaaa en qualité de intitulé du poste, dans le cadre d’un contrat à durée indéterminée (ou : déterminée, jusqu’au jj/mm/aaaa), à temps complet (ou : partiel).",
        ),
        Block::Para(para("")),
        justified("À ce jour, l’intéressé(e) n’est ni démissionnaire, ni en période d’essai, ni sous le coup d’une procédure de licenciement."),
        Block::Para(para("")),
        justified("Cette attestation est délivrée à la demande de l’intéressé(e), pour servir et valoir ce que de droit."),
        Block::Para(para("")),
        Block::Para(aligned(para("Fait à Ville, le jj/mm/aaaa"), Align::Right)),
        Block::Para(aligned(para("Prénom Nom, fonction"), Align::Right)),
        Block::Para(aligned(run_para("Signature et cachet de l’entreprise", CharProps { italic: Some(true), ..Default::default() }), Align::Right)),
    ];
    blocks(&mut d, b);
    d
}

/// Un devis : émetteur, client, lignes, totaux, validité et « bon pour accord ».
pub fn quote_fr() -> Document {
    let mut d = document_fr("Devis");
    let mut no = para("Devis n° 2026-001 · Date : jj/mm/aaaa · Valable jusqu’au : jj/mm/aaaa");
    no.props.space_after = Some(12.0);
    let b = vec![
        Block::Para(styled("Devis", "Title")),
        Block::Para(no),
        table(
            &[
                &["Émetteur", "Client"],
                &["Nom ou raison sociale", "Nom ou raison sociale"],
                &["Adresse", "Adresse"],
                &["Code postal Ville", "Code postal Ville"],
                &["SIRET : 000 000 000 00000", "Téléphone · courriel"],
            ],
            true,
        ),
        Block::Para(para("")),
        Block::Para(bold_para("Objet : description de la prestation ou des travaux")),
        Block::Para(para("")),
        table(
            &[
                &["Désignation", "Quantité", "Prix unitaire HT", "Total HT"],
                &["Prestation, fourniture ou main-d’œuvre", "1", "0,00 €", "0,00 €"],
                &["Prestation, fourniture ou main-d’œuvre", "1", "0,00 €", "0,00 €"],
                &["Prestation, fourniture ou main-d’œuvre", "1", "0,00 €", "0,00 €"],
                &["", "", "Total HT", "0,00 €"],
                &["", "", "TVA 20 %", "0,00 €"],
                &["", "", "Total TTC", "0,00 €"],
            ],
            true,
        ),
        Block::Para(para("")),
        Block::Para(bold_para("Conditions")),
        Block::Para(para("Délai de réalisation : à convenir. Acompte de 30 % à la signature, solde à la livraison. Devis gratuit.")),
        Block::Para(para("")),
        table(&[&["Bon pour accord", ""], &["Date :", "Signature du client, précédée de la mention « Bon pour accord » :"], &["", ""]], true),
    ];
    blocks(&mut d, b);
    d
}

/// Un certificat (de réussite, de participation…) : A4 à l'italienne dans un double cadre doré.
pub fn certificate_fr() -> Document {
    let mut d = document_fr("Certificat");
    let s = &mut d.last_section;
    s.page_w = A4_H;
    s.page_h = A4_W;
    s.landscape = true;
    let gold = Rgb(0xA6, 0x7C, 0x1F);
    let navy = Rgb(0x1F, 0x3A, 0x5F);
    let grey = Rgb(0x59, 0x59, 0x59);
    page_border(&mut d, gold, BorderStyle::Double, 4.5);
    let width = A4_H - 2.0 * 72.0;
    let mut sign = Table::new(2, 2, width);
    sign.props.style = None;
    for (c, label) in ["Date", "Signature"].iter().enumerate() {
        if let Some(cell) = sign.rows.get_mut(0).and_then(|r| r.cells.get_mut(c)) {
            cell.blocks = vec![para_block(aligned(spaced(para("______________________"), 30.0, 0.0), Align::Center))];
        }
        if let Some(cell) = sign.rows.get_mut(1).and_then(|r| r.cells.get_mut(c)) {
            cell.blocks = vec![para_block(aligned(run_para(label, colored(grey, 11.0, false)), Align::Center))];
        }
    }
    let b = vec![
        center_spaced("CERTIFICAT", CharProps { spacing: Some(6.0), ..colored(navy, 40.0, true) }, 6.0, 0.0),
        center("de réussite", CharProps { italic: Some(true), ..colored(gold, 20.0, false) }),
        center_spaced("décerné à", colored(grey, 14.0, false), 26.0, 6.0),
        center_spaced("Prénom Nom", colored(navy, 34.0, true), 0.0, 14.0),
        center("pour avoir suivi avec succès la formation « Intitulé de la formation »", colored(grey, 14.0, false)),
        center("organisée par Nom de l’organisme, du jj/mm/aaaa au jj/mm/aaaa.", colored(grey, 14.0, false)),
        Block::Para(para("")),
        Block::Table(sign),
    ];
    blocks(&mut d, b);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_valid() {
        assert_eq!(sample_document().word_count(), 361);
        for d in [
            sample_document(),
            letter(),
            resume(),
            report(),
            cover_letter_fr(),
            minutes_fr(),
            invoice_fr(),
            memo_fr(),
            attestation_fr(),
            resume_serious_fr(),
            resume_creative_fr(),
            business_cards_fr(),
            brochure_fr(),
            invitation_fr(),
            birth_announcement_fr(),
            menu_fr(),
            employer_certificate_fr(),
            quote_fr(),
            certificate_fr(),
        ] {
            assert!(d.word_count() > 5);
            let l = wordcraft_layout::layout(&d, &mut wordcraft_layout::LayoutCache::new(), &Default::default());
            assert!(!l.pages.is_empty());
        }
    }
}
