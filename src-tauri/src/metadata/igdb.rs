//! Métadonnées via IGDB (base de données de jeux cross-plateforme), à travers notre
//! mini-proxy Cloudflare qui détient le token Twitch. Source UNIQUE des infos
//! descriptives (genre, description, captures, jaquette de repli, hero, studio, année)
//! pour TOUS les launchers — là où Steam Store est aveugle aux jeux hors-Steam
//! (Fortnite, Valorant, WoW…). Les données propres au joueur (temps de jeu, installé,
//! possédé) restent fournies par les launchers, tout comme leur jaquette native
//! (IGDB ne sert que de repli pour la jaquette, décision utilisateur).
//!
//! Deux chemins de correspondance (validés en test réel, 92 % de couverture) :
//!   - **Steam** : match EXACT par appid via `external_games` (`external_game_source = 1`),
//!     en masse (jusqu'à 500 jeux/requête).
//!   - **Autres launchers** : match exact sur le nom **ou un nom alternatif**
//!     (`where name ~ "…" | alternative_names.name ~ "…"`), repli `search` avec sélection
//!     du nom normalisé (évite les DLC/jeux voisins remontés par `search`).
//!     🔑 L'alias est INTERROGÉ, pas seulement comparé : IGDB n'ouvre pas toujours une
//!     fiche par titre commercial, et sa recherche ne remonte pas l'entrée qui porte
//!     l'alias — « Overwatch 2 » ne ramène que des DLC (cf. `name_meta`).

use crate::models::GameDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROXY_URL: &str = "https://torii-igdb-proxy.toriiapp.workers.dev";
const IMG: &str = "https://images.igdb.com/igdb/image/upload";
const STEAM_SOURCE: u8 = 1; // external_game_source : Steam
const CALL_DELAY_MS: u64 = 300; // < 4 req/s (limite IGDB)
const STEAM_CHUNK: usize = 400; // < 500 résultats/requête
const NONSTEAM_BATCH: usize = 12; // taille des lots émis au front

/// Champs IGDB récupérés pour chaque jeu (partagés entre les deux chemins).
const FIELDS: &str = "fields id, name, alternative_names.name, parent_game, genres.name, \
themes, game_modes, player_perspectives, external_games.uid, external_games.external_game_source, \
summary, cover.image_id, artworks.image_id, screenshots.image_id, \
involved_companies.company.name, involved_companies.developer, first_release_date;";

/// Métadonnées descriptives d'un jeu, issues d'IGDB.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgdbMeta {
    pub genre: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub hero_url: Option<String>,
    pub developer: Option<String>,
    pub year: Option<i64>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    /// Identifiant de la fiche IGDB. Une prochaine évolution du cache pourra redemander
    /// les fiches par lots de 500 (`where id = (…)`) au lieu de rechercher à nouveau
    /// chaque titre hors Steam, un par un — c'est ce qu'a coûté l'arrivée de `classement`.
    #[serde(default)]
    pub igdb_id: Option<u64>,
    /// Appid Steam du jeu d'après IGDB (`external_games`), y compris pour un jeu acheté
    /// ailleurs : c'est par lui qu'un jeu Epic ou GOG reçoit ses tags Steam.
    #[serde(default)]
    pub steam_appid: Option<u64>,
    /// Genres, thèmes, modes et vues d'IGDB, en identifiants bruts.
    ///
    /// 🔑 `None` = fiche d'avant les catégories, à compléter : `fill_metadata` l'affiche
    /// tout de suite PUIS la redemande. Une fiche complète a toujours `Some`, même vide.
    /// C'est ce marqueur qui évite de changer le nom du cache — le changer aurait vidé
    /// l'écran de tout le monde le temps de tout retélécharger.
    #[serde(default)]
    pub classement: Option<Classement>,
}

/// Le classement d'un jeu chez IGDB, tel quel. Converti en tags Steam à la sortie
/// (`categories::tags_igdb`) et pas à l'écriture : corriger la table de correspondance ne
/// coûtera pas un retéléchargement, comme pour la traduction des genres.
///
/// ⚠️ Quatre listes et pas une : les identifiants se recouvrent d'une table à l'autre
/// (31 = « Adventure » chez les genres, « Drama » chez les thèmes).
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
pub struct Classement {
    #[serde(default)]
    pub genres: Vec<u64>,
    #[serde(default)]
    pub themes: Vec<u64>,
    #[serde(default)]
    pub modes: Vec<u64>,
    #[serde(default)]
    pub vues: Vec<u64>,
}

/// id du jeu → métadonnées (None = cherché mais introuvable dans IGDB, pour ne pas re-chercher).
type MetaCache = HashMap<String, Option<IgdbMeta>>;

fn cache_file(dir: &Path) -> PathBuf {
    // Versionné : incrémenter si le schéma `IgdbMeta` ou la stratégie de correspondance change.
    dir.join("igdb_meta_cache_v4.json")
}

/// Les caches des versions précédentes, du plus récent au plus ancien.
///
/// `v1` : d'avant la distinction panne / absence (cf. `Reponse`) — ses « introuvables »
/// contiennent des pannes réseau prises pour des absences.
/// `v2` : d'avant le pliage des accents (cf. `norm`) — ses « introuvables » contiennent
/// des jeux qui ne différaient de leur fiche que par une lettre accentuée.
/// `v3` : d'avant l'INTERROGATION des noms alternatifs. La v3 savait déjà les comparer,
/// mais IGDB ne les renvoyait pas : ses « introuvables » contiennent tous les jeux qu'IGDB
/// ne connaît que sous un alias, Overwatch 2 en tête.
const ANCIENS_CACHES: [&str; 3] = [
    "igdb_meta_cache_v3.json",
    "igdb_meta_cache_v2.json",
    "igdb_meta_cache_v1.json",
];

fn lire(chemin: &Path) -> Option<MetaCache> {
    std::fs::read_to_string(chemin)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

/// Lit le cache, en réparant au passage les dégâts des versions précédentes.
///
/// 🔑 POURQUOI UNE MIGRATION ET PAS UN SIMPLE CHANGEMENT DE NUMÉRO. Un cache qui n'expire
/// jamais garde ses erreurs pour toujours. Deux fois déjà, un jeu s'est retrouvé marqué
/// « introuvable » à tort : une coupure réseau prise pour une absence, puis un titre
/// commercial qu'IGDB ne connaît que comme alias. Dans les deux cas le jeu n'était **plus
/// jamais** recherché, donc sans jaquette ni description, définitivement.
///
/// Repartir de zéro réparerait tout, mais ferait re-télécharger la totalité des fiches à
/// tout le monde, d'un coup, à travers le proxy — cher pour lui, long pour eux, et inutile
/// puisque les fiches trouvées sont bonnes. Or les dégâts ont toujours la même forme,
/// `Some(None)`. On garde donc **toutes les entrées résolues** et on ne jette que les
/// « introuvables ». Les jeux réellement absents d'IGDB seront cherchés une fois de plus
/// puis remémorisés : le prix est payé une seule fois, par les seuls jeux concernés.
///
/// ⚠️ LES ANCIENS FICHIERS SONT CONSERVÉS, et ce n'est pas de la négligence : si
/// l'écriture du nouveau est interrompue, sa relecture échoue et la migration se rejoue
/// depuis le précédent au lancement suivant. Sans eux, ce cas dégénérerait en cache vide —
/// exactement le re-téléchargement massif qu'on cherche à éviter. « Vider le cache » les
/// supprime tous, comme n'importe quel autre fichier de cache.
fn load_cache(dir: &Path) -> MetaCache {
    if let Some(cache) = lire(&cache_file(dir)) {
        return cache;
    }
    for ancien in ANCIENS_CACHES {
        if let Some(vieux) = lire(&dir.join(ancien)) {
            let repare: MetaCache = vieux.into_iter().filter(|(_, meta)| meta.is_some()).collect();
            save_cache(dir, &repare);
            return repare;
        }
    }
    MetaCache::default()
}

fn save_cache(dir: &Path, cache: &MetaCache) {
    if std::fs::create_dir_all(dir).is_ok() {
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = std::fs::write(cache_file(dir), json);
        }
    }
}

/// Quand chaque « introuvable » a été constaté (id du jeu → horodatage Unix).
///
/// 🔑 Un jeu absent d'IGDB le jour de sa sortie peut y entrer la semaine suivante. Tant
/// que le cache ne savait pas DE QUAND datait une absence, il la gardait pour toujours.
/// Fichier à part plutôt qu'un changement de format du cache : la fiche d'un jeu trouvé
/// ne change pas de forme, et le cache existant reste lisible tel quel.
type Absences = HashMap<String, u64>;

/// Délai avant de rechercher de nouveau un jeu introuvable.
const DUREE_ABSENT: u64 = 7 * 86_400;

fn absences_file(dir: &Path) -> PathBuf {
    dir.join("igdb_absents_cache_v1.json")
}

fn load_absences(dir: &Path) -> Absences {
    std::fs::read_to_string(absences_file(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_absences(dir: &Path, absences: &Absences) {
    if let Ok(json) = serde_json::to_string(absences) {
        let _ = std::fs::write(absences_file(dir), json);
    }
}

fn maintenant() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Oublie un jeu (bouton « Actualiser les infos ») : il sera recherché au prochain passage.
pub fn oublier(dir: &Path, id: &str) {
    let mut cache = load_cache(dir);
    if cache.remove(id).is_some() {
        save_cache(dir, &cache);
    }
    let mut liens = load_liens(dir);
    if liens.remove(id).is_some() {
        save_liens(dir, &liens);
    }
}

/// Lien d'un jeu acheté hors de Steam vers sa fiche Steam (id du jeu → lien).
type Liens = HashMap<String, Lien>;

#[derive(Serialize, Deserialize)]
struct Lien {
    appid: Option<u64>,
    /// Quand le lien a été établi (horodatage Unix).
    t: u64,
}

/// Un lien trouvé se revérifie au bout d'un mois, une absence au bout d'une semaine.
const DUREE_LIEN: u64 = 30 * 86_400;

fn liens_file(dir: &Path) -> PathBuf {
    dir.join("igdb_steam_cache_v1.json")
}

fn load_liens(dir: &Path) -> Liens {
    std::fs::read_to_string(liens_file(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_liens(dir: &Path, liens: &Liens) {
    if let Ok(json) = serde_json::to_string(liens) {
        let _ = std::fs::write(liens_file(dir), json);
    }
}

/// L'appid Steam d'un jeu acheté ailleurs (Control sur Epic), d'après IGDB. Mémorisé par
/// jeu ; rien n'est retenu d'une panne.
///
/// 🔑 MÊMES EXIGENCES QUE LA DESCRIPTION, ET UNE DE PLUS. Le nom doit correspondre
/// exactement, nom principal ou alternatif (comme `name_meta`), sans repli sur `search` ;
/// DLC et éditions sont écartés. Et un seul identifiant Steam doit ressortir : IGDB
/// connaît deux « Hades », dont un seul est sur Steam — c'est celui-là ; deux jeux du même
/// nom tous deux sur Steam ne donnent aucun lien.
pub fn steam_appid(id: &str, titre: &str, dir: &Path) -> Option<u64> {
    let mut liens = load_liens(dir);
    let t = maintenant();
    if let Some(l) = liens.get(id) {
        let duree = if l.appid.is_some() { DUREE_LIEN } else { DUREE_ABSENT };
        if t.saturating_sub(l.t) < duree {
            return l.appid;
        }
    }
    let appid = chercher_steam(titre)?;
    liens.insert(id.to_string(), Lien { appid, t });
    save_liens(dir, &liens);
    appid
}

/// Les pages d'un jeu qu'on peut envoyer à quelqu'un, d'après IGDB.
#[derive(Default, Debug, PartialEq)]
pub struct Pages {
    pub steam: Option<String>,
    pub epic: Option<String>,
    pub gog: Option<String>,
    pub site: Option<String>,
}

/// Source `external_games` : Epic Games Store (Steam = `STEAM_SOURCE`, GOG = 5).
const EPIC_SOURCE: u64 = 26;
const GOG_SOURCE: u64 = 5;
/// Types de `websites` : site officiel, Steam, Epic, GOG.
const SITE_OFFICIEL: u64 = 1;
const SITE_STEAM: u64 = 13;
const SITE_EPIC: u64 = 16;
const SITE_GOG: u64 = 17;

/// Le lien à partager pour un jeu (bouton « Partager » des fiches) : sa page Steam, sinon
/// Epic, sinon GOG, sinon son site officiel. `None` si IGDB ne connaît rien de lui.
///
/// Le jeu est retrouvé par sa fiche en cache (jeu de la bibliothèque, `id`), sinon par son
/// titre, avec la même exigence de nom exact que l'enrichissement (`name_meta`).
pub fn lien_partage(id: Option<&str>, titre: &str, dir: &Path) -> Option<String> {
    let en_cache = id.and_then(|id| load_cache(dir).get(id).cloned().flatten()).and_then(|m| m.igdb_id);
    let igdb_id = match en_cache {
        Some(i) => i,
        None => match name_meta(titre, false) {
            Issue::Trouve(meta) => meta.igdb_id?,
            _ => return None,
        },
    };
    let body = format!(
        "fields websites.url, websites.type, external_games.url, external_games.uid, \
         external_games.external_game_source; where id = {igdb_id};"
    );
    let Reponse::Corps(Value::Array(jeux)) = query("games", &body) else { return None };
    let pages = lire_pages(jeux.first()?);
    pages.steam.or(pages.epic).or(pages.gog).or(pages.site)
}

fn lire_pages(g: &Value) -> Pages {
    let mut p = Pages::default();
    let liens = g["external_games"].as_array().into_iter().flatten();
    for e in liens {
        let source = e["external_game_source"].as_u64();
        if source == Some(STEAM_SOURCE as u64) && p.steam.is_none() {
            p.steam = e["uid"].as_str().and_then(|u| u.parse::<u64>().ok()).map(page_steam);
        } else if source == Some(EPIC_SOURCE) && p.epic.is_none() {
            p.epic = e["url"].as_str().and_then(page_epic);
        } else if source == Some(GOG_SOURCE) && p.gog.is_none() {
            p.gog = e["url"].as_str().filter(|u| u.contains("gog.com/")).map(String::from);
        }
    }
    for w in g["websites"].as_array().into_iter().flatten() {
        let Some(url) = w["url"].as_str() else { continue };
        match w["type"].as_u64() {
            Some(SITE_STEAM) if p.steam.is_none() => p.steam = Some(url.to_string()),
            Some(SITE_EPIC) if p.epic.is_none() => p.epic = page_epic(url),
            Some(SITE_GOG) if p.gog.is_none() => p.gog = Some(url.to_string()),
            Some(SITE_OFFICIEL) if p.site.is_none() => p.site = Some(url.to_string()),
            _ => {}
        }
    }
    p
}

fn page_steam(appid: u64) -> String {
    format!("https://store.steampowered.com/app/{appid}/")
}

/// La page Epic d'un jeu, sans la langue : Epic redirige chacun vers la sienne, et un lien
/// en `/en-US/` enverrait un ami français sur la page anglaise. IGDB connaît trois formes :
/// `store.epicgames.com/en-US/p/<slug>`, `www.epicgames.com/store/p/<slug>` et l'ancienne
/// `www.epicgames.com/store/en-US/product/<slug>/home`.
fn page_epic(url: &str) -> Option<String> {
    let (_, chemin) = url.split_once("epicgames.com")?;
    let segments: Vec<&str> = chemin.split(['/', '?', '#']).filter(|s| !s.is_empty()).collect();
    let i = segments.iter().position(|s| *s == "p" || *s == "product")?;
    let slug = segments.get(i + 1)?;
    Some(format!("https://store.epicgames.com/p/{slug}"))
}

/// `None` : panne. `Some(None)` : IGDB a répondu, pas de lien sûr.
fn chercher_steam(titre: &str) -> Option<Option<u64>> {
    let clean = clean_title(titre);
    if clean.is_empty() {
        return Some(None);
    }
    let body = format!(
        "fields name, alternative_names.name, parent_game, version_parent, external_games.uid, \
         external_games.external_game_source; \
         where name ~ \"{clean}\" | alternative_names.name ~ \"{clean}\"; limit 10;"
    );
    match query("games", &body) {
        Reponse::Corps(Value::Array(jeux)) => Some(appid_unique(&jeux, &norm(&clean))),
        Reponse::Corps(_) => Some(None),
        Reponse::Panne | Reponse::Limite => None,
    }
}

fn appid_unique(jeux: &[Value], cible: &str) -> Option<u64> {
    let ids: HashSet<u64> = jeux
        .iter()
        .filter(|g| g["parent_game"].is_null() && g["version_parent"].is_null() && nom_correspond(g, cible))
        .flat_map(|g| g["external_games"].as_array().into_iter().flatten())
        .filter(|e| e["external_game_source"].as_u64() == Some(STEAM_SOURCE as u64))
        .filter_map(|e| e["uid"].as_str()?.parse().ok())
        .collect();
    if ids.len() == 1 {
        ids.into_iter().next()
    } else {
        None
    }
}

/// Ce qu'a donné un appel au proxy.
///
/// 🔑 `Corps` veut dire « IGDB a répondu », pas « IGDB a trouvé » : une réponse vide en
/// est une. C'est cette distinction-là qui compte, parce que le cache disque mémorise les
/// recherches infructueuses pour ne pas les refaire — et qu'une panne mémorisée comme une
/// absence prive un jeu de sa description POUR TOUJOURS (le cache n'a pas d'expiration).
enum Reponse {
    Corps(Value),
    /// Réseau coupé, proxy en erreur, JSON illisible : on ne sait rien, on ne retient rien.
    Panne,
    /// Limite du proxy atteinte (429). Comme `Panne`, mais il faut en plus arrêter la
    /// passe : les appels suivants seraient refusés de la même façon.
    Limite,
}

/// POST une requête Apicalypse au proxy.
fn query(endpoint: &str, body: &str) -> Reponse {
    match ureq::post(&format!("{PROXY_URL}/{endpoint}"))
        .timeout(Duration::from_secs(15))
        .send_string(body)
    {
        Ok(resp) => match resp.into_json() {
            Ok(v) => Reponse::Corps(v),
            Err(_) => Reponse::Panne,
        },
        Err(ureq::Error::Status(429, _)) => Reponse::Limite,
        Err(_) => Reponse::Panne,
    }
}

/// URL d'une image IGDB à une taille donnée (ex. « cover_big_2x », « 1080p »).
fn img(image_id: &str, size: &str) -> String {
    format!("{IMG}/t_{size}/{image_id}.jpg")
}

/// Abrège les libellés de genre entre parenthèses (« Real Time Strategy (RTS) » → « RTS »).
fn clean_genre(name: &str) -> String {
    if let (Some(o), Some(c)) = (name.find('('), name.rfind(')')) {
        if c > o + 1 {
            return name[o + 1..c].trim().to_string();
        }
    }
    name.to_string()
}

/// Traduction des genres IGDB, qui n'existent **qu'en anglais** (l'API ne localise ni les
/// genres ni les résumés).
///
/// 🔑 Une table exhaustive et pas une heuristique : le vocabulaire des genres IGDB est
/// **fermé** — 23 entrées, relevées sur son endpoint `/genres` — donc il se traduit
/// intégralement et ne dérivera pas. Un genre inconnu (nouvelle entrée chez IGDB) ressort
/// tel quel plutôt que de disparaître : mieux vaut un libellé anglais qu'un jeu sans
/// catégorie.
///
/// ⚠️ Ces libellés servent aussi de **clé au filtre par catégorie**, qui regroupe par
/// chaîne exacte : mélanger anglais et français y ferait deux entrées pour un même genre.
fn genre_fr(g: &str) -> &str {
    match g {
        "Adventure" => "Aventure",
        "Card & Board Game" => "Cartes et plateau",
        "Fighting" => "Combat",
        "Hack and slash/Beat 'em up" => "Beat'em up",
        "Indie" => "Indépendant",
        "Music" => "Musique",
        "Pinball" => "Flipper",
        "Platform" => "Plateforme",
        "Point-and-click" => "Pointer-cliquer",
        "Puzzle" => "Réflexion",
        "Quiz/Trivia" => "Quiz",
        "Racing" => "Course",
        "RTS" => "Stratégie temps réel",
        "RPG" => "Jeu de rôle",
        "Shooter" => "Tir",
        "Simulator" => "Simulation",
        "Strategy" => "Stratégie",
        "Tactical" => "Tactique",
        "TBS" => "Stratégie au tour par tour",
        "Visual Novel" => "Roman visuel",
        // Arcade, MOBA, Sport : identiques en français.
        autre => autre,
    }
}

/// Francise les genres d'un lot avant de le rendre au front.
///
/// 🔑 En **sortie** et pas au moment de l'écriture en cache : le cache garde les libellés
/// IGDB d'origine, donc corriger ou compléter la table plus tard ne coûtera pas un
/// retéléchargement de toute la bibliothèque (plusieurs minutes pour les jeux non-Steam,
/// interrogés un par un et throttlés).
fn traduire_lot(lot: &mut [(String, IgdbMeta)]) {
    traduire_lot_pour(lot, crate::locale::langue());
}

/// Même travail, pour une langue donnée.
///
/// ⚠️ Séparé de `traduire_lot` pour les tests : la langue courante est un état global, et
/// un test qui en dépendrait verrait celle qu'un autre test, en parallèle, vient de poser.
fn traduire_lot_pour(lot: &mut [(String, IgdbMeta)], langue: crate::locale::Langue) {
    let traduire: fn(&str) -> &str = match langue {
        crate::locale::Langue::Fr => genre_fr,
        crate::locale::Langue::En => genre_en,
    };
    for (_, meta) in lot.iter_mut() {
        if let Some(genres) = meta.genre.as_deref() {
            meta.genre = Some(genres.split(", ").map(traduire).collect::<Vec<_>>().join(", "));
        }
    }
}

/// Les genres IGDB, relus pour une interface anglaise.
///
/// 🔑 IGDB EST DÉJÀ EN ANGLAIS, mais `clean_genre` abrège ses sigles pour qu'ils tiennent
/// dans une pastille : « RTS », « TBS », « RPG ». Un joueur français lisait « Stratégie
/// temps réel » grâce à `genre_fr` ; sans cette table, un joueur anglophone lirait le
/// sigle nu. On rend donc les formes longues, et on arrondit les deux libellés composés
/// qu'IGDB écrit comme des catégories de base de données.
///
/// ⚠️ Même contrainte que `genre_fr` : ces libellés sont la **clé du filtre par
/// catégorie**. Deux libellés pour un même genre y feraient deux entrées.
fn genre_en(g: &str) -> &str {
    match g {
        "Card & Board Game" => "Card & board",
        "Hack and slash/Beat 'em up" => "Beat 'em up",
        "Quiz/Trivia" => "Quiz",
        "RTS" => "Real-time strategy",
        "RPG" => "Role-playing",
        "Simulator" => "Simulation",
        "TBS" => "Turn-based strategy",
        autre => autre,
    }
}

/// Année à partir d'un timestamp Unix (approximation suffisante pour un affichage).
fn unix_to_year(ts: i64) -> i64 {
    1970 + ts / 31_556_952 // secondes dans une année moyenne
}

/// Normalise un nom pour comparaison : minuscules, alphanumérique seul.
/// Replie une lettre accentuée sur sa forme ASCII. `None` si elle n'en a pas.
///
/// Table écrite à la main plutôt qu'une dépendance de normalisation Unicode : on ne
/// couvre que le latin, c'est-à-dire l'alphabet dans lequel les jeux sont titrés, et
/// trente lignes valent mieux qu'une caisse de plus à compiler dans le binaire.
fn plier(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a",
        'æ' => "ae",
        'ç' => "c",
        'è' | 'é' | 'ê' | 'ë' => "e",
        'ì' | 'í' | 'î' | 'ï' => "i",
        'ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o",
        'œ' => "oe",
        'ù' | 'ú' | 'û' | 'ü' => "u",
        'ý' | 'ÿ' => "y",
        'ß' => "ss",
        _ => return None,
    })
}

/// Réduit un titre à sa forme comparable : minuscules, sans ponctuation, sans accents.
///
/// 🔑 LES ACCENTS COMPTENT, ET C'EST CONTRE-INTUITIF. Les fiches IGDB sont titrées en
/// anglais, les launchers rendent parfois le titre localisé : « Hadès » côté Epic contre
/// « Hades » côté IGDB, et la correspondance échouait sur un seul caractère. Le jeu était
/// alors mémorisé « introuvable » — définitivement, le cache n'expirant pas.
///
/// ⚠️ Toucher à cette fonction change la stratégie de correspondance : le cache doit être
/// versionné en même temps (cf. `cache_file`), sinon les jeux déjà marqués introuvables le
/// restent et la correction ne sert à personne.
fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // Minuscules d'abord : la table de pliage n'a ainsi que les formes minuscules à
    // couvrir, et « É » passe par « é » sans qu'on ait à l'écrire deux fois.
    for c in s.chars().flat_map(|c| c.to_lowercase()) {
        if !c.is_alphanumeric() {
            continue;
        }
        match plier(c) {
            Some(ascii) => out.push_str(ascii),
            None => out.push(c),
        }
    }
    out
}

/// Vrai si `cible` (déjà normalisée) correspond au nom principal du jeu **ou à l'un de
/// ses noms alternatifs**.
///
/// 🔑 POURQUOI LES ALTERNATIFS COMPTENT AUTANT QUE LE NOM PRINCIPAL. IGDB ne crée pas
/// toujours une entrée par titre commercial : quand une suite remplace son aîné sur les
/// mêmes serveurs, elle hérite de la fiche existante et l'ancien titre reste le nom
/// principal. « Overwatch 2 » n'existe ainsi nulle part comme nom de jeu — l'entrée
/// s'appelle « Overwatch » et porte « Overwatch 2 » en nom alternatif. Ne comparer que le
/// nom principal condamnait ces jeux : le match exact ne trouvait rien, et la recherche
/// ne ramenait que des DLC (« Overwatch 2: Invasion Bundle »…), tous écartés à juste
/// titre. Le jeu restait donc sans jaquette et sans description, définitivement.
///
/// ⚠️ Cela n'assouplit RIEN : on exige toujours une égalité stricte après normalisation,
/// simplement sur un jeu de noms plus complet. Les DLC continuent d'être rejetés.
fn nom_correspond(g: &Value, cible: &str) -> bool {
    if g["name"].as_str().map(norm).as_deref() == Some(cible) {
        return true;
    }
    g["alternative_names"]
        .as_array()
        .is_some_and(|noms| {
            noms.iter()
                .any(|n| n["name"].as_str().map(norm).as_deref() == Some(cible))
        })
}

/// Nettoie un titre pour l'insérer dans une chaîne Apicalypse (retire guillemets et ™®©).
fn clean_title(t: &str) -> String {
    t.chars()
        .filter(|c| !matches!(c, '"' | '™' | '®' | '©'))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Construit un `IgdbMeta` à partir d'un objet jeu IGDB.
fn parse_meta(g: &Value) -> IgdbMeta {
    let genre = g["genres"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|x| x["name"].as_str())
        .map(clean_genre);

    let description = g["summary"].as_str().map(String::from);

    let cover_url = g["cover"]["image_id"]
        .as_str()
        .map(|id| img(id, "cover_big_2x"));

    // Hero paysage : 1re artwork, à défaut 1re capture.
    let hero_url = g["artworks"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|x| x["image_id"].as_str())
        .or_else(|| {
            g["screenshots"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["image_id"].as_str())
        })
        .map(|id| img(id, "1080p"));

    let developer = g["involved_companies"]
        .as_array()
        .and_then(|a| a.iter().find(|c| c["developer"].as_bool().unwrap_or(false)))
        .and_then(|c| c["company"]["name"].as_str())
        .map(String::from);

    let year = g["first_release_date"].as_i64().map(unix_to_year);

    let screenshots = g["screenshots"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s["image_id"].as_str())
                .take(6)
                .map(|id| img(id, "1080p"))
                .collect()
        })
        .unwrap_or_default();

    // Les genres arrivent développés (`genres.name` → `{id, name}`), le reste en
    // identifiants nus : on lit les deux formes.
    let ids = |cle: &str| -> Vec<u64> {
        g[cle]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_u64().or_else(|| x["id"].as_u64())).collect())
            .unwrap_or_default()
    };
    let classement = Classement {
        genres: ids("genres"),
        themes: ids("themes"),
        modes: ids("game_modes"),
        vues: ids("player_perspectives"),
    };

    IgdbMeta {
        genre,
        description,
        cover_url,
        hero_url,
        developer,
        year,
        screenshots,
        igdb_id: g["id"].as_u64(),
        steam_appid: appid_de_la_fiche(g),
        classement: Some(classement),
    }
}

/// Appid Steam d'une fiche IGDB : le premier de ses liens vers Steam.
///
/// Contrairement à `appid_unique`, plusieurs appids ne sont pas une ambiguïté : la fiche
/// est déjà choisie, et ses entrées Steam multiples sont des éditions du même jeu (plus
/// rarement une démo), qui portent les mêmes tags à peu de chose près.
fn appid_de_la_fiche(g: &Value) -> Option<u64> {
    g["external_games"]
        .as_array()?
        .iter()
        .filter(|e| e["external_game_source"].as_u64() == Some(STEAM_SOURCE as u64))
        .find_map(|e| e["uid"].as_str()?.parse().ok())
}

/// Ce que rapporte la passe Steam.
///
/// `interroges` n'est pas un détail : ce sont les seuls appids dont on a le droit de
/// mémoriser l'absence. Ceux d'un lot dont l'appel a échoué doivent rester inconnus, pour
/// être retentés au prochain scan.
#[derive(Default)]
struct PasseSteam {
    metas: HashMap<String, IgdbMeta>,
    interroges: HashSet<String>,
    limite: bool,
}

/// Métadonnées des jeux Steam en masse (appid → meta), via `external_games` puis `games`.
fn steam_metas(appids: &[String]) -> PasseSteam {
    let mut out = PasseSteam::default();
    for chunk in appids.chunks(STEAM_CHUNK) {
        let uids = chunk
            .iter()
            .map(|a| format!("\"{a}\""))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            "fields game,uid; where external_game_source = {STEAM_SOURCE} & uid = ({uids}); limit 500;"
        );
        let ext = match query("external_games", &body) {
            Reponse::Corps(v) => v,
            Reponse::Limite => {
                out.limite = true;
                return out;
            }
            Reponse::Panne => continue,
        };
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));

        // id du jeu IGDB → appid Steam.
        let mut gid_to_appid: HashMap<i64, String> = HashMap::new();
        if let Some(arr) = ext.as_array() {
            for e in arr {
                if let (Some(g), Some(uid)) = (e["game"].as_i64(), e["uid"].as_str()) {
                    gid_to_appid.entry(g).or_insert_with(|| uid.to_string());
                }
            }
        }
        // IGDB a répondu : un appid qu'il ne rattache à aucun jeu est vraiment absent de
        // sa base, et cette absence-là se mémorise sans risque.
        let mappes: HashSet<&String> = gid_to_appid.values().collect();
        for appid in chunk {
            if !mappes.contains(appid) {
                out.interroges.insert(appid.clone());
            }
        }
        if gid_to_appid.is_empty() {
            continue;
        }

        let ids = gid_to_appid
            .keys()
            .map(|g| g.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let body2 = format!("{FIELDS} where id = ({ids}); limit 500;");
        let games = match query("games", &body2) {
            Reponse::Corps(v) => v,
            Reponse::Limite => {
                out.limite = true;
                return out;
            }
            Reponse::Panne => continue,
        };
        std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));

        // Le second appel a répondu : le reste du lot est tranché lui aussi.
        for appid in chunk {
            out.interroges.insert(appid.clone());
        }

        if let Some(arr) = games.as_array() {
            for g in arr {
                if let Some(gid) = g["id"].as_i64() {
                    if let Some(appid) = gid_to_appid.get(&gid) {
                        out.metas.insert(appid.clone(), parse_meta(g));
                    }
                }
            }
        }
    }
    out
}

/// Reconnaît un titre **deviné** (nom de dossier d'un jeu détecté hors launcher) et
/// renvoie le vrai nom IGDB avec ses métadonnées.
///
/// Différence avec [`name_meta`], qui exige un nom déjà exact : ici la recherche est
/// tolérante — « Return To Moria » doit pouvoir tomber sur « The Lord of the Rings:
/// Return to Moria ». Le garde-fou est l'**inclusion** d'un nom normalisé dans l'autre,
/// avec un minimum de 5 caractères : sans lui, `search` renverrait toujours quelque
/// chose et on baptiserait les jeux au hasard.
pub fn recognize(guess: &str) -> Option<(String, IgdbMeta)> {
    let clean = clean_title(guess);
    let target = norm(&clean);
    if target.len() < 5 {
        return None;
    }

    let body = format!("search \"{clean}\"; {FIELDS} limit 10;");
    let Reponse::Corps(Value::Array(arr)) = query("games", &body) else {
        return None;
    };
    std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));

    // Nom exact d'abord, puis la première inclusion (les résultats de `search` sont
    // déjà classés par pertinence).
    let mut fallback: Option<&Value> = None;
    for g in &arr {
        let Some(name) = g["name"].as_str() else { continue };
        // Le nom alternatif compte ici aussi : un jeu détecté sous son titre commercial
        // (« Overwatch 2 ») doit tomber sur l'entrée qui le porte en alias.
        if nom_correspond(g, &target) {
            return Some((name.to_string(), parse_meta(g)));
        }
        // ⚠️ LE REPLI N'ACCEPTE QUE DES JEUX DE PLEIN DROIT. Il se contente d'une
        // inclusion d'un nom dans l'autre — assez lâche pour qu'un DLC l'emporte : le
        // normalisé de « Overwatch 2: Invasion Bundle » CONTIENT celui d'« Overwatch 2 »,
        // et le jeu détecté aurait pris le nom, la jaquette et la description du DLC.
        // `parent_game` est le marqueur : vérifié chez IGDB, les jeux de base n'en ont
        // pas, les DLC, extensions et bundles en portent toujours un.
        if !g["parent_game"].is_null() {
            continue;
        }
        let key = norm(name);
        if fallback.is_none() && (key.contains(&target) || target.contains(&key)) {
            fallback = Some(g);
        }
    }
    let g = fallback?;
    Some((g["name"].as_str()?.to_string(), parse_meta(g)))
}

/// Résultat d'une recherche par titre. Trois issues et pas deux : « IGDB ne connaît pas
/// ce jeu » se mémorise, « je n'ai pas pu demander » surtout pas.
enum Issue {
    Trouve(IgdbMeta),
    /// IGDB a répondu et n'a rien : inutile de redemander au prochain lancement.
    Absent,
    /// L'appel a échoué. Le jeu reste inconnu du cache, on retentera.
    Panne,
    /// Limite du proxy atteinte : comme `Panne`, et il faut arrêter la passe.
    Limite,
}

/// Titres hors Steam cherchés ensemble par `noms_exacts`.
const NOMS_PAR_APPEL: usize = 12;

/// Ce que donne une recherche exacte groupée.
enum Groupe {
    /// IGDB a répondu : id du jeu → fiche, pour les titres qui ont une correspondance exacte.
    Trouves(HashMap<String, IgdbMeta>),
    Panne,
    Limite,
}

/// La correspondance exacte de `name_meta` (son premier appel), pour plusieurs titres en
/// UN appel : `name ~ "A" | alternative_names.name ~ "A" | name ~ "B" | …`.
///
/// 🔑 Même fiche que titre par titre : pour chaque titre, la première de la réponse qui
/// lui correspond, et l'ordre des réponses ne dépend pas du nombre de titres demandés.
/// Vérifié le 24/10/2026 sur douze titres pièges (Overwatch 2 par son alias, Doom et
/// Tetris aux dizaines d'homonymes, VALORANT en capitales) : aucun écart. 4 s pour douze
/// titres contre 18 un par un, et un appel au quota d'IGDB — partagé par tous les
/// joueurs — au lieu de douze.
fn noms_exacts(jeux: &[&GameDto]) -> Groupe {
    let cibles: Vec<(&GameDto, String, String)> = jeux
        .iter()
        .filter_map(|g| {
            let clean = clean_title(&g.title);
            (!clean.is_empty()).then(|| (*g, norm(&clean), clean))
        })
        .collect();
    if cibles.is_empty() {
        return Groupe::Trouves(HashMap::new());
    }
    let conditions = cibles
        .iter()
        .map(|(_, _, clean)| format!("name ~ \"{clean}\" | alternative_names.name ~ \"{clean}\""))
        .collect::<Vec<_>>()
        .join(" | ");
    match query("games", &format!("{FIELDS} where {conditions}; limit 500;")) {
        Reponse::Corps(Value::Array(fiches)) => {
            std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
            let trouves = cibles
                .iter()
                .filter_map(|(g, cible, _)| {
                    let fiche = fiches.iter().find(|f| nom_correspond(f, cible))?;
                    Some((g.id.clone(), parse_meta(fiche)))
                })
                .collect();
            Groupe::Trouves(trouves)
        }
        Reponse::Corps(_) => Groupe::Trouves(HashMap::new()),
        Reponse::Panne => Groupe::Panne,
        Reponse::Limite => Groupe::Limite,
    }
}

/// Métadonnées d'un jeu non-Steam par son titre : match exact puis repli `search`.
///
/// `exact_fait` : la correspondance exacte a déjà été demandée (`noms_exacts`) et n'a rien
/// donné — inutile de la redemander, on passe directement au repli.
fn name_meta(title: &str, exact_fait: bool) -> Issue {
    let clean = clean_title(title);
    if clean.is_empty() {
        return Issue::Absent;
    }
    let target = norm(&clean);

    // 1) Correspondance exacte du nom **ou d'un nom alternatif**, insensible à la casse.
    //
    // 🔑 POURQUOI L'ALIAS EST INTERROGÉ ICI, ET PAS SEULEMENT COMPARÉ PLUS BAS. Comparer
    // les alias des résultats ne sert à rien s'ils n'y sont pas : `search "Overwatch 2"`
    // renvoie quinze DLC et **jamais** l'entrée 125174, qui porte pourtant cet alias.
    // Vérifié contre l'API. Il faut donc le DEMANDER à IGDB, pas l'espérer.
    //
    // ⚠️ `~` est un égal insensible à la casse, PAS un « contient » : « overwatch » ne
    // ramène que les entrées nommées ainsi, aucun bundle. Vérifié aussi — c'est ce qui
    // permet d'élargir la recherche sans rouvrir la porte aux DLC.
    if !exact_fait {
        let body = format!(
            "{FIELDS} where name ~ \"{clean}\" | alternative_names.name ~ \"{clean}\"; limit 5;"
        );
        match query("games", &body) {
            Reponse::Corps(Value::Array(arr)) => {
                std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
                for g in &arr {
                    if nom_correspond(g, &target) {
                        return Issue::Trouve(parse_meta(g));
                    }
                }
            }
            Reponse::Corps(_) => {}
            Reponse::Limite => return Issue::Limite,
            // 🔑 On ne tente pas le repli `search` : sans réponse au premier appel, un
            // « rien trouvé » au second ne prouverait rien de plus, et le jeu serait quand
            // même mémorisé comme absent alors que la panne est la seule chose établie.
            Reponse::Panne => return Issue::Panne,
        }
    }

    // 2) Repli `search` (tolérant à la casse), on ne garde qu'un nom normalisé identique.
    let body = format!("search \"{clean}\"; {FIELDS} limit 15;");
    match query("games", &body) {
        Reponse::Corps(Value::Array(arr)) => {
            std::thread::sleep(Duration::from_millis(CALL_DELAY_MS));
            for g in &arr {
                if nom_correspond(g, &target) {
                    return Issue::Trouve(parse_meta(g));
                }
            }
        }
        Reponse::Corps(_) => {}
        Reponse::Limite => return Issue::Limite,
        Reponse::Panne => return Issue::Panne,
    }

    Issue::Absent
}

/// Remplit la métadonnée descriptive de tous les jeux (Steam en masse + autres par nom),
/// en cache disque. `emit` reçoit des lots `(id, meta)` au fil de l'eau (fusion en direct
/// côté front). Renvoie l'ensemble des `(id, meta)` résolus.
pub fn fill_metadata(
    games: &[GameDto],
    config_dir: &Path,
    emit: impl Fn(&[(String, IgdbMeta)]),
) -> Vec<(String, IgdbMeta)> {
    let mut cache = load_cache(config_dir);
    let mut absences = load_absences(config_dir);
    let t = maintenant();
    let mut out: Vec<(String, IgdbMeta)> = Vec::new();
    let mut dirty = false;

    // Répartition : déjà en cache / Steam (appid) / autres.
    let mut cached_batch: Vec<(String, IgdbMeta)> = Vec::new();
    let mut steam_todo: Vec<(String, String)> = Vec::new(); // (id jeu, appid)
    let mut other_todo: Vec<&GameDto> = Vec::new();
    // Fiches d'avant les catégories : affichées tout de suite, redemandées quand même.
    let mut a_completer: HashSet<String> = HashSet::new();

    for g in games {
        match cache.get(&g.id) {
            Some(Some(meta)) => {
                cached_batch.push((g.id.clone(), meta.clone()));
                if meta.classement.is_some() {
                    continue;
                }
                a_completer.insert(g.id.clone());
            }
            // Introuvable récemment : inutile de redemander. Une absence sans date
            // (d'avant `Absences`) est retentée une fois, puis datée.
            Some(None)
                if absences
                    .get(&g.id)
                    .is_some_and(|&quand| t.saturating_sub(quand) < DUREE_ABSENT) =>
            {
                continue;
            }
            _ => {}
        }
        if g.platform == "steam" {
            if let Some(appid) = g.id.strip_prefix("steam:") {
                steam_todo.push((g.id.clone(), appid.to_string()));
                continue;
            }
        }
        other_todo.push(g);
    }

    // Lot immédiat des métas déjà connues (2e lancement = tout ici).
    if !cached_batch.is_empty() {
        traduire_lot(&mut cached_batch);
        emit(&cached_batch);
        out.extend(cached_batch);
    }

    // Vrai dès qu'un appel a échoué : la passe est incomplète, il ne faut pas croire que
    // les jeux non résolus sont absents d'IGDB.
    let mut interrompue = false;

    // Steam en masse.
    if !steam_todo.is_empty() {
        let appids: Vec<String> = steam_todo.iter().map(|(_, a)| a.clone()).collect();
        let passe = steam_metas(&appids);
        interrompue |= passe.limite;
        let mut batch = Vec::new();
        for (gid, appid) in &steam_todo {
            match passe.metas.get(appid) {
                Some(meta) => {
                    cache.insert(gid.clone(), Some(meta.clone()));
                    dirty = true;
                    batch.push((gid.clone(), meta.clone()));
                }
                // Interrogé, rien trouvé : IGDB ne connaît pas cet appid, on le note.
                None if passe.interroges.contains(appid) => {
                    noter_absent(&mut cache, &mut absences, &a_completer, gid, t);
                    dirty = true;
                }
                // Lot en panne : on ne sait pas, donc on ne dit rien. Retenté au prochain scan.
                None => interrompue = true,
            }
        }
        if !batch.is_empty() {
            traduire_lot(&mut batch);
            emit(&batch);
            out.extend(batch);
        }
    }

    // Non-Steam, d'abord par groupes de titres exacts (un appel pour douze).
    // `(jeu, exact déjà demandé)` : ce qui reste pour le passage un par un.
    let mut un_par_un: Vec<(&GameDto, bool)> = Vec::new();
    let mut limite = false;
    for lot in other_todo.chunks(NOMS_PAR_APPEL) {
        if limite {
            break;
        }
        match noms_exacts(lot) {
            Groupe::Trouves(mut trouves) => {
                let mut batch = Vec::new();
                for g in lot {
                    match trouves.remove(&g.id) {
                        Some(meta) => {
                            cache.insert(g.id.clone(), Some(meta.clone()));
                            dirty = true;
                            batch.push((g.id.clone(), meta));
                        }
                        None => un_par_un.push((g, true)),
                    }
                }
                if !batch.is_empty() {
                    traduire_lot(&mut batch);
                    emit(&batch);
                    out.extend(batch);
                    // 🔑 Enregistré au fil de l'eau : une grosse bibliothèque hors Steam
                    // prend plusieurs minutes, et fermer Torii en cours de route perdait
                    // tout ce qui avait été trouvé jusque-là.
                    save_cache(config_dir, &cache);
                    save_absences(config_dir, &absences);
                }
            }
            // ⚠️ Le lot repasse ENTIER par le chemin titre par titre : un titre qui fait
            // échouer la requête groupée (caractère que l'Apicalypse refuse) ne doit pas
            // priver ses onze voisins de fiche à chaque lancement.
            Groupe::Panne => un_par_un.extend(lot.iter().map(|g| (*g, false))),
            Groupe::Limite => {
                interrompue = true;
                limite = true;
            }
        }
    }

    // Puis un par un (throttlé) : repli `search` pour ceux sans correspondance exacte.
    let mut batch = Vec::new();
    for (g, exact_fait) in un_par_un {
        if limite {
            break;
        }
        match name_meta(&g.title, exact_fait) {
            Issue::Trouve(meta) => {
                cache.insert(g.id.clone(), Some(meta.clone()));
                dirty = true;
                batch.push((g.id.clone(), meta));
            }
            Issue::Absent => {
                noter_absent(&mut cache, &mut absences, &a_completer, &g.id, t);
                dirty = true;
            }
            Issue::Panne => interrompue = true,
            // Inutile d'insister : les appels suivants seraient refusés pareillement, et
            // chacun compterait quand même dans la limite. Le reste attendra le prochain scan.
            Issue::Limite => {
                interrompue = true;
                break;
            }
        }
        if batch.len() >= NONSTEAM_BATCH {
            traduire_lot(&mut batch);
            emit(&batch);
            out.extend(batch.drain(..));
            if dirty {
                save_cache(config_dir, &cache);
                save_absences(config_dir, &absences);
            }
        }
    }
    if !batch.is_empty() {
        traduire_lot(&mut batch);
        emit(&batch);
        out.extend(batch);
    }

    // Trace explicite : sans elle, une passe écourtée ressemble à s'y méprendre à une
    // bibliothèque dont IGDB ne connaîtrait pas la moitié des jeux.
    if interrompue {
        crate::journal::write(
            config_dir,
            "INFO",
            "métadonnées IGDB : passe incomplète (proxy injoignable ou limite atteinte). \
             Les jeux non résolus ne sont PAS mémorisés comme absents, ils seront \
             retentés au prochain scan.",
        );
    }

    if dirty {
        // Les jeux trouvés depuis n'ont plus d'absence à dater.
        absences.retain(|id, _| matches!(cache.get(id), Some(None)));
        save_cache(config_dir, &cache);
        save_absences(config_dir, &absences);
    }
    // Une fiche complétée est sortie deux fois : son ancienne forme, émise d'entrée, puis
    // la nouvelle. On ne rend que la dernière.
    if !a_completer.is_empty() {
        let mut vus = HashSet::new();
        out.reverse();
        out.retain(|(id, _)| vus.insert(id.clone()));
        out.reverse();
    }
    out
}

/// Note qu'IGDB ne trouve pas un jeu.
///
/// ⚠️ Pour une fiche ancienne qu'on cherchait seulement à compléter, ce n'est PAS une
/// absence : elle a été trouvée autrefois, et la jeter ôterait jaquette et description à
/// un jeu qui les avait. On la garde, marquée complète (classement vide) pour ne pas la
/// redemander à chaque lancement.
fn noter_absent(
    cache: &mut MetaCache,
    absences: &mut Absences,
    a_completer: &HashSet<String>,
    id: &str,
    t: u64,
) {
    if a_completer.contains(id) {
        if let Some(Some(meta)) = cache.get_mut(id) {
            meta.classement = Some(Classement::default());
            return;
        }
    }
    cache.insert(id.to_string(), None);
    absences.insert(id.to_string(), t);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La table des genres doit couvrir TOUT le vocabulaire IGDB (relevé sur `/genres`),
    /// une fois passé par `clean_genre` qui abrège les sigles entre parenthèses. Un genre
    /// oublié ici s'afficherait en anglais au milieu des autres — et compterait comme une
    /// catégorie distincte dans le filtre.
    #[test]
    fn tous_les_genres_igdb_sont_traduits() {
        // Les trois derniers sont identiques en français, c'est voulu.
        let identiques = ["Arcade", "MOBA", "Sport"];
        for brut in [
            "Adventure", "Arcade", "Card & Board Game", "Fighting",
            "Hack and slash/Beat 'em up", "Indie", "MOBA", "Music", "Pinball", "Platform",
            "Point-and-click", "Puzzle", "Quiz/Trivia", "Racing", "Real Time Strategy (RTS)",
            "Role-playing (RPG)", "Shooter", "Simulator", "Sport", "Strategy", "Tactical",
            "Turn-based strategy (TBS)", "Visual Novel",
        ] {
            let court = clean_genre(brut);
            let fr = genre_fr(&court);
            if !identiques.contains(&brut) {
                assert_ne!(fr, court, "genre non traduit : {brut}");
            }
        }
    }

    /// Un genre qu'IGDB ajouterait demain doit ressortir tel quel, pas disparaître.
    #[test]
    fn un_genre_inconnu_traverse_sans_dommage() {
        let mut lot = vec![(
            "steam:1".to_string(),
            IgdbMeta { genre: Some("Roguelike".into()), ..Default::default() },
        )];
        traduire_lot_pour(&mut lot, crate::locale::Langue::Fr);
        assert_eq!(lot[0].1.genre.as_deref(), Some("Roguelike"));
    }

    #[test]
    fn traduit_les_genres_d_un_lot() {
        let mut lot = vec![(
            "steam:2".to_string(),
            IgdbMeta { genre: Some("Role-playing (RPG)".into()), ..Default::default() },
        )];
        // `parse_meta` abrège avant de stocker : on simule la valeur telle qu'elle est en cache.
        lot[0].1.genre = Some(clean_genre("Role-playing (RPG)"));
        traduire_lot_pour(&mut lot, crate::locale::Langue::Fr);
        assert_eq!(lot[0].1.genre.as_deref(), Some("Jeu de rôle"));
    }

    /// En anglais, les sigles abrégés par `clean_genre` redeviennent lisibles.
    #[test]
    fn en_anglais_les_sigles_se_deplient() {
        let mut lot = vec![(
            "steam:3".to_string(),
            IgdbMeta { genre: Some(format!("{}, Indie", clean_genre("Real Time Strategy (RTS)"))), ..Default::default() },
        )];
        traduire_lot_pour(&mut lot, crate::locale::Langue::En);
        assert_eq!(lot[0].1.genre.as_deref(), Some("Real-time strategy, Indie"));
    }

    /// Un jeu dont le titre commercial n'est qu'un ALIAS chez IGDB doit être reconnu.
    /// Cas réel : « Overwatch 2 » n'existe pas comme nom de jeu — l'entrée s'appelle
    /// « Overwatch » et le porte en `alternative_names`. Sans ça, la recherche ne ramène
    /// que des DLC (« Overwatch 2: Invasion Bundle »), tous écartés, et le jeu reste sans
    /// jaquette pour toujours.
    #[test]
    fn un_nom_alternatif_vaut_le_nom_principal() {
        let jeu = serde_json::json!({
            "name": "Overwatch",
            "alternative_names": [{ "name": "Overwatch II" }, { "name": "Overwatch 2" }],
        });
        assert!(nom_correspond(&jeu, &norm("Overwatch 2")), "l'alias doit suffire");
        assert!(nom_correspond(&jeu, &norm("Overwatch")), "le nom principal aussi");

        // ⚠️ Et rien d'autre : la règle reste une égalité stricte, les DLC sont rejetés.
        let dlc = serde_json::json!({ "name": "Overwatch 2: Invasion Bundle" });
        assert!(!nom_correspond(&dlc, &norm("Overwatch 2")), "un DLC ne passe pas");

        // Un jeu sans alias ne doit pas faire échouer la lecture du champ absent.
        let nu = serde_json::json!({ "name": "Hadès" });
        assert!(nom_correspond(&nu, &norm("HADÈS")));
        assert!(!nom_correspond(&nu, &norm("Hadès II")));
    }

    /// Réponses réelles d'IGDB (23/10/2026), réduites aux champs lus.
    #[test]
    fn le_lien_steam_exige_un_identifiant_unique() {
        let steam = |uid: &str| serde_json::json!({ "uid": uid, "external_game_source": 1 });
        let gog = serde_json::json!({ "uid": "1207664663", "external_game_source": 5 });
        // Deux « Hades » chez IGDB : un seul est sur Steam, c'est lui.
        let hades = [
            serde_json::json!({ "name": "Hades", "external_games": [gog.clone()] }),
            serde_json::json!({ "name": "Hades", "external_games": [steam("1145360"), gog.clone()] }),
        ];
        assert_eq!(appid_unique(&hades, &norm("Hades")), Some(1145360));
        // Deux jeux du même nom, tous deux sur Steam : aucun lien plutôt qu'un mauvais.
        let doublon = [
            serde_json::json!({ "name": "Control", "external_games": [steam("870780")] }),
            serde_json::json!({ "name": "Control", "external_games": [steam("123")] }),
        ];
        assert_eq!(appid_unique(&doublon, &norm("Control")), None);
        // Le DLC et l'édition ne prêtent pas leur identifiant au jeu.
        let fortnite = [
            serde_json::json!({ "name": "Fortnite", "external_games": [] }),
            serde_json::json!({ "name": "Fortnite", "parent_game": 1905, "external_games": [steam("1")] }),
            serde_json::json!({ "name": "Fortnite", "version_parent": 1905, "external_games": [steam("2")] }),
        ];
        assert_eq!(appid_unique(&fortnite, &norm("Fortnite")), None);
        // Un nom voisin ne compte pas.
        let voisin = [serde_json::json!({ "name": "CONTROL Resonant", "external_games": [steam("9")] })];
        assert_eq!(appid_unique(&voisin, &norm("Control")), None);
    }

    /// Un titre localisé et son équivalent anglais ne doivent plus se manquer pour un
    /// accent : les launchers rendent parfois « Hadès » là où IGDB dit « Hades ».
    #[test]
    fn les_accents_ne_font_pas_echouer_la_correspondance() {
        assert_eq!(norm("Hadès"), norm("Hades"));
        assert_eq!(norm("Pokémon Écarlate"), "pokemonecarlate");
        assert_eq!(norm("CŒUR & âme"), "coeurame");
        assert_eq!(norm("Straße"), "strasse");
        // La casse et la ponctuation restent neutralisées comme avant.
        assert_eq!(norm("PUBG: Battlegrounds"), "pubgbattlegrounds");
        // ⚠️ Et deux jeux différents ne deviennent pas égaux au passage.
        assert_ne!(norm("Hadès"), norm("Hadès II"));
    }

    /// La migration `v1` → `v2` doit garder ce qui a été trouvé et oublier les
    /// « introuvables » — c'est là que sont les fiches perdues par une panne réseau.
    #[test]
    fn la_migration_garde_les_fiches_et_oublie_les_introuvables() {
        let dir = std::env::temp_dir().join(format!("torii-igdb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut v1 = MetaCache::new();
        v1.insert(
            "steam:1".into(),
            Some(IgdbMeta { genre: Some("RPG".into()), ..Default::default() }),
        );
        v1.insert("steam:2".into(), None); // peut-être une panne, on ne peut pas savoir
        std::fs::write(dir.join(ANCIENS_CACHES[2]), serde_json::to_string(&v1).unwrap()).unwrap();

        let migre = load_cache(&dir);
        assert_eq!(migre.len(), 1, "seule la fiche trouvée survit");
        assert!(migre.contains_key("steam:1"));
        assert!(!migre.contains_key("steam:2"), "l'introuvable doit être re-cherché");

        assert!(cache_file(&dir).exists(), "la v2 est écrite dès la première lecture");
        assert!(dir.join(ANCIENS_CACHES[2]).exists(), "l'ancien reste, filet en cas d'écriture coupée");

        // Deuxième lecture : on repart de la v2, et l'introuvable ne ressuscite pas.
        assert_eq!(load_cache(&dir).len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Genres développés, thèmes/modes/vues en identifiants nus, liens externes : tout ce
    /// qui fait les catégories doit être lu.
    #[test]
    fn la_fiche_donne_son_classement_et_son_appid_steam() {
        let g = serde_json::json!({
            "id": 1942,
            "name": "The Witcher 3: Wild Hunt",
            "genres": [ { "id": 12, "name": "Role-playing (RPG)" }, { "id": 31, "name": "Adventure" } ],
            "themes": [1, 17, 38],
            "game_modes": [1],
            "player_perspectives": [2],
            "external_games": [
                { "id": 1, "uid": "1207664643", "external_game_source": 5 },
                { "id": 2, "uid": "292030", "external_game_source": 1 },
                { "id": 3, "uid": "499450", "external_game_source": 1 }
            ]
        });
        let meta = parse_meta(&g);
        assert_eq!(meta.igdb_id, Some(1942));
        assert_eq!(meta.genre.as_deref(), Some("RPG"));
        // Le premier lien Steam, pas celui de GOG (source 5) ni l'édition suivante.
        assert_eq!(meta.steam_appid, Some(292030));
        assert_eq!(
            meta.classement,
            Some(Classement { genres: vec![12, 31], themes: vec![1, 17, 38], modes: vec![1], vues: vec![2] })
        );
    }

    /// Une fiche écrite avant les catégories se relit telle quelle, marquée à compléter.
    #[test]
    fn une_fiche_d_avant_les_categories_est_a_completer() {
        let ancien = r#"{"steam:1":{"genre":"RPG","description":"d","screenshots":[]},"steam:2":null}"#;
        let cache: MetaCache = serde_json::from_str(ancien).unwrap();
        let meta = cache["steam:1"].as_ref().unwrap();
        assert_eq!(meta.genre.as_deref(), Some("RPG"));
        assert!(meta.classement.is_none());
        assert!(cache["steam:2"].is_none());
    }

    /// Les trois formes d'adresse Epic qu'IGDB connaît donnent la même page, sans langue.
    #[test]
    fn la_page_epic_perd_sa_langue() {
        for url in [
            "https://store.epicgames.com/en-US/p/alan-wake-2",
            "https://www.epicgames.com/store/p/alan-wake-2",
            "https://www.epicgames.com/store/en-US/product/alan-wake-2/home",
        ] {
            assert_eq!(page_epic(url).as_deref(), Some("https://store.epicgames.com/p/alan-wake-2"), "{url}");
        }
        assert_eq!(page_epic("https://www.epicgames.com/fortnite"), None);
    }

    /// Relevé IGDB de VALORANT (24/10/2026) : pas de Steam, une page Epic, un site officiel.
    #[test]
    fn les_pages_d_un_jeu_hors_steam() {
        let g = serde_json::json!({
            "websites": [
                { "type": 1, "url": "https://playvalorant.com/" },
                { "type": 16, "url": "https://www.epicgames.com/store/en-US/p/valorant" }
            ],
            "external_games": [
                { "external_game_source": 3, "url": "https://www.giantbomb.com/games/3030-77445/" },
                { "external_game_source": 26, "url": "https://store.epicgames.com/en-US/p/valorant" }
            ]
        });
        let p = lire_pages(&g);
        assert_eq!(p.steam, None);
        assert_eq!(p.epic.as_deref(), Some("https://store.epicgames.com/p/valorant"));
        assert_eq!(p.site.as_deref(), Some("https://playvalorant.com/"));
    }

    /// Le lien Steam vient de l'appid (une adresse propre), pas d'une URL recopiée.
    #[test]
    fn la_page_steam_vient_de_l_appid() {
        let g = serde_json::json!({
            "external_games": [ { "external_game_source": 1, "uid": "2357570", "url": "https://store.steampowered.com/app/2357570" } ]
        });
        assert_eq!(lire_pages(&g).steam.as_deref(), Some("https://store.steampowered.com/app/2357570/"));
    }

    /// Une fiche qu'on cherchait seulement à compléter n'est jamais jetée : IGDB ne la
    /// retrouve plus, mais elle avait sa jaquette et sa description.
    #[test]
    fn completer_une_fiche_ne_la_jette_jamais() {
        let mut cache = MetaCache::new();
        cache.insert("epic:a".into(), Some(IgdbMeta { genre: Some("RPG".into()), ..Default::default() }));
        let mut absences = Absences::new();
        let a_completer: HashSet<String> = ["epic:a".to_string()].into();

        noter_absent(&mut cache, &mut absences, &a_completer, "epic:a", 10);
        let meta = cache["epic:a"].as_ref().expect("la fiche est gardée");
        assert_eq!(meta.genre.as_deref(), Some("RPG"));
        assert_eq!(meta.classement, Some(Classement::default()), "complète : plus redemandée");
        assert!(absences.is_empty());

        // Un jeu nouveau, lui, est bien noté absent.
        noter_absent(&mut cache, &mut absences, &a_completer, "epic:b", 10);
        assert!(cache["epic:b"].is_none());
        assert_eq!(absences.get("epic:b"), Some(&10));
    }
}
