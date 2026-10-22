//! Fiche produit de la Boutique : ce que Steam sait d'un jeu, au-delà du prix.
//!
//! Deux appels Steam **sans clé**, faits en parallèle et hors du quota partagé du
//! comparateur :
//! - `IStoreBrowseService/GetItems` : toutes les captures, l'aperçu vidéo, les langues
//!   (voix, sous-titres), l'appréciation Steam et les tags ;
//! - `appdetails` : configuration requise, fonctionnalités (solo, coop, manette…),
//!   succès, classification d'âge, contenus additionnels, description complète.
//!
//! 🔑 **AUCUN HTML NE PASSE À L'INTERFACE.** Steam décrit ses jeux et leur configuration
//! en HTML. L'afficher tel quel (`v-html`) ouvrirait la WebView — qui a accès aux
//! commandes Tauri — à du contenu tiers. Ce module le convertit en blocs typés
//! (paragraphe, titre, puce, image) : le texte est décodé et rendu comme du texte, les
//! images ne sont gardées que si elles viennent des serveurs d'images de Steam.
//!
//! ⚠️ Les vraies bandes-annonces ne sont plus servies qu'en flux adaptatifs (DASH/HLS),
//! qu'il faudrait une bibliothèque de lecture pour lire. On ne garde que le
//! « microtrailer » : quelques secondes en mp4/webm, muettes, lisibles par la WebView.

use crate::locale;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

const GET_ITEMS: &str = "https://api.steampowered.com/IStoreBrowseService/GetItems/v1";
const IMAGES: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
const VIDEOS: &str = "https://video.akamai.steamstatic.com/store_trailers/";
/// Pays du catalogue Steam pour `appdetails` (voir `steam_store::CATALOGUE_STEAM`) :
/// aucun prix n'est lu ici, et Steam masque des fiches selon le pays.
const CATALOGUE: &str = "us";

/// Ce que Steam ajoute à la fiche produit.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FicheSteam {
    pub review_score: u32,
    pub review_percent: Option<u32>,
    pub review_count: u32,
    /// Tags dans la langue de l'interface (les plus votés d'abord).
    pub tags: Vec<String>,
    /// Les mêmes, en identifiants : ils servent à chercher les jeux similaires.
    pub tag_ids: Vec<u64>,
    pub captures: Vec<Capture>,
    pub apercu: Option<Apercu>,
    pub langues: Vec<Langue>,
    pub classification: Option<Classification>,
    /// Fonctionnalités Steam (solo, coop, manette, succès…), libellés traduits par Steam.
    pub categories: Vec<Categorie>,
    pub succes: Option<u32>,
    pub configuration: Option<Configuration>,
    /// Appids des contenus additionnels (12 au plus).
    pub dlc: Vec<u64>,
    pub site: Option<String>,
    pub description: Vec<Bloc>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub mini: String,
    pub grande: String,
}

/// Aperçu vidéo (« microtrailer ») : quelques secondes, sans son.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Apercu {
    pub nom: String,
    pub affiche: Option<String>,
    pub mp4: Option<String>,
    pub webm: Option<String>,
}

/// Une langue prise en charge. `code` est une étiquette BCP 47 : l'interface en tire
/// le nom dans sa propre langue (`Intl.DisplayNames`).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Langue {
    pub code: String,
    pub voix: bool,
    pub sous_titres: bool,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Classification {
    /// `PEGI`, `ESRB`, `USK`…
    pub systeme: String,
    pub note: String,
    /// Descripteurs tels que Steam les donne (en anglais) : traduits côté interface.
    pub descripteurs: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Categorie {
    pub id: u64,
    pub libelle: String,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    pub minimale: Vec<LigneConfig>,
    pub recommandee: Vec<LigneConfig>,
    /// Lignes sans étiquette (« Système 64 bits nécessaire », notes de l'éditeur).
    pub notes: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LigneConfig {
    pub libelle: String,
    pub valeur: String,
}

/// Un bloc de la description complète.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bloc {
    /// `p` (paragraphe), `h` (intertitre), `li` (puce), `img` ou `video` (boucle muette,
    /// comme sur la page Steam).
    pub genre: String,
    pub texte: String,
    pub url: Option<String>,
    /// Image fixe d'une vidéo (montrée seule si l'utilisateur réduit les animations).
    pub affiche: Option<String>,
}

/// La fiche Steam d'un jeu ; `None` si Steam n'a rien répondu du tout.
pub fn fiche(appid: u64, dir: &Path) -> Option<FicheSteam> {
    let (item, details, noms) = std::thread::scope(|s| {
        let item = s.spawn(|| get_items(appid));
        let details = s.spawn(|| appdetails(appid));
        let noms = super::vitrine::noms_tags(dir);
        (item.join().ok().flatten(), details.join().ok().flatten(), noms)
    });
    if item.is_none() && details.is_none() {
        return None;
    }
    let mut f = FicheSteam::default();
    if let Some(i) = &item {
        lire_item(&mut f, i, &noms);
    }
    if let Some(d) = &details {
        lire_details(&mut f, d, &locale::region());
    }
    Some(f)
}

fn get_items(appid: u64) -> Option<Value> {
    let entree = json!({
        "ids": [{ "appid": appid }],
        "context": { "language": locale::steam_langue(), "country_code": locale::region() },
        "data_request": {
            "include_screenshots": true,
            "include_trailers": true,
            "include_supported_languages": true,
            "include_reviews": true,
            "include_tag_count": 12,
        },
    });
    let racine: Value = ureq::get(GET_ITEMS)
        .query("input_json", &entree.to_string())
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    racine["response"]["store_items"].as_array()?.first().cloned()
}

fn appdetails(appid: u64) -> Option<Value> {
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={appid}&l={}&cc={CATALOGUE}",
        locale::steam_langue()
    );
    let racine: Value = ureq::get(&url).timeout(Duration::from_secs(15)).call().ok()?.into_json().ok()?;
    // Comme dans `steam_store::appdetails` : Steam peut répondre sous un autre appid
    // (éditions regroupées) — une seule entrée demandée, une seule reçue.
    let entree = racine
        .get(appid.to_string())
        .or_else(|| racine.as_object().filter(|o| o.len() == 1).and_then(|o| o.values().next()))?;
    entree["success"].as_bool().filter(|s| *s)?;
    entree.get("data").cloned()
}

/// Tags jamais affichés, ni utilisés pour les jeux similaires : contenu (nudité, sexe,
/// gore, violence) et généralités qui rapprocheraient n'importe quoi (solo,
/// multijoueur, bonne bande-son, atmosphérique…).
const TAGS_ECARTES: [u64; 11] = [6650, 12095, 4345, 4667, 113, 4182, 3859, 1756, 4166, 4191, 3871];

fn lire_item(f: &mut FicheSteam, item: &Value, noms: &HashMap<u64, String>) {
    let avis = &item["reviews"]["summary_filtered"];
    f.review_count = avis["review_count"].as_u64().unwrap_or(0) as u32;
    if f.review_count > 0 {
        f.review_score = avis["review_score"].as_u64().unwrap_or(0) as u32;
        f.review_percent = avis["percent_positive"].as_u64().map(|p| p as u32);
    }

    let ids: Vec<u64> = item["tags"]
        .as_array()
        .map(|t| t.iter().filter_map(|t| t["tagid"].as_u64()).collect())
        .unwrap_or_default();
    f.tag_ids = ids.iter().copied().filter(|id| !TAGS_ECARTES.contains(id)).take(6).collect();
    f.tags = f.tag_ids.iter().filter_map(|id| noms.get(id).cloned()).take(4).collect();

    f.captures = item["screenshots"]["all_ages_screenshots"]
        .as_array()
        .map(|liste| {
            liste
                .iter()
                .filter_map(|s| s["filename"].as_str())
                .filter(|f| f.contains(".jpg"))
                .map(FicheSteam::adresses)
                .collect()
        })
        .unwrap_or_default();

    f.apercu = item["trailers"]["highlights"].as_array().and_then(|t| t.first()).map(|t| {
        let micro = t["microtrailer"].as_array().cloned().unwrap_or_default();
        let fichier = |genre: &str| {
            micro
                .iter()
                .find(|m| m["type"].as_str() == Some(genre))
                .and_then(|m| m["filename"].as_str())
                .map(|f| format!("{VIDEOS}{f}"))
        };
        Apercu {
            nom: t["trailer_name"].as_str().unwrap_or_default().to_string(),
            affiche: t["screenshot_full"].as_str().map(|s| format!("{IMAGES}steam/apps/{s}")),
            mp4: fichier("video/mp4"),
            webm: fichier("video/webm"),
        }
    });
    // Un aperçu sans vidéo ne sert à rien.
    if f.apercu.as_ref().is_some_and(|a| a.mp4.is_none() && a.webm.is_none()) {
        f.apercu = None;
    }

    f.langues = item["supported_languages"]
        .as_array()
        .map(|l| {
            l.iter()
                .filter(|l| l["supported"].as_bool().unwrap_or(true))
                .filter_map(|l| {
                    Some(Langue {
                        code: code_langue(l["elanguage"].as_i64()?)?.to_string(),
                        voix: l["full_audio"].as_bool().unwrap_or(false),
                        sous_titres: l["subtitles"].as_bool().unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
}

impl FicheSteam {
    /// `steam/apps/…/ss_xxx.jpg?t=…` → miniature 600×338 et grande image 1920×1080.
    /// Steam sert ces tailles en suffixant le nom de fichier (vérifié) ; l'original pèse
    /// 1 à 2 Mo, la miniature 50 Ko.
    fn adresses(chemin: &str) -> Capture {
        let (base, requete) = chemin.split_once('?').map_or((chemin, ""), |(b, q)| (b, q));
        let sans_ext = base.strip_suffix(".jpg").unwrap_or(base);
        let suffixe = if requete.is_empty() { String::new() } else { format!("?{requete}") };
        Capture {
            mini: format!("{IMAGES}{sans_ext}.600x338.jpg{suffixe}"),
            grande: format!("{IMAGES}{sans_ext}.1920x1080.jpg{suffixe}"),
        }
    }
}

/// Énumération `ELanguage` de Steam → étiquette BCP 47. Relevé sur la fiche de
/// Cyberpunk 2077 : l'ordre de `GetItems` (0, 2, 3, 1, 5, 25…) suit celui de la liste
/// lisible d'`appdetails` (anglais, français, italien, allemand, espagnol, arabe…).
fn code_langue(e: i64) -> Option<&'static str> {
    Some(match e {
        0 => "en",
        1 => "de",
        2 => "fr",
        3 => "it",
        4 => "ko",
        5 => "es",
        6 => "zh-Hans",
        7 => "zh-Hant",
        8 => "ru",
        9 => "th",
        10 => "ja",
        11 => "pt",
        12 => "pl",
        13 => "da",
        14 => "nl",
        15 => "fi",
        16 => "no",
        17 => "sv",
        18 => "hu",
        19 => "cs",
        20 => "ro",
        21 => "tr",
        22 => "pt-BR",
        23 => "bg",
        24 => "el",
        25 => "ar",
        26 => "uk",
        27 => "es-419",
        28 => "vi",
        29 => "id",
        _ => return None,
    })
}

fn lire_details(f: &mut FicheSteam, d: &Value, region: &str) {
    f.categories = d["categories"]
        .as_array()
        .map(|c| {
            c.iter()
                .filter_map(|c| {
                    Some(Categorie {
                        id: c["id"].as_u64()?,
                        libelle: c["description"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    f.succes = d["achievements"]["total"].as_u64().filter(|n| *n > 0).map(|n| n as u32);
    f.dlc = d["dlc"]
        .as_array()
        .map(|l| l.iter().filter_map(Value::as_u64).take(12).collect())
        .unwrap_or_default();
    f.site = d["website"].as_str().filter(|s| s.starts_with("https://")).map(String::from);
    f.classification = classification(&d["ratings"], region);
    f.configuration = configuration(&d["pc_requirements"]);
    f.description = blocs(d["about_the_game"].as_str().unwrap_or_default());
}

/// La classification d'âge de la région : PEGI en Europe, USK en Allemagne, ESRB en
/// Amérique du Nord… avec repli sur PEGI puis ESRB quand l'éditeur n'a pas donné celle
/// du pays.
fn classification(notes: &Value, region: &str) -> Option<Classification> {
    let prefere = match region {
        "DE" => "usk",
        "US" | "CA" | "MX" => "esrb",
        "AU" | "NZ" => "oflc",
        "BR" => "dejus",
        _ => "pegi",
    };
    [prefere, "pegi", "esrb"].iter().find_map(|systeme| {
        let n = notes.get(*systeme)?;
        let note = n["rating"].as_str().filter(|r| !r.is_empty())?;
        Some(Classification {
            systeme: systeme.to_uppercase(),
            note: note.to_uppercase(),
            descripteurs: n["descriptors"]
                .as_str()
                .unwrap_or_default()
                .split(['\r', '\n'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        })
    })
}

/// Configuration requise PC : listes « libellé : valeur » tirées du HTML de Steam.
/// Les libellés sont ceux de Steam, déjà dans la langue demandée.
fn configuration(pc: &Value) -> Option<Configuration> {
    let mut c = Configuration::default();
    for (cle, cible) in [("minimum", 0), ("recommended", 1)] {
        let Some(html) = pc.get(cle).and_then(Value::as_str) else { continue };
        for ligne in lignes_html(html) {
            match ligne.split_once(':') {
                // « Minimale : » / « Recommandée : » : le titre de la colonne, qu'on a déjà.
                Some((_, v)) if v.trim().is_empty() => {}
                Some((libelle, valeur)) if libelle.chars().count() <= 40 => {
                    // « Système d'exploitation * » : l'astérisque renvoie à une note de Steam
                    // sur la fin de Windows 7, qui n'est pas dans la réponse.
                    let libelle = libelle.trim().trim_end_matches('*').trim();
                    let l = LigneConfig { libelle: libelle.to_string(), valeur: valeur.trim().to_string() };
                    if est_note(&l.libelle) {
                        ajouter_note(&mut c.notes, l.valeur);
                    } else if cible == 0 {
                        c.minimale.push(l);
                    } else {
                        c.recommandee.push(l);
                    }
                }
                _ => ajouter_note(&mut c.notes, ligne),
            }
        }
    }
    (!c.minimale.is_empty() || !c.recommandee.is_empty()).then_some(c)
}

/// Les deux colonnes répètent souvent les mêmes notes, parfois l'une en version longue
/// (« SSD required. Attention: … » à gauche, « SSD required. » à droite) : une note déjà
/// contenue dans une autre n'est pas ajoutée.
fn ajouter_note(notes: &mut Vec<String>, note: String) {
    if notes.iter().any(|n| n.contains(&note)) {
        return;
    }
    notes.retain(|n| !note.contains(n.as_str()));
    notes.push(note);
}

/// « Notes supplémentaires » / « Additional Notes » : texte libre, pas une ligne de tableau.
fn est_note(libelle: &str) -> bool {
    let l = libelle.to_lowercase();
    l.starts_with("notes") || l.starts_with("additional")
}

/// Une ligne de texte par élément de liste (ou par saut de ligne, faute de liste).
fn lignes_html(html: &str) -> Vec<String> {
    let mut lignes = Vec::new();
    let mut courant = String::new();
    for jeton in jetons(html) {
        match jeton {
            Jeton::Texte(t) => courant.push_str(&t),
            Jeton::Balise { nom, .. } if matches!(nom.as_str(), "li" | "br" | "ul" | "p") => {
                let l = espaces(&courant);
                if !l.is_empty() {
                    lignes.push(l);
                }
                courant.clear();
            }
            _ => {}
        }
    }
    let l = espaces(&courant);
    if !l.is_empty() {
        lignes.push(l);
    }
    lignes
}

/// Nombre de blocs gardés de la description : au-delà, c'est la page Steam qu'il faut lire.
const MAX_BLOCS: usize = 120;

/// La description complète en blocs sûrs. Seuls les images et les vidéos des serveurs de
/// Steam sont gardées ; liens, styles et tout le reste disparaissent, leur texte restant.
///
/// ⚠️ Beaucoup d'éditeurs ne mettent QUE des images et des vidéos en boucle dans cette
/// description (celle de Cyberpunk 2077 ne contient pas un mot : son texte est dans les
/// images). Sans les vidéos, il n'en restait qu'une image.
fn blocs(html: &str) -> Vec<Bloc> {
    let mut blocs: Vec<Bloc> = Vec::new();
    let mut texte = String::new();
    let mut genre = "p";
    // Vidéo en cours de lecture des balises : (affiche, fichier mp4).
    let mut video: Option<(Option<String>, Option<String>)> = None;
    let vider = |blocs: &mut Vec<Bloc>, texte: &mut String, genre: &str| {
        let t = texte
            .split('\n')
            .map(espaces)
            .collect::<Vec<_>>()
            .join("\n")
            .trim_matches('\n')
            .to_string();
        if !t.is_empty() {
            blocs.push(Bloc { genre: genre.to_string(), texte: t, ..Default::default() });
        }
        texte.clear();
    };
    for jeton in jetons(html) {
        match jeton {
            Jeton::Texte(t) => texte.push_str(&t),
            Jeton::Balise { nom, fermante, src, poster } => match nom.as_str() {
                "br" => texte.push('\n'),
                "p" | "div" | "ul" | "ol" => {
                    vider(&mut blocs, &mut texte, genre);
                    genre = "p";
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    vider(&mut blocs, &mut texte, genre);
                    genre = if fermante { "p" } else { "h" };
                }
                "li" => {
                    vider(&mut blocs, &mut texte, genre);
                    genre = if fermante { "p" } else { "li" };
                }
                "img" => {
                    if let Some(url) = src.filter(|u| image_steam(u)) {
                        vider(&mut blocs, &mut texte, genre);
                        blocs.push(Bloc { genre: "img".into(), url: Some(url), ..Default::default() });
                    }
                }
                "video" if !fermante => {
                    vider(&mut blocs, &mut texte, genre);
                    video = Some((poster.filter(|u| image_steam(u)), None));
                }
                "source" => {
                    if let (Some(v), Some(url)) = (video.as_mut(), src) {
                        let chemin = url.split('?').next().unwrap_or_default();
                        if v.1.is_none() && chemin.ends_with(".mp4") && image_steam(&url) {
                            v.1 = Some(url);
                        }
                    }
                }
                "video" => {
                    match video.take() {
                        Some((affiche, Some(url))) => {
                            blocs.push(Bloc { genre: "video".into(), url: Some(url), affiche, ..Default::default() })
                        }
                        // Pas de mp4 lisible : l'affiche seule vaut mieux que rien.
                        Some((Some(affiche), None)) => {
                            blocs.push(Bloc { genre: "img".into(), url: Some(affiche), ..Default::default() })
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
        }
        if blocs.len() >= MAX_BLOCS {
            return blocs;
        }
    }
    vider(&mut blocs, &mut texte, genre);
    blocs
}

/// Une image n'est gardée que si elle vient des serveurs d'images de Steam, en https.
fn image_steam(url: &str) -> bool {
    let Some(reste) = url.strip_prefix("https://") else { return false };
    let hote = reste.split('/').next().unwrap_or_default();
    hote.ends_with(".steamstatic.com") || hote == "steamcdn-a.akamaihd.net"
}

enum Jeton {
    Texte(String),
    Balise { nom: String, fermante: bool, src: Option<String>, poster: Option<String> },
}

/// Découpe du HTML en texte et balises. Volontairement minimal : on ne reconstruit pas
/// d'arbre, on ne garde que le nom de la balise, son sens et un éventuel `src`.
fn jetons(html: &str) -> Vec<Jeton> {
    let mut out = Vec::new();
    let mut reste = html;
    while !reste.is_empty() {
        match reste.find('<') {
            Some(0) => {
                let Some(fin) = reste.find('>') else {
                    out.push(Jeton::Texte(decoder(reste)));
                    break;
                };
                let dedans = &reste[1..fin];
                reste = &reste[fin + 1..];
                let fermante = dedans.starts_with('/');
                let corps = dedans.trim_start_matches('/');
                let nom: String = corps
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect::<String>()
                    .to_ascii_lowercase();
                if nom.is_empty() {
                    continue; // commentaire, doctype…
                }
                let src = matches!(nom.as_str(), "img" | "source").then(|| attribut(corps, "src")).flatten();
                let poster = (nom == "video").then(|| attribut(corps, "poster")).flatten();
                out.push(Jeton::Balise { nom, fermante, src, poster });
            }
            Some(i) => {
                out.push(Jeton::Texte(decoder(&reste[..i])));
                reste = &reste[i..];
            }
            None => {
                out.push(Jeton::Texte(decoder(reste)));
                break;
            }
        }
    }
    out
}

/// Valeur d'un attribut entre guillemets (`src="…"` ou `src='…'`).
fn attribut(balise: &str, nom: &str) -> Option<String> {
    let bas = balise.to_ascii_lowercase();
    let mut debut = 0;
    while let Some(i) = bas[debut..].find(nom) {
        let pos = debut + i + nom.len();
        let suite = balise[pos..].trim_start();
        if let Some(apres_egal) = suite.strip_prefix('=') {
            let apres_egal = apres_egal.trim_start();
            let guillemet = apres_egal.chars().next()?;
            if guillemet == '"' || guillemet == '\'' {
                let fin = apres_egal[1..].find(guillemet)?;
                return Some(decoder(&apres_egal[1..1 + fin]));
            }
        }
        debut = pos;
    }
    None
}

/// Décode les entités HTML courantes (`&amp;`, `&#39;`, `&#x27;`, `&nbsp;`…).
fn decoder(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut reste = s;
    while let Some(i) = reste.find('&') {
        out.push_str(&reste[..i]);
        reste = &reste[i..];
        let Some(fin) = reste.find(';').filter(|f| *f <= 10) else {
            out.push('&');
            reste = &reste[1..];
            continue;
        };
        let entite = &reste[1..fin];
        let car = match entite {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ if entite.starts_with("#x") || entite.starts_with("#X") => {
                u32::from_str_radix(&entite[2..], 16).ok().and_then(char::from_u32)
            }
            _ if entite.starts_with('#') => entite[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match car {
            Some(c) => {
                out.push(c);
                reste = &reste[fin + 1..];
            }
            None => {
                out.push('&');
                reste = &reste[1..];
            }
        }
    }
    out.push_str(reste);
    out
}

/// Espaces repliés et retirés aux deux bouts (le HTML ne compte pas les espaces).
fn espaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extrait réel de `pc_requirements` (Cyberpunk 2077, en français, 22/10/2026).
    const CONFIG_MIN: &str = "<strong>Minimale :</strong><br><ul class=\"bb_ul\"><li>Système d'exploitation et processeur 64 bits nécessaires<br></li><li><strong>Système d'exploitation :</strong> 64-bit Windows 10<br></li><li><strong>Processeur :</strong> Core i7-6700 or Ryzen 5 1600<br></li><li><strong>Mémoire vive :</strong> 12 GB de mémoire<br></li><li><strong>Espace disque :</strong> 70 GB d'espace disque disponible<br></li><li><strong>Notes supplémentaires :</strong> SSD required.</li></ul>";
    const CONFIG_REC: &str = "<strong>Recommandée :</strong><br><ul class=\"bb_ul\"><li>Système d'exploitation et processeur 64 bits nécessaires<br></li><li><strong>Processeur :</strong> Core i7-12700 or Ryzen 7 7800X3D<br></li><li><strong>Notes supplémentaires :</strong> SSD required.</li></ul>";

    #[test]
    fn lit_la_configuration_requise() {
        let c = configuration(&json!({ "minimum": CONFIG_MIN, "recommended": CONFIG_REC })).unwrap();
        assert_eq!(c.minimale.len(), 4);
        assert_eq!(c.minimale[0], LigneConfig { libelle: "Système d'exploitation".into(), valeur: "64-bit Windows 10".into() });
        assert_eq!(c.minimale[3].valeur, "70 GB d'espace disque disponible");
        assert_eq!(c.recommandee, vec![LigneConfig { libelle: "Processeur".into(), valeur: "Core i7-12700 or Ryzen 7 7800X3D".into() }]);
        // Les deux colonnes répètent les mêmes notes : on ne les garde qu'une fois.
        assert_eq!(c.notes, vec!["Système d'exploitation et processeur 64 bits nécessaires".to_string(), "SSD required.".to_string()]);
    }

    #[test]
    fn pas_de_configuration_sans_lignes() {
        // Steam renvoie un tableau vide pour les jeux qui n'en déclarent pas.
        assert_eq!(configuration(&json!([])), None);
    }

    #[test]
    fn la_description_devient_des_blocs_surs() {
        let html = "<h2 class=\"bb_tag\">L&#39;AVENTURE</h2><p class=\"bb_paragraph\">Night&nbsp;City &amp; ses <a href=\"https://exemple.org\">rues</a>.<br>Deuxième ligne.</p>\
            <ul class=\"bb_ul\"><li>Un <strong>monde</strong> ouvert</li><li>Des choix</li></ul>\
            <img class=\"bb_img\" src=\"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1/extras/a.gif?t=1\" />\
            <img src=\"https://exemple.org/pisteur.png\"><script>alert(1)</script><p onclick=\"x()\">Fin</p>";
        let b = blocs(html);
        let resume: Vec<(&str, &str)> = b.iter().map(|b| (b.genre.as_str(), b.texte.as_str())).collect();
        assert_eq!(
            resume,
            vec![
                ("h", "L'AVENTURE"),
                ("p", "Night City & ses rues.\nDeuxième ligne."),
                ("li", "Un monde ouvert"),
                ("li", "Des choix"),
                ("img", ""),
                // Le texte d'un <script> reste du texte : il ne sera jamais exécuté,
                // l'interface l'affiche comme tel.
                ("p", "alert(1)"),
                ("p", "Fin"),
            ]
        );
        assert!(b[4].url.as_deref().unwrap().starts_with("https://shared.akamai.steamstatic.com/"));
        // L'image d'un autre hôte a disparu.
        assert_eq!(b.iter().filter(|b| b.genre == "img").count(), 1);
    }

    /// Extrait réel (Hades, 22/10/2026) : une vidéo en boucle, puis du texte.
    #[test]
    fn garde_les_videos_de_steam() {
        let html = "<span class=\"bb_img_ctn\"><video class=\"bb_img\" autoplay muted loop playsinline poster=\"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1145360/extras/c2.poster.avif?t=1\" width=600 height=338 >\
            <source src=\"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1145360/extras/c2.webm?t=1\" type=\"video/webm; codecs=vp9\">\
            <source src=\"https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1145360/extras/c2.mp4?t=1\" type=\"video/mp4\"></video></span><br><strong>Hades</strong> est un rogue-like.\
            <video poster=\"https://ailleurs.example/p.jpg\"><source src=\"https://ailleurs.example/v.mp4\"></video>";
        let b = blocs(html);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].genre, "video");
        assert!(b[0].url.as_deref().unwrap().ends_with("/c2.mp4?t=1"));
        assert!(b[0].affiche.as_deref().unwrap().ends_with("/c2.poster.avif?t=1"));
        assert_eq!((b[1].genre.as_str(), b[1].texte.as_str()), ("p", "Hades est un rogue-like."));
    }

    #[test]
    fn decode_les_entites() {
        assert_eq!(decoder("L&#39;&#x27;a &amp; b &lt;c&gt; &bogus; &"), "L''a & b <c> &bogus; &");
    }

    #[test]
    fn adresses_des_captures() {
        let c = FicheSteam::adresses("steam/apps/1091500/ss_2f64.jpg?t=17");
        assert_eq!(c.mini, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1091500/ss_2f64.600x338.jpg?t=17");
        assert_eq!(c.grande, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1091500/ss_2f64.1920x1080.jpg?t=17");
    }

    #[test]
    fn langues_et_apercu() {
        let item = json!({
            "reviews": { "summary_filtered": { "review_count": 10, "percent_positive": 87, "review_score": 8 } },
            "tags": [ { "tagid": 4115 }, { "tagid": 6650 }, { "tagid": 1695 }, { "tagid": 4182 }, { "tagid": 122 } ],
            "screenshots": { "all_ages_screenshots": [ { "filename": "steam/apps/1/ss_a.jpg?t=1" } ] },
            "trailers": { "highlights": [ { "trailer_name": "Rêves brisés", "screenshot_full": "257/abc/movie_full.jpg",
                "microtrailer": [ { "filename": "1/2/micro.webm", "type": "video/webm" }, { "filename": "1/2/micro.mp4", "type": "video/mp4" } ] } ] },
            "supported_languages": [
                { "elanguage": 0, "supported": true, "full_audio": true, "subtitles": true },
                { "elanguage": 2, "supported": true, "full_audio": false, "subtitles": true },
                { "elanguage": 999, "supported": true }
            ]
        });
        let noms: HashMap<u64, String> = [(4115, "Cyberpunk"), (1695, "Monde ouvert"), (122, "RPG")]
            .into_iter()
            .map(|(i, n)| (i, n.to_string()))
            .collect();
        let mut f = FicheSteam::default();
        lire_item(&mut f, &item, &noms);
        assert_eq!(f.tag_ids, vec![4115, 1695, 122]);
        assert_eq!(f.tags, vec!["Cyberpunk", "Monde ouvert", "RPG"]);
        assert_eq!(f.captures.len(), 1);
        assert!(f.captures[0].mini.ends_with("ss_a.600x338.jpg?t=1"));
        let a = f.apercu.unwrap();
        assert_eq!(a.mp4.as_deref(), Some("https://video.akamai.steamstatic.com/store_trailers/1/2/micro.mp4"));
        assert_eq!(a.affiche.as_deref(), Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/257/abc/movie_full.jpg"));
        assert_eq!(f.langues, vec![
            Langue { code: "en".into(), voix: true, sous_titres: true },
            Langue { code: "fr".into(), voix: false, sous_titres: true },
        ]);
        assert_eq!((f.review_score, f.review_percent), (8, Some(87)));
    }

    #[test]
    fn classification_de_la_region() {
        let notes = json!({
            "pegi": { "rating": "18", "descriptors": "Violence\r\nSex\r\nBad Language" },
            "esrb": { "rating": "m", "descriptors": "Blood and Gore" }
        });
        let fr = classification(&notes, "FR").unwrap();
        assert_eq!((fr.systeme.as_str(), fr.note.as_str()), ("PEGI", "18"));
        assert_eq!(fr.descripteurs, vec!["Violence", "Sex", "Bad Language"]);
        let us = classification(&notes, "US").unwrap();
        assert_eq!((us.systeme.as_str(), us.note.as_str()), ("ESRB", "M"));
        // Pas d'USK fourni : l'Allemagne retombe sur PEGI.
        assert_eq!(classification(&notes, "DE").unwrap().systeme, "PEGI");
    }
}
