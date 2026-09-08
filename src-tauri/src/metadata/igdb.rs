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
//!   - **Autres launchers** : match exact du nom (`where name = "…"`), repli `search`
//!     avec sélection du nom normalisé (évite les DLC/jeux voisins remontés par `search`).
//!     La comparaison porte sur le nom principal **et les noms alternatifs** : IGDB
//!     n'ouvre pas toujours une fiche par titre commercial (cf. `nom_correspond`).

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
}

/// id du jeu → métadonnées (None = cherché mais introuvable dans IGDB, pour ne pas re-chercher).
type MetaCache = HashMap<String, Option<IgdbMeta>>;

fn cache_file(dir: &Path) -> PathBuf {
    // Versionné : incrémenter si le schéma `IgdbMeta` ou la stratégie de correspondance change.
    dir.join("igdb_meta_cache_v3.json")
}

/// Les caches des versions précédentes, du plus récent au plus ancien.
///
/// `v1` : d'avant la distinction panne / absence (cf. `Reponse`) — ses « introuvables »
/// contiennent des pannes réseau prises pour des absences.
/// `v2` : d'avant la reconnaissance des noms alternatifs (cf. `nom_correspond`) et le
/// pliage des accents (cf. `norm`) — ses « introuvables » contiennent des jeux qu'IGDB
/// connaît sous un alias, et d'autres qui ne différaient que par une lettre accentuée.
const ANCIENS_CACHES: [&str; 2] = ["igdb_meta_cache_v2.json", "igdb_meta_cache_v1.json"];

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
    for (_, meta) in lot.iter_mut() {
        if let Some(genres) = meta.genre.as_deref() {
            meta.genre = Some(genres.split(", ").map(genre_fr).collect::<Vec<_>>().join(", "));
        }
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

    IgdbMeta {
        genre,
        description,
        cover_url,
        hero_url,
        developer,
        year,
        screenshots,
    }
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

/// Métadonnées d'un jeu non-Steam par son titre : match exact puis repli `search`.
fn name_meta(title: &str) -> Issue {
    let clean = clean_title(title);
    if clean.is_empty() {
        return Issue::Absent;
    }
    let target = norm(&clean);

    // 1) Correspondance exacte du nom (précis quand la casse coïncide).
    let body = format!("{FIELDS} where name = \"{clean}\"; limit 3;");
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
    let mut out: Vec<(String, IgdbMeta)> = Vec::new();
    let mut dirty = false;

    // Répartition : déjà en cache / Steam (appid) / autres.
    let mut cached_batch: Vec<(String, IgdbMeta)> = Vec::new();
    let mut steam_todo: Vec<(String, String)> = Vec::new(); // (id jeu, appid)
    let mut other_todo: Vec<&GameDto> = Vec::new();

    for g in games {
        if let Some(cached) = cache.get(&g.id) {
            if let Some(meta) = cached {
                cached_batch.push((g.id.clone(), meta.clone()));
            }
            continue; // déjà résolu (Some ou None)
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
                    cache.insert(gid.clone(), None);
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

    // Non-Steam, un par un (throttlé), émis par petits lots.
    let mut batch = Vec::new();
    for g in other_todo {
        match name_meta(&g.title) {
            Issue::Trouve(meta) => {
                cache.insert(g.id.clone(), Some(meta.clone()));
                dirty = true;
                batch.push((g.id.clone(), meta));
            }
            Issue::Absent => {
                cache.insert(g.id.clone(), None);
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
        save_cache(config_dir, &cache);
    }
    out
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
        traduire_lot(&mut lot);
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
        traduire_lot(&mut lot);
        assert_eq!(lot[0].1.genre.as_deref(), Some("Jeu de rôle"));
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
        std::fs::write(dir.join(ANCIENS_CACHES[1]), serde_json::to_string(&v1).unwrap()).unwrap();

        let migre = load_cache(&dir);
        assert_eq!(migre.len(), 1, "seule la fiche trouvée survit");
        assert!(migre.contains_key("steam:1"));
        assert!(!migre.contains_key("steam:2"), "l'introuvable doit être re-cherché");

        assert!(cache_file(&dir).exists(), "la v2 est écrite dès la première lecture");
        assert!(dir.join(ANCIENS_CACHES[1]).exists(), "l'ancien reste, filet en cas d'écriture coupée");

        // Deuxième lecture : on repart de la v2, et l'introuvable ne ressuscite pas.
        assert_eq!(load_cache(&dir).len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
