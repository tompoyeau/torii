//! Catégories de la bibliothèque : les tags Steam de chaque jeu.
//!
//! 🔑 **UN SEUL VOCABULAIRE, CELUI DE STEAM.** Le genre IGDB seul donnait une poignée de
//! catégories écrasées par deux ou trois géantes (285 « Tir », 199 « Jeu de rôle » sur
//! une bibliothèque de 930 jeux), parce qu'on ne gardait que le premier genre de chaque
//! fiche. Les tags que les joueurs posent sur Steam disent ce qu'on cherche vraiment :
//! Roguelike, Survie, Coop, Monde ouvert, Metroidvania… Ils sont aussi déjà traduits par
//! Steam (`tagdata/populartags/<langue>`), et ce sont ceux des rayons de la Boutique.
//!
//! Trois chemins, du plus sûr au plus approché :
//! 1. un jeu Steam : les tags de son appid ;
//! 2. un jeu acheté ailleurs (Epic, GOG, Ubisoft…) : l'appid Steam que lui connaît IGDB
//!    (`IgdbMeta::steam_appid`), donc les mêmes tags que sa version Steam ;
//! 3. un jeu absent de Steam (Valorant, League of Legends, World of Warcraft) : ses
//!    genres, thèmes, modes et vues IGDB, convertis par des tables fermées (`tags_igdb`).
//!
//! Endpoint : `IStoreBrowseService/GetItems` (sans clé), jusqu'à 200 appids par appel
//! (mesuré le 24/10/2026 : 300 dépassent la longueur d'URL acceptée, réponse 400). On en
//! demande 150. Une bibliothèque de 470 jeux Steam coûte donc 4 appels, puis un mois de
//! cache.

use super::igdb::{Classement, IgdbMeta};
use super::vitrine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Appids par appel à `GetItems`.
const LOT: usize = 150;
/// Le maximum que Steam rend par jeu : ce sont les tags affichés sur sa page.
const TAGS_PAR_JEU: u32 = 20;
/// Les tags d'un jeu bougent peu : un mois de cache.
const DUREE_TAGS: u64 = 30 * 86_400;
/// Un jeu sans fiche Steam (retiré de la vente, pas vendu dans le pays) est redemandé
/// au bout d'une semaine.
const DUREE_SANS_TAGS: u64 = 7 * 86_400;

/// Tags qui ne deviennent jamais une catégorie.
///
/// - un contenu, pas un jeu : Gore (4345), Violence (4667), Nudité (6650), Contenu à
///   caractère sexuel (12095), Hentai (9130) — la Boutique les masque aussi ;
/// - une appréciation, pas un genre : Super B.O. (1756), Addictif (4190), Beau (5411),
///   Épopée (3965), Culte (1693), Mèmes (10397) ;
/// - un état passager : Accès anticipé (493).
pub(super) const EXCLUS: [u64; 12] = [4345, 4667, 6650, 12095, 9130, 1756, 4190, 5411, 3965, 1693, 10397, 493];

// --- Tags Steam des jeux -----------------------------------------------------------

#[derive(Serialize, Deserialize, Clone)]
struct Entree {
    /// Du plus voté au moins voté. Vide : Steam n'a pas de fiche pour ce jeu.
    tags: Vec<u64>,
    at: u64,
}

impl Entree {
    fn perimee(&self, t: u64) -> bool {
        let duree = if self.tags.is_empty() { DUREE_SANS_TAGS } else { DUREE_TAGS };
        t.saturating_sub(self.at) >= duree
    }
}

type CacheTags = HashMap<u64, Entree>;

/// `enrich_igdb` et « Actualiser les infos » peuvent écrire en même temps : sans verrou,
/// la seconde écriture effacerait ce qu'a trouvé la première.
static VERROU: Mutex<()> = Mutex::new(());

/// ⚠️ Le mot `cache` dans le nom n'est pas décoratif : « Vider le cache » (Paramètres)
/// supprime les `*cache*.json` du dossier de config.
fn fichier(dir: &Path) -> PathBuf {
    dir.join("steam_game_tags_cache_v1.json")
}

fn lire(dir: &Path) -> CacheTags {
    std::fs::read_to_string(fichier(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn ecrire(dir: &Path, cache: &CacheTags) {
    if std::fs::create_dir_all(dir).is_ok() {
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = std::fs::write(fichier(dir), json);
        }
    }
}

/// Oublie les tags d'un jeu (bouton « Actualiser les infos »).
pub fn oublier(dir: &Path, appid: u64) {
    let _verrou = VERROU.lock();
    let mut cache = lire(dir);
    if cache.remove(&appid).is_some() {
        ecrire(dir, &cache);
    }
}

/// Tags Steam (identifiants, du plus voté au moins voté) d'une liste d'appids. Les jeux
/// sans tags sont absents du résultat.
///
/// Un cache périmé sert encore si Steam ne répond pas : des tags d'il y a deux mois
/// valent mieux que pas de catégorie du tout.
pub fn tags_steam(appids: &[u64], dir: &Path) -> HashMap<u64, Vec<u64>> {
    let _verrou = VERROU.lock();
    let mut cache = lire(dir);
    let t = maintenant();

    let mut vus = HashSet::new();
    let a_demander: Vec<u64> = appids
        .iter()
        .copied()
        .filter(|a| vus.insert(*a))
        .filter(|a| cache.get(a).map_or(true, |e| e.perimee(t)))
        .collect();

    let mut modifie = false;
    for lot in a_demander.chunks(LOT) {
        let Some(items) = demander(lot) else { continue };
        let mut recus: HashMap<u64, Vec<u64>> = items.iter().filter_map(lire_item).collect();
        // Steam a répondu : un appid qui n'a rien rendu n'a pas de fiche. Ça se mémorise,
        // pour une semaine seulement.
        for appid in lot {
            let tags = recus.remove(appid).unwrap_or_default();
            cache.insert(*appid, Entree { tags, at: t });
        }
        modifie = true;
    }
    if modifie {
        ecrire(dir, &cache);
    }

    appids
        .iter()
        .filter_map(|a| {
            let e = cache.get(a)?;
            (!e.tags.is_empty()).then(|| (*a, e.tags.clone()))
        })
        .collect()
}

/// Un appel à `GetItems`, réduit aux tags. `None` = panne.
fn demander(appids: &[u64]) -> Option<Vec<Value>> {
    let ids: Vec<Value> = appids.iter().map(|a| json!({ "appid": a })).collect();
    vitrine::steam(
        vitrine::GET_ITEMS,
        &json!({
            "ids": ids,
            "context": vitrine::contexte(),
            "data_request": { "include_tag_count": TAGS_PAR_JEU },
        }),
    )
}

/// `(appid, tags)` d'un item de `GetItems`. `None` si Steam n'a pas de fiche pour ce jeu.
fn lire_item(item: &Value) -> Option<(u64, Vec<u64>)> {
    if item["success"].as_u64() != Some(1) {
        return None;
    }
    let appid = item["appid"].as_u64()?;
    let tags: Vec<u64> = item["tags"]
        .as_array()?
        .iter()
        .filter_map(|t| t["tagid"].as_u64())
        .collect();
    (!tags.is_empty()).then_some((appid, tags))
}

// --- Correspondance IGDB → Steam ----------------------------------------------------

/// Tags Steam d'un jeu d'après son classement IGDB : le repli des jeux absents de Steam.
///
/// 🔑 Des tables et pas une heuristique : les quatre vocabulaires d'IGDB sont **fermés**
/// (relevés le 24/10/2026 : 23 genres, 22 thèmes, 6 modes, 7 vues). Un identifiant
/// inconnu (ajout futur chez IGDB) ne donne simplement rien.
///
/// ⚠️ Pas de traduction ici : les noms viennent de Steam, dans la langue de l'interface,
/// comme pour les jeux Steam. Un jeu hors Steam rejoint donc les MÊMES catégories.
pub fn tags_igdb(c: &Classement) -> Vec<u64> {
    let mut vus = HashSet::new();
    c.genres
        .iter()
        .flat_map(|g| genre_igdb(*g))
        .chain(c.themes.iter().flat_map(|t| theme_igdb(*t)))
        .chain(c.modes.iter().flat_map(|m| mode_igdb(*m)))
        .chain(c.vues.iter().flat_map(|v| vue_igdb(*v)))
        .copied()
        .filter(|t| vus.insert(*t))
        .collect()
}

fn genre_igdb(id: u64) -> &'static [u64] {
    match id {
        2 => &[1698],       // Point-and-click → Pointer et cliquer
        4 => &[1743],       // Fighting → Combat
        5 => &[1774],       // Shooter → Tir
        7 => &[1621],       // Music → Musique
        8 => &[1625],       // Platform → Plateforme
        9 => &[1664],       // Puzzle → Casse-tête
        10 => &[699],       // Racing → Course
        11 => &[1676, 9],   // Real Time Strategy → Stratégie en temps réel, Stratégie
        12 => &[122],       // Role-playing → RPG
        13 => &[599],       // Simulator → Simulation
        14 => &[701],       // Sport → Sport
        15 => &[9],         // Strategy → Stratégie
        16 => &[1741, 9],   // Turn-based strategy → Stratégie au tour par tour, Stratégie
        24 => &[1708],      // Tactical → Tactique
        25 => &[1646, 4158], // Hack and slash/Beat 'em up → Hack 'n' Slash, Beat Them All
        26 => &[10437],     // Quiz/Trivia → Quiz
        30 => &[6621],      // Pinball → Flipper
        31 => &[21],        // Adventure → Aventure
        32 => &[492],       // Indie → Indépendant
        33 => &[1773],      // Arcade → Arcade
        34 => &[3799],      // Visual Novel → Roman graphique
        35 => &[1666, 1770], // Card & Board Game → Jeu de cartes, Jeu de société
        36 => &[1718],      // MOBA → MOBA
        _ => &[],
    }
}

fn theme_igdb(id: u64) -> &'static [u64] {
    match id {
        1 => &[19],     // Action
        17 => &[1684],  // Fantasy
        18 => &[3942],  // Science fiction
        19 => &[1667],  // Horror → Horreur
        20 => &[4064],  // Thriller
        21 => &[1662],  // Survival → Survie
        22 => &[3987],  // Historical → Historique
        23 => &[1687],  // Stealth → Infiltration
        27 => &[1719],  // Comedy → Comédie
        28 => &[12472], // Business → Gestion
        33 => &[3810],  // Sandbox → Bac à sable
        34 => &[1036],  // Educational → Éducation
        35 => &[5350],  // Kids → Familial
        38 => &[1695],  // Open world → Monde ouvert
        39 => &[1678],  // Warfare → Guerre
        40 => &[7178],  // Party → En groupe
        41 => &[1670],  // 4X
        43 => &[5716],  // Mystery → Mystère
        44 => &[4947],  // Romance → Romantique
        // Drama (31), Non-fiction (32) : pas d'équivalent chez Steam.
        // Erotic (42) : jamais une catégorie (cf. `EXCLUS`).
        _ => &[],
    }
}

fn mode_igdb(id: u64) -> &'static [u64] {
    match id {
        1 => &[4182],   // Single player → Jeu solo
        2 => &[3859],   // Multiplayer → Multijoueur
        3 => &[1685],   // Co-operative → Coop
        4 => &[10816],  // Split screen → Écran partagé
        5 => &[128],    // MMO → Massivement multijoueur
        6 => &[176981], // Battle Royale
        _ => &[],
    }
}

fn vue_igdb(id: u64) -> &'static [u64] {
    match id {
        1 => &[3839],  // First person → 1ʳᵉ personne
        2 => &[1697],  // Third person → 3ᵉ personne
        3 => &[5851],  // Bird view / Isometric → Isométrique
        4 => &[3798],  // Side view → Défilement horizontal
        5 => &[31275], // Text → Basé sur du texte
        7 => &[21978], // Virtual Reality → VR
        // Auditory (6) : pas d'équivalent chez Steam.
        _ => &[],
    }
}

// --- Assemblage ------------------------------------------------------------------------

/// L'appid Steam d'un jeu : le sien s'il vient de Steam, sinon celui que lui connaît IGDB.
pub fn appid(id: &str, meta: Option<&IgdbMeta>) -> Option<u64> {
    match id.strip_prefix("steam:") {
        Some(a) => a.parse().ok(),
        None => meta?.steam_appid,
    }
}

/// Ce qu'il faut pour nommer les catégories d'une bibliothèque : les tags Steam déjà
/// connus et leurs noms dans la langue de l'interface.
pub struct Categories {
    dir: PathBuf,
    steam: HashMap<u64, Vec<u64>>,
    noms: HashMap<u64, String>,
}

impl Categories {
    /// Charge les noms des tags et les tags des jeux Steam de la liste.
    pub fn charger(ids: &[&str], dir: &Path) -> Self {
        let appids: Vec<u64> = ids.iter().filter_map(|id| appid(id, None)).collect();
        Categories {
            dir: dir.to_path_buf(),
            steam: tags_steam(&appids, dir),
            noms: vitrine::noms_tags(dir),
        }
    }

    /// Demande à Steam les tags des appids qu'on n'a pas encore (jeux achetés ailleurs).
    pub fn completer(&mut self, appids: impl IntoIterator<Item = u64>) {
        let manquants: Vec<u64> = appids.into_iter().filter(|a| !self.steam.contains_key(a)).collect();
        if !manquants.is_empty() {
            self.steam.extend(tags_steam(&manquants, &self.dir));
        }
    }

    /// Les catégories d'un jeu, nommées : ses tags Steam s'il en a, sinon ceux déduits
    /// d'IGDB. Vide si on ne sait rien de lui.
    pub fn du_jeu(&self, id: &str, meta: Option<&IgdbMeta>) -> Vec<String> {
        let tags = appid(id, meta)
            .and_then(|a| self.steam.get(&a).cloned())
            .or_else(|| meta?.classement.as_ref().map(tags_igdb))
            .unwrap_or_default();
        nommer(&tags, &self.noms)
    }
}

/// Noms des tags, dans l'ordre, sans les exclus. Un tag que Steam ne nomme pas (retiré
/// de sa liste) est ignoré : mieux vaut une catégorie de moins qu'un numéro à l'écran.
fn nommer(tags: &[u64], noms: &HashMap<u64, String>) -> Vec<String> {
    tags.iter()
        .filter(|t| !EXCLUS.contains(t))
        .filter_map(|t| noms.get(t).cloned())
        .collect()
}

fn maintenant() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque identifiant des quatre vocabulaires d'IGDB (relevés sur `/genres`,
    /// `/themes`, `/game_modes`, `/player_perspectives`) doit donner au moins un tag, sauf
    /// les rares sans équivalent, listés ici exprès. Un oubli laisserait des jeux hors
    /// Steam sans la catégorie que leurs voisins Steam portent.
    #[test]
    fn tout_le_vocabulaire_igdb_a_une_correspondance() {
        let genres = [2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 24, 25, 26, 30, 31, 32, 33, 34, 35, 36];
        let themes = [1, 17, 18, 19, 20, 21, 22, 23, 27, 28, 31, 32, 33, 34, 35, 38, 39, 40, 41, 42, 43, 44];
        let sans_equivalent = [31, 32, 42];
        for g in genres {
            assert!(!genre_igdb(g).is_empty(), "genre IGDB sans tag : {g}");
        }
        for t in themes {
            assert_eq!(theme_igdb(t).is_empty(), sans_equivalent.contains(&t), "thème IGDB {t}");
        }
        for m in 1..=6 {
            assert!(!mode_igdb(m).is_empty(), "mode IGDB sans tag : {m}");
        }
        for v in [1, 2, 3, 4, 5, 7] {
            assert!(!vue_igdb(v).is_empty(), "vue IGDB sans tag : {v}");
        }
    }

    #[test]
    fn un_classement_igdb_donne_des_tags_sans_doublon() {
        // Stratégie temps réel + stratégie au tour par tour : « Stratégie » (9) une fois.
        let c = Classement { genres: vec![11, 16], themes: vec![39], modes: vec![1, 3], vues: vec![3] };
        assert_eq!(tags_igdb(&c), vec![1676, 9, 1741, 1678, 4182, 1685, 5851]);
    }

    /// Les mêmes identifiants n'ont pas le même sens d'une table à l'autre.
    #[test]
    fn genres_et_themes_ne_se_confondent_pas() {
        let genre = Classement { genres: vec![31], ..Default::default() };
        let theme = Classement { themes: vec![31], ..Default::default() };
        assert_eq!(tags_igdb(&genre), vec![21]); // Adventure → Aventure
        assert!(tags_igdb(&theme).is_empty()); // Drama → rien
    }

    #[test]
    fn les_tags_exclus_ne_deviennent_pas_des_categories() {
        let noms: HashMap<u64, String> =
            [(1662, "Survie"), (4345, "Gore"), (1685, "Coop"), (493, "Accès anticipé")]
                .into_iter()
                .map(|(id, n)| (id, n.to_string()))
                .collect();
        assert_eq!(nommer(&[1662, 4345, 1685, 493, 99999], &noms), vec!["Survie", "Coop"]);
    }

    #[test]
    fn steam_l_emporte_sur_igdb() {
        let mut steam = HashMap::new();
        steam.insert(570u64, vec![1662]);
        let noms: HashMap<u64, String> =
            [(1662, "Survie"), (1774, "Tir")].into_iter().map(|(id, n)| (id, n.to_string())).collect();
        let cats = Categories { dir: PathBuf::new(), steam, noms };
        let meta = IgdbMeta {
            steam_appid: Some(570),
            classement: Some(Classement { genres: vec![5], ..Default::default() }),
            ..Default::default()
        };
        // Jeu Epic dont IGDB connaît l'appid Steam : les tags de Steam.
        assert_eq!(cats.du_jeu("epic:abc", Some(&meta)), vec!["Survie"]);
        // Sans appid connu : la correspondance IGDB.
        let hors_steam = IgdbMeta { steam_appid: None, ..meta.clone() };
        assert_eq!(cats.du_jeu("riot:valorant", Some(&hors_steam)), vec!["Tir"]);
        // Un jeu Steam se fie à son propre appid, pas à celui d'IGDB.
        assert!(cats.du_jeu("steam:10", None).is_empty());
    }

    #[test]
    fn lit_un_item_de_get_items() {
        let item = json!({ "appid": 368360, "success": 1, "tags": [ { "tagid": 14139, "weight": 527 }, { "tagid": 4486, "weight": 524 } ] });
        assert_eq!(lire_item(&item), Some((368360, vec![14139, 4486])));
        // Steam n'a pas de fiche : `success` 15, aucun tag.
        assert_eq!(lire_item(&json!({ "appid": 1, "success": 15 })), None);
    }

    #[test]
    fn une_absence_se_redemande_plus_tot_qu_une_fiche() {
        let t = 100 * 86_400;
        let vide = Entree { tags: vec![], at: t - 8 * 86_400 };
        let pleine = Entree { tags: vec![1], at: t - 8 * 86_400 };
        assert!(vide.perimee(t));
        assert!(!pleine.perimee(t));
    }
}
