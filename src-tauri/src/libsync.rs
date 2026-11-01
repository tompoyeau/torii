//! Synchronisation de la bibliothèque vers le service Torii.
//!
//! Le PC dépose la liste de ce qu'il possède ; le serveur la range dans R2 (un objet par
//! appareil) et n'en garde en base qu'un index. Deux usages, dans cet ordre d'importance :
//! retrouver sa bibliothèque sur son **propre** mobile, et laisser ses **amis** voir ce
//! qu'on possède ailleurs que sur Steam — ce que ni Steam ni Discord ne savent faire.
//!
//! ## Deux interrupteurs qui ne disent pas la même chose
//!
//!   * `SocialPrefs::sync_library` (ici, côté client) : **envoyer**. Faux par défaut, comme
//!     le partage de présence. Rien ne quitte la machine tant qu'il est éteint.
//!   * `share_library` (côté serveur, via `set_profile`) : **laisser lire aux amis**. Sa
//!     propre bibliothèque reste lisible par soi seul quand il est éteint.
//!
//! ## Le temps de jeu, à part
//!
//! `SocialPrefs::sync_playtime` (éteint par défaut) envoie en plus le temps de jeu et la dernière
//! session de chaque jeu, pour les retrouver sur ses autres appareils. Ils partent dans un objet
//! distinct que les amis ne peuvent pas lire, et sans toucher à la bibliothèque : sa date et son
//! empreinte, que les amis voient, ne bougent pas quand on joue. Voir `server/src/library.js`.
//!
//! ## Ce qui n'est pas envoyé
//!
//! Les jeux **masqués** (ils n'existent nulle part ailleurs dans Torii) et ceux marqués
//! « ne pas diffuser » (`PRESENCE_MUTED`) : quelqu'un qui a pris la peine de cacher un jeu
//! à ses amis dans la présence ne s'attend pas à le retrouver dans sa bibliothèque
//! partagée. Les DLC, bandes-son et vidéos non plus — ce ne sont pas des jeux.
//!
//! ## Pourquoi une empreinte
//!
//! Une bibliothèque bouge rarement, et un scan a lieu à chaque démarrage. Sans garde-fou,
//! Torii réécrirait le même objet plusieurs fois par jour pour rien. On calcule donc une
//! empreinte du contenu : tant qu'elle ne change pas, aucune requête ne part. C'est la même
//! valeur qui sert d'ETag au serveur, donc elle économise aussi les lectures des amis.

use crate::models::GameDto;
use crate::platforms::id_set;
use crate::social;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Types Steam qui ne sont pas des jeux. Un compte Steam en traîne beaucoup, et ils
/// n'ont rien à faire dans la bibliothèque qu'on montre à un ami.
const NON_JEUX: [&str; 4] = ["dlc", "music", "video", "tool"];

/// Un jeu tel qu'il part sur le serveur. Volontairement pauvre : de quoi l'afficher et le
/// croiser, rien qui décrive des habitudes. Le serveur rejette de toute façon le reste.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibGame {
    /// Clé cross-launcher (`social::game_key`) : la même que celle de la présence, pour
    /// qu'un jeu possédé sur GOG et un jeu joué sur Steam se reconnaissent.
    pub key: String,
    pub title: String,
    /// Tous les launchers où on le possède — c'est là tout l'intérêt côté ami.
    pub platforms: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// Vrai quand le jeu n'arrive QUE par le partage familial Steam : il est dans la
    /// bibliothèque, il se lance, mais il appartient à quelqu'un d'autre du groupe.
    /// 🔑 Le dire est le but même de cette liste : « ce que mon ami possède » deviendrait
    /// faux si on comptait comme sien un jeu qui repartira le jour où sa famille le retire.
    /// Un jeu possédé pour de bon quelque part (même sur un autre launcher) ne l'est pas.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub family_shared: bool,
    /// L'appid Steam quand le jeu y est possédé : sa fiche se relie sans deviner par le titre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_app_id: Option<u64>,
}

/// Le temps de jeu d'un jeu, tel qu'il part sur le serveur (option `sync_playtime`).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatsJeu {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played: Option<i64>,
}

/// Une ligne d'index : un appareil, à moi ou à un ami qui partage.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub account_id: String,
    pub display_name: String,
    pub device_id: String,
    pub device_name: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub game_count: u32,
    #[serde(default)]
    pub size_bytes: u64,
    #[serde(default)]
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibraryIndex {
    #[serde(default)]
    pub mine: Vec<LibraryEntry>,
    #[serde(default)]
    pub friends: Vec<LibraryEntry>,
}

/// La bibliothèque d'un appareil, telle que le serveur la rend.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub device_name: String,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub games: Vec<LibGame>,
}

/* ── Ce qu'on envoie ───────────────────────────────────────────────────────── */

/// Ce jeu peut-il quitter la machine ? 🔑 La même réponse pour la liste et pour le temps de jeu.
fn diffusable(game: &GameDto, muets: &HashSet<String>) -> bool {
    if game.hidden || muets.contains(&game.id) {
        return false;
    }
    if let Some(t) = game.app_type.as_deref() {
        if NON_JEUX.contains(&t.to_lowercase().as_str()) {
            return false;
        }
    }
    !game.title.trim().is_empty()
}

/// Réduit la bibliothèque scannée à ce qui part sur le serveur.
///
/// Fonction **pure** et testée : c'est ici que se joue la promesse faite à l'utilisateur
/// (« un jeu masqué ou muet ne sort pas d'ici »), au même titre que `presence_for` pour la
/// présence. Le regroupement par clé est ce qui produit « je l'ai sur Steam ET sur GOG » :
/// le scan rend une entrée par launcher, l'ami veut une ligne par jeu.
pub fn partageables(games: &[GameDto], muets: &HashSet<String>) -> Vec<LibGame> {
    // BTreeMap : l'ordre de sortie ne dépend pas de l'ordre du scan, donc deux scans
    // identiques donnent la même empreinte — sans quoi on réenverrait tout à chaque fois.
    let mut par_cle: BTreeMap<String, LibGame> = BTreeMap::new();

    // 🔑 Dans l'ordre des ids, pas dans celui du scan. Pour un jeu présent sur plusieurs
    // launchers, le titre et la jaquette retenus sont ceux de la première copie vue ; or le
    // scan sort d'une table de hachage, et à titre égal son ordre change d'une fois sur
    // l'autre. L'empreinte changeait donc presque à chaque scan et la bibliothèque repartait
    // au serveur sans que rien n'ait bougé (51 jeux sur 941 relevés le 31/10/2026) — ce que
    // la veille de la bibliothèque, qui relève toutes les 30 min, aurait multiplié.
    let mut dans_l_ordre: Vec<&GameDto> = games.iter().collect();
    dans_l_ordre.sort_by(|a, b| a.id.cmp(&b.id));

    for game in dans_l_ordre.into_iter().filter(|g| diffusable(g, muets)) {
        // ⚠️ HTTPS seulement : le serveur écarte le reste, autant ne pas l'envoyer (et
        // surtout ne pas le compter dans l'empreinte, qui ne correspondrait plus).
        let cover = game
            .cover_url
            .as_deref()
            .filter(|u| u.starts_with("https://"))
            .map(str::to_string);

        let entree = par_cle
            .entry(social::game_key(&game.title))
            .or_insert_with(|| LibGame {
                key: social::game_key(&game.title),
                title: game.title.trim().to_string(),
                platforms: Vec::new(),
                cover: None,
                // Vrai jusqu'à preuve du contraire : c'est le `&=` plus bas qui tranche,
                // une fois toutes les sources du jeu vues.
                family_shared: true,
                steam_app_id: None,
            });
        if game.platform == "steam" && entree.steam_app_id.is_none() {
            entree.steam_app_id = game.launch_target.parse().ok();
        }
        if !entree.platforms.contains(&game.platform) {
            entree.platforms.push(game.platform.clone());
        }
        // Une seule source possédée pour de bon suffit à faire du jeu le sien — y compris
        // quand la copie familiale Steam côtoie un exemplaire acheté sur GOG.
        entree.family_shared &= game.family_shared;
        // Un jeu installé (scan local) n'a souvent pas de jaquette là où sa jumelle
        // possédée en a une : la première trouvée gagne, peu importe laquelle des deux.
        if entree.cover.is_none() {
            entree.cover = cover;
        }
    }

    // Les plateformes aussi doivent être ordonnées : l'ordre du scan varie, l'empreinte non.
    for jeu in par_cle.values_mut() {
        jeu.platforms.sort();
    }
    par_cle.into_values().collect()
}

/// Le temps de jeu de ce qui peut quitter la machine. Fonction **pure**, comme `partageables` : un
/// jeu masqué ou muet ne part pas. Un même jeu sur deux launchers garde le plus grand temps et la
/// session la plus récente, comme la carte qui les réunit dans l'interface (`mergeDuplicates`).
pub fn stats_partageables(games: &[GameDto], muets: &HashSet<String>) -> Vec<StatsJeu> {
    let mut par_cle: BTreeMap<String, StatsJeu> = BTreeMap::new();
    for game in games.iter().filter(|g| diffusable(g, muets)) {
        let minutes = game.playtime_minutes.filter(|m| *m > 0);
        let derniere = game.last_played.filter(|t| *t > 0);
        if minutes.is_none() && derniere.is_none() {
            continue;
        }
        let cle = social::game_key(&game.title);
        let entree = par_cle.entry(cle.clone()).or_insert(StatsJeu { key: cle, minutes: None, last_played: None });
        entree.minutes = entree.minutes.max(minutes);
        entree.last_played = entree.last_played.max(derniere);
    }
    par_cle.into_values().collect()
}

/// L'empreinte d'un envoi de temps de jeu : tant qu'on n'a pas joué, rien ne repart.
fn empreinte_stats(stats: &[StatsJeu]) -> String {
    fnv1a(&serde_json::to_vec(stats).unwrap_or_default())
}

/// FNV-1a 64 bits, en hexadécimal.
fn fnv1a(octets: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in octets {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    format!("{h:016x}")
}

/// Empreinte du contenu (FNV-1a 64 bits, en hexadécimal).
///
/// 🔑 Volontairement **pas** une empreinte cryptographique, et donc aucune dépendance
/// nouvelle : elle ne protège rien, elle répond à « est-ce que ça a changé depuis la
/// dernière fois ? ». Le serveur ne la recalcule pas — il la stocke telle quelle et s'en
/// sert d'ETag. Une collision (1 sur 1,8 × 10¹⁹) ne coûterait qu'une bibliothèque périmée
/// jusqu'au changement suivant.
pub fn empreinte(games: &[LibGame]) -> String {
    let mut tampon: Vec<u8> = Vec::new();
    let mut avale = |octets: &[u8]| tampon.extend_from_slice(octets);
    for jeu in games {
        avale(jeu.key.as_bytes());
        avale(b"\x1f");
        avale(jeu.title.as_bytes());
        avale(b"\x1f");
        avale(jeu.platforms.join(",").as_bytes());
        avale(b"\x1f");
        avale(jeu.cover.as_deref().unwrap_or("").as_bytes());
        avale(b"\x1f");
        // Sans lui, une bibliothèque dont seul le statut familial change ne repartirait
        // jamais : l'empreinte doit couvrir tout ce que l'ami verra.
        avale(if jeu.family_shared { b"f" } else { b"o" });
        avale(jeu.steam_app_id.map(|id| id.to_string()).unwrap_or_default().as_bytes());
        avale(b"\x1e");
    }
    fnv1a(&tampon)
}

/* ── Appareil ──────────────────────────────────────────────────────────────── */

/// Identifiant stable de CET appareil, créé au premier besoin et gardé dans les
/// préférences. C'est lui qui empêche deux PC de s'écraser mutuellement sur le serveur.
///
/// Ni secret ni universellement unique : il n'est comparé qu'aux autres appareils du même
/// compte. L'horloge et le nom de la machine suffisent donc à le distinguer.
fn appareil(config_dir: &Path, prefs: &mut social::SocialPrefs) -> String {
    if let Some(id) = prefs.device_id.clone().filter(|s| !s.is_empty()) {
        return id;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let hote = empreinte(&[LibGame {
        key: social::whoami_host(),
        title: String::new(),
        platforms: Vec::new(),
        cover: None,
        family_shared: false,
        steam_app_id: None,
    }]);
    let id = format!("{nanos:x}-{hote}");
    prefs.device_id = Some(id.clone());
    let _ = social::save_prefs(config_dir, prefs);
    id
}

/* ── Envoi ─────────────────────────────────────────────────────────────────── */

/// Résultat d'une tentative de synchronisation, tel que l'interface l'affiche.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    /// Faux si rien n'est parti (synchronisation éteinte, non connecté, ou déjà à jour).
    pub uploaded: bool,
    pub game_count: usize,
    pub digest: String,
    /// Ce qui explique un envoi qui n'a pas eu lieu : `off`, `deconnecte`, `inchange`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
}

/// Envoie la bibliothèque si elle a changé. `force` ignore l'empreinte mémorisée.
///
/// 🔑 Ne renvoie une erreur que si l'envoi lui-même échoue : « éteint » et « pas connecté »
/// sont des situations normales, pas des pannes. Appelée après chaque scan, elle ne doit
/// jamais faire remonter d'erreur à quelqu'un qui n'a rien demandé.
pub fn sync(config_dir: &Path, games: &[GameDto], force: bool) -> Result<SyncResult, String> {
    let mut prefs = social::load_prefs(config_dir);
    let repos = |raison: &str| SyncResult {
        uploaded: false,
        game_count: 0,
        digest: String::new(),
        skipped: Some(raison.to_string()),
    };
    if !prefs.sync_library {
        return Ok(repos("off"));
    }
    if social::token(config_dir).is_err() {
        return Ok(repos("deconnecte"));
    }

    let muets = id_set::PRESENCE_MUTED.load(config_dir);
    let liste = partageables(games, &muets);
    let digest = empreinte(&liste);

    // 🔑 L'empreinte mémorisée est liée au COMPTE : se déconnecter puis se reconnecter
    // avec un autre compte doit tout renvoyer, sinon le nouveau compte reste vide pour
    // toujours. Même chose après une suppression de compte.
    let compte = social::me(config_dir).map(|a| a.id).unwrap_or_default();
    let deja = prefs
        .last_library_sync
        .as_ref()
        .is_some_and(|s| s.digest == digest && s.account_id == compte);
    if deja && !force {
        envoyer_stats(config_dir, games, &muets, &mut prefs, &compte);
        return Ok(SyncResult {
            uploaded: false,
            game_count: liste.len(),
            digest,
            skipped: Some("inchange".into()),
        });
    }

    let device_id = appareil(config_dir, &mut prefs);
    let corps = serde_json::json!({
        "deviceId": device_id,
        "deviceName": social::whoami_host(),
        "digest": digest,
        "games": liste,
    });
    let _: serde_json::Value = social::call(config_dir, "PUT", "/v1/library", Some(corps))?;

    // Mémorisé APRÈS l'accusé de réception : un envoi raté doit être retenté au prochain
    // scan, pas considéré comme fait.
    prefs.last_library_sync = Some(social::LastLibrarySync {
        account_id: compte.clone(),
        digest: digest.clone(),
    });
    let _ = social::save_prefs(config_dir, &prefs);
    envoyer_stats(config_dir, games, &muets, &mut prefs, &compte);

    Ok(SyncResult {
        uploaded: true,
        game_count: liste.len(),
        digest,
        skipped: None,
    })
}

/// Envoie le temps de jeu si l'option est allumée et s'il a changé.
///
/// ⚠️ Après la bibliothèque, jamais à sa place : le serveur n'accepte que les appareils qu'il
/// connaît. Et sans remonter d'erreur : cet envoi ne doit pas faire échouer l'autre. Une panne
/// se rattrape au scan suivant, puisque l'empreinte n'est mémorisée qu'après l'accusé de réception.
fn envoyer_stats(
    config_dir: &Path,
    games: &[GameDto],
    muets: &HashSet<String>,
    prefs: &mut social::SocialPrefs,
    compte: &str,
) {
    if !prefs.sync_playtime {
        return;
    }
    let stats = stats_partageables(games, muets);
    let digest = empreinte_stats(&stats);
    let deja = prefs
        .last_stats_sync
        .as_ref()
        .is_some_and(|s| s.digest == digest && s.account_id == compte);
    if deja {
        return;
    }
    let corps = serde_json::json!({ "deviceId": appareil(config_dir, prefs), "games": stats });
    let envoi: Result<serde_json::Value, String> = social::call(config_dir, "PUT", "/v1/library-stats", Some(corps));
    if envoi.is_ok() {
        prefs.last_stats_sync = Some(social::LastLibrarySync { account_id: compte.to_string(), digest });
        let _ = social::save_prefs(config_dir, prefs);
    }
}

/// Efface du serveur le temps de jeu de tous les appareils du compte.
pub fn forget_stats(config_dir: &Path) -> Result<(), String> {
    let _: serde_json::Value = social::call(config_dir, "DELETE", "/v1/library-stats", None)?;
    Ok(())
}

/* ── Lecture ───────────────────────────────────────────────────────────────── */

/// Index : mes appareils, et ceux des amis qui partagent. Aucun jeu, juste de quoi savoir
/// quoi télécharger.
pub fn index(config_dir: &Path) -> Result<LibraryIndex, String> {
    social::call(config_dir, "GET", "/v1/library", None)
}

/// La bibliothèque d'un appareil (le mien, ou celui d'un ami qui partage).
pub fn fetch(config_dir: &Path, account_id: &str, device_id: &str) -> Result<Snapshot, String> {
    social::call(
        config_dir,
        "GET",
        &format!("/v1/library/{account_id}/{device_id}"),
        None,
    )
}

/// Oublie un appareil côté serveur (objet R2 compris).
pub fn forget_device(config_dir: &Path, device_id: &str) -> Result<(), String> {
    let _: serde_json::Value =
        social::call(config_dir, "DELETE", &format!("/v1/library/{device_id}"), None)?;
    Ok(())
}

/// Cesse de synchroniser : le serveur oublie tout, et l'empreinte locale part avec — sans
/// quoi rallumer l'option ne renverrait rien (l'empreinte n'aurait pas changé).
pub fn forget_all(config_dir: &Path) -> Result<(), String> {
    let _: serde_json::Value = social::call(config_dir, "DELETE", "/v1/library", None)?;
    let mut prefs = social::load_prefs(config_dir);
    prefs.last_library_sync = None;
    prefs.last_stats_sync = None;
    let _ = social::save_prefs(config_dir, &prefs);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jeu(id: &str, titre: &str, plateforme: &str) -> GameDto {
        GameDto {
            id: id.into(),
            title: titre.into(),
            platform: plateforme.into(),
            ..Default::default()
        }
    }

    #[test]
    fn meme_titre_et_jaquette_quel_que_soit_l_ordre() {
        let mut steam = jeu("steam:208650", "Batman™: Arkham Knight", "steam");
        steam.cover_url = Some("https://cdn.steam/208650.jpg".into());
        let mut epic = jeu("epic:Cowbird", "Batman™ Arkham Knight", "epic");
        epic.cover_url = Some("https://cdn.epic/cowbird.jpg".into());
        let a = partageables(&[steam.clone(), epic.clone()], &HashSet::new());
        let b = partageables(&[epic, steam], &HashSet::new());
        assert_eq!(a.len(), 1);
        assert_eq!((&a[0].title, &a[0].cover), (&b[0].title, &b[0].cover));
        assert_eq!(empreinte(&a), empreinte(&b));
    }

    #[test]
    fn le_temps_de_jeu_garde_le_plus_grand_et_le_plus_recent() {
        let mut steam = jeu("steam:292030", "The Witcher 3: Wild Hunt", "steam");
        steam.playtime_minutes = Some(600);
        steam.last_played = Some(100);
        let mut gog = jeu("gog:1207664663", "The Witcher 3: Wild Hunt™", "gog");
        gog.playtime_minutes = Some(90);
        gog.last_played = Some(200);
        let stats = stats_partageables(&[steam, gog], &HashSet::new());
        assert_eq!(stats.len(), 1);
        assert_eq!((stats[0].minutes, stats[0].last_played), (Some(600), Some(200)));
    }

    #[test]
    fn un_jeu_masque_ou_muet_ne_donne_pas_son_temps_de_jeu() {
        let mut masque = jeu("steam:1", "Masqué", "steam");
        masque.hidden = true;
        masque.playtime_minutes = Some(600);
        let mut muet = jeu("steam:2", "Muet", "steam");
        muet.playtime_minutes = Some(600);
        let mut dlc = jeu("steam:3", "Un DLC", "steam");
        dlc.app_type = Some("dlc".into());
        dlc.playtime_minutes = Some(600);
        let muets = HashSet::from(["steam:2".to_string()]);
        assert!(stats_partageables(&[masque, muet, dlc], &muets).is_empty());
    }

    #[test]
    fn un_jeu_sans_temps_de_jeu_ne_part_pas() {
        let mut zero = jeu("riot:lol", "League of Legends", "riot");
        zero.playtime_minutes = Some(0);
        assert!(stats_partageables(&[zero], &HashSet::new()).is_empty());
    }

    #[test]
    fn la_liste_des_jeux_ne_porte_jamais_le_temps_de_jeu() {
        let mut steam = jeu("steam:620", "Portal 2", "steam");
        steam.launch_target = "620".into();
        steam.playtime_minutes = Some(600);
        steam.last_played = Some(100);
        let liste = partageables(&[steam], &HashSet::new());
        assert_eq!(liste[0].steam_app_id, Some(620));
        let json = serde_json::to_string(&liste).unwrap();
        assert!(!json.contains("600") && !json.to_lowercase().contains("minutes") && !json.contains("lastPlayed"));
    }

    #[test]
    fn jouer_ne_change_pas_l_empreinte_de_la_bibliotheque() {
        let avant = jeu("steam:620", "Portal 2", "steam");
        let mut apres = avant.clone();
        apres.playtime_minutes = Some(600);
        apres.last_played = Some(100);
        assert_eq!(
            empreinte(&partageables(&[avant], &HashSet::new())),
            empreinte(&partageables(&[apres], &HashSet::new())),
            "les amis voient cette empreinte : elle ne doit pas dire qu'on vient de jouer"
        );
    }

    #[test]
    fn regroupe_les_launchers_d_un_meme_jeu() {
        let games = vec![
            jeu("steam:292030", "The Witcher 3: Wild Hunt", "steam"),
            jeu("gog:1207664663", "The Witcher 3: Wild Hunt™", "gog"),
        ];
        let liste = partageables(&games, &HashSet::new());
        assert_eq!(liste.len(), 1, "un seul jeu, deux launchers");
        assert_eq!(liste[0].platforms, vec!["gog", "steam"]);
    }

    #[test]
    fn n_envoie_ni_les_masques_ni_les_muets_ni_les_dlc() {
        let mut masque = jeu("steam:1", "Masqué", "steam");
        masque.hidden = true;
        let mut dlc = jeu("steam:2", "Un DLC", "steam");
        dlc.app_type = Some("DLC".into());
        let games = vec![
            masque,
            dlc,
            jeu("steam:3", "Muet", "steam"),
            jeu("steam:4", "Visible", "steam"),
        ];
        let muets: HashSet<String> = ["steam:3".to_string()].into_iter().collect();
        let liste = partageables(&games, &muets);
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].title, "Visible");
    }

    #[test]
    fn garde_la_premiere_jaquette_https_trouvee() {
        let mut installe = jeu("steam:5", "Hollow Knight", "steam");
        installe.cover_url = None;
        let mut possede = jeu("gog:5", "Hollow Knight", "gog");
        possede.cover_url = Some("https://img/hk.jpg".into());
        let mut clair = jeu("epic:5", "Hollow Knight", "epic");
        clair.cover_url = Some("http://img/pas-sur.jpg".into());

        let liste = partageables(&[installe, possede, clair], &HashSet::new());
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0].cover.as_deref(), Some("https://img/hk.jpg"));
    }

    /// Un jeu qui n'arrive que par la famille Steam doit se dire comme tel, et un jeu
    /// possédé quelque part ne doit JAMAIS l'être — c'est toute la distinction demandée :
    /// « ce que mon ami possède » ne peut pas inclure ce qui appartient à son frère.
    #[test]
    fn distingue_le_partage_familial_de_la_possession() {
        let mut famille = jeu("steam:1", "Emprunté", "steam");
        famille.family_shared = true;
        let mut aussi_famille = jeu("steam:2", "Mixte", "steam");
        aussi_famille.family_shared = true;
        // La même « Mixte » achetée sur GOG : elle est bien à lui.
        let achete = jeu("gog:2", "Mixte", "gog");

        let liste = partageables(&[famille, aussi_famille, achete], &HashSet::new());
        let par_titre = |t: &str| liste.iter().find(|g| g.title == t).unwrap().family_shared;
        assert!(par_titre("Emprunté"), "seule source = famille");
        assert!(!par_titre("Mixte"), "une copie possédée suffit");
        assert!(
            !partageables(&[jeu("steam:3", "Acheté", "steam")], &HashSet::new())[0].family_shared,
            "un jeu ordinaire n'est pas familial",
        );
    }

    /// Le serveur ne lit que des noms en camelCase et jette le reste : un renommage
    /// silencieux ici enverrait des bibliothèques vides sans que rien ne proteste.
    #[test]
    fn le_corps_envoye_porte_les_noms_attendus() {
        let mut steam = jeu("steam:1", "Alpha", "steam");
        steam.cover_url = Some("https://img/a.jpg".into());
        let liste = partageables(&[steam], &HashSet::new());
        let corps = serde_json::json!({
            "deviceId": "pc-test",
            "deviceName": "PC",
            "digest": empreinte(&liste),
            "games": liste,
        });
        let jeu0 = &corps["games"][0];
        assert!(jeu0["key"].is_string() && jeu0["title"].is_string());
        assert_eq!(jeu0["platforms"][0], "steam");
        assert_eq!(jeu0["cover"], "https://img/a.jpg");
        assert!(corps["deviceId"].is_string() && corps["digest"].is_string());

        // Une jaquette absente ne doit pas partir en `null` : le serveur teste
        // `cover.startsWith("https://")`, un null y serait simplement ignoré, mais un
        // objet plus petit est un objet moins cher à stocker et à transférer.
        let sans = partageables(&[jeu("gog:2", "Beta", "gog")], &HashSet::new());
        assert_eq!(serde_json::to_value(&sans[0]).unwrap().get("cover"), None);
        // Même raison pour le statut familial : le cas courant est « possédé », il ne
        // mérite pas un champ dans chacun des milliers de jeux d'une bibliothèque.
        assert_eq!(
            serde_json::to_value(&sans[0]).unwrap().get("familyShared"),
            None,
        );
        let mut empruntee = jeu("steam:9", "Gamma", "steam");
        empruntee.family_shared = true;
        let famille = partageables(&[empruntee], &HashSet::new());
        assert_eq!(serde_json::to_value(&famille[0]).unwrap()["familyShared"], true);
    }

    #[test]
    fn l_empreinte_ne_depend_pas_de_l_ordre_du_scan() {
        let a = vec![
            jeu("steam:1", "Alpha", "steam"),
            jeu("gog:2", "Beta", "gog"),
            jeu("epic:1", "Alpha", "epic"),
        ];
        let b = vec![
            jeu("epic:1", "Alpha", "epic"),
            jeu("steam:1", "Alpha", "steam"),
            jeu("gog:2", "Beta", "gog"),
        ];
        assert_eq!(
            empreinte(&partageables(&a, &HashSet::new())),
            empreinte(&partageables(&b, &HashSet::new())),
        );
    }

    #[test]
    fn l_empreinte_change_quand_la_bibliotheque_change() {
        let base = vec![jeu("steam:1", "Alpha", "steam")];
        let plus = vec![jeu("steam:1", "Alpha", "steam"), jeu("gog:2", "Beta", "gog")];
        assert_ne!(
            empreinte(&partageables(&base, &HashSet::new())),
            empreinte(&partageables(&plus, &HashSet::new())),
        );
        // Masquer un jeu doit se voir aussi : sinon il resterait chez les amis.
        let mut cache = plus.clone();
        cache[1].hidden = true;
        assert_ne!(
            empreinte(&partageables(&plus, &HashSet::new())),
            empreinte(&partageables(&cache, &HashSet::new())),
        );
    }
}
