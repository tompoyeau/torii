//! Client du service social de Torii (comptes, amis, présence).
//!
//! Le serveur vit dans `server/` ; son contrat est décrit dans `server/README.md`.
//!
//! 🔑 Le jeton de session est rangé dans les identifiants chiffrés (`credentials.dat`,
//! DPAPI) comme les jetons des launchers : c'est un secret de longue durée, il n'a rien
//! à faire dans un fichier en clair ni dans le `localStorage` de la WebView.

use crate::accounts::secrets;
use crate::platforms::id_set;
use crate::procwatch;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{Emitter, Manager};

/// URL de l'API. Surchargeable par `TORII_API` pour développer contre un serveur local
/// (`npx wrangler dev` dans `server/`), sans quoi il faudrait recompiler pour tester.
///
/// 🔑 Domaine propre, PAS l'URL `*.workers.dev` : cette adresse part codée en dur dans
/// chaque version installée (et demain dans l'application mobile). Un domaine à soi
/// permet de changer d'hébergeur sans avoir à republier chez tous les utilisateurs.
const DEFAULT_API: &str = "https://torii-api.topo-host.com";

fn api() -> String {
    std::env::var("TORII_API").unwrap_or_else(|_| DEFAULT_API.to_string())
}

const TIMEOUT: Duration = Duration::from_secs(20);

/// Compte Torii tel que le front l'affiche.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub friend_code: String,
    #[serde(default)]
    pub steam_id: Option<String>,
    #[serde(default)]
    pub steam_discoverable: bool,
}

/// Un ami et sa présence. `status` vaut `in-game`, `online`, `away` ou `offline` —
/// ce dernier signifiant « aucune nouvelle depuis 90 s », donc Torii fermé.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub id: String,
    pub display_name: String,
    /// SteamID, si l'ami s'est rendu découvrable — sert à fusionner sa ligne avec sa
    /// fiche Steam (avatar, présence Steam) au lieu d'afficher deux fois la personne.
    #[serde(default)]
    pub steam_id: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub game_key: Option<String>,
    #[serde(default)]
    pub game_title: Option<String>,
    #[serde(default)]
    pub since: Option<i64>,
}

/// Une personne sans présence (demande d'ami en attente, suggestion).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub steam_id: Option<String>,
}

/// Le cercle complet, tel que renvoyé par `GET /friends` et par chaque battement de cœur.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Circle {
    #[serde(default)]
    pub friends: Vec<Friend>,
    #[serde(default)]
    pub incoming: Vec<Person>,
    #[serde(default)]
    pub outgoing: Vec<Person>,
}

/// État de présence publié par ce PC.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    pub status: String,
    #[serde(default)]
    pub game_key: Option<String>,
    #[serde(default)]
    pub game_title: Option<String>,
    #[serde(default)]
    pub since: Option<i64>,
}

/* ── Transport ─────────────────────────────────────────────────────────────── */

/// Traduit une réponse d'erreur de l'API en message lisible.
///
/// Le serveur répond `{ error, message }` : on remonte `message`, écrit pour être
/// affiché tel quel. Un corps illisible (panne, proxy d'hôtel…) donne un repli
/// générique plutôt qu'un pavé technique.
fn api_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, resp) => {
            let body: serde_json::Value = resp.into_json().unwrap_or_default();
            match body["message"].as_str() {
                Some(msg) => msg.to_string(),
                None => format!("Le serveur a répondu {code}."),
            }
        }
        ureq::Error::Transport(_) => "Service injoignable. Vérifie ta connexion.".into(),
    }
}

/// Jeton de session stocké, ou une erreur explicite si personne n'est connecté.
fn token(config_dir: &Path) -> Result<String, String> {
    secrets::load(config_dir)
        .torii_token
        .ok_or_else(|| "Non connecté à Torii.".to_string())
}

/// Requête authentifiée. `body` absent = GET.
fn call<T: for<'de> Deserialize<'de>>(
    config_dir: &Path,
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<T, String> {
    let token = token(config_dir)?;
    let req = ureq::request(method, &format!("{}{path}", api()))
        .timeout(TIMEOUT)
        .set("authorization", &format!("Bearer {token}"));
    let resp = match body {
        Some(json) => req.send_json(json),
        None => req.call(),
    }
    .map_err(api_error)?;
    resp.into_json().map_err(|e| e.to_string())
}

/* ── Connexion ─────────────────────────────────────────────────────────────── */

/// Demande l'envoi d'un code à cette adresse. Renvoie le code lui-même **uniquement**
/// si le serveur tourne en mode développement (`DEV_CODES=1`), auquel cas le front
/// l'affiche pour éviter d'attendre un e-mail qui ne partira pas.
pub fn request_code(email: &str) -> Result<Option<String>, String> {
    let resp = ureq::post(&format!("{}/v1/auth/request-code", api()))
        .timeout(TIMEOUT)
        .send_json(serde_json::json!({ "email": email }))
        .map_err(api_error)?;
    let body: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
    Ok(body["devCode"].as_str().map(str::to_string))
}

/// Échange le code contre une session et la persiste (chiffrée). Renvoie le compte.
pub fn verify(config_dir: &Path, email: &str, code: &str) -> Result<Account, String> {
    #[derive(Deserialize)]
    struct Verified {
        token: String,
        account: Account,
    }
    // Le nom d'appareil aide à s'y retrouver quand le mobile arrivera : chaque session
    // est indépendante et révocable séparément.
    let device = format!("PC {}", whoami_host());
    let resp = ureq::post(&format!("{}/v1/auth/verify", api()))
        .timeout(TIMEOUT)
        .send_json(serde_json::json!({ "email": email, "code": code, "device": device }))
        .map_err(api_error)?;
    let verified: Verified = resp.into_json().map_err(|e| e.to_string())?;

    let mut creds = secrets::load(config_dir);
    creds.torii_token = Some(verified.token);
    secrets::save(config_dir, &creds)?;
    Ok(verified.account)
}

/// Ferme la session côté serveur puis efface le jeton local.
///
/// L'effacement local a lieu **même si l'appel échoue** : sinon une panne réseau
/// laisserait l'utilisateur bloqué dans un état « connecté » dont il ne peut pas sortir.
pub fn logout(config_dir: &Path) -> Result<(), String> {
    let _: Result<serde_json::Value, String> =
        call(config_dir, "POST", "/v1/auth/logout", Some(serde_json::json!({})));
    let mut creds = secrets::load(config_dir);
    creds.torii_token = None;
    secrets::save(config_dir, &creds)
}

/// Compte connecté, ou `None` si aucune session valide (jeton absent ou révoqué).
pub fn me(config_dir: &Path) -> Option<Account> {
    #[derive(Deserialize)]
    struct Wrapper {
        account: Account,
    }
    let w: Wrapper = call(config_dir, "GET", "/v1/me", None).ok()?;
    Some(w.account)
}

/// Met à jour le profil. Chaque champ absent est laissé tel quel côté serveur.
pub fn set_profile(
    config_dir: &Path,
    display_name: Option<String>,
    steam_id: Option<String>,
    steam_discoverable: Option<bool>,
) -> Result<Account, String> {
    #[derive(Deserialize)]
    struct Wrapper {
        account: Account,
    }
    let mut patch = serde_json::Map::new();
    if let Some(v) = display_name {
        patch.insert("displayName".into(), v.into());
    }
    // 🔑 Chaîne VIDE = délier, `None` = ne pas toucher.
    //
    // Un `null` JSON se désérialise en `None` dans un `Option<String>` : impossible d'y
    // distinguer « je ne parle pas de ce champ » de « efface-le ». Envoyer `null` pour
    // délier ne faisait donc rien du tout — le SteamID restait en base alors que
    // l'interface croyait l'avoir retiré.
    if let Some(v) = steam_id {
        patch.insert(
            "steamId".into(),
            if v.is_empty() { serde_json::Value::Null } else { v.into() },
        );
    }
    if let Some(v) = steam_discoverable {
        patch.insert("steamDiscoverable".into(), v.into());
    }
    let w: Wrapper = call(config_dir, "PATCH", "/v1/me", Some(patch.into()))?;
    Ok(w.account)
}

/* ── Amis ──────────────────────────────────────────────────────────────────── */

pub fn circle(config_dir: &Path) -> Result<Circle, String> {
    call(config_dir, "GET", "/v1/friends", None)
}

pub fn invite(config_dir: &Path, friend_code: &str) -> Result<(), String> {
    let _: serde_json::Value = call(
        config_dir,
        "POST",
        "/v1/friends/invite",
        Some(serde_json::json!({ "friendCode": friend_code })),
    )?;
    Ok(())
}

/// Invite quelqu'un trouvé par suggestion : on connaît son identifiant, pas son code.
pub fn invite_account(config_dir: &Path, account_id: &str) -> Result<(), String> {
    let _: serde_json::Value = call(
        config_dir,
        "POST",
        "/v1/friends/invite",
        Some(serde_json::json!({ "accountId": account_id })),
    )?;
    Ok(())
}

pub fn respond(config_dir: &Path, account_id: &str, accept: bool) -> Result<(), String> {
    let _: serde_json::Value = call(
        config_dir,
        "POST",
        "/v1/friends/respond",
        Some(serde_json::json!({ "accountId": account_id, "accept": accept })),
    )?;
    Ok(())
}

pub fn remove_friend(config_dir: &Path, account_id: &str) -> Result<(), String> {
    let _: serde_json::Value =
        call(config_dir, "DELETE", &format!("/v1/friends/{account_id}"), None)?;
    Ok(())
}

/// Régénère son code d'ami : l'ancien cesse aussitôt de fonctionner.
pub fn rotate_code(config_dir: &Path) -> Result<String, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Rotated {
        friend_code: String,
    }
    let r: Rotated = call(
        config_dir,
        "POST",
        "/v1/friends/code",
        Some(serde_json::json!({})),
    )?;
    Ok(r.friend_code)
}

/// Amis Steam déjà sur Torii. Ne renvoie que les comptes découvrables, et exige que le
/// demandeur le soit aussi (le serveur refuse sinon).
pub fn suggestions(config_dir: &Path, steam_ids: &[String]) -> Result<Vec<Person>, String> {
    #[derive(Deserialize)]
    struct Wrapper {
        suggestions: Vec<Person>,
    }
    let w: Wrapper = call(
        config_dir,
        "POST",
        "/v1/friends/suggestions",
        Some(serde_json::json!({ "steamIds": steam_ids })),
    )?;
    Ok(w.suggestions)
}

/* ── Présence ──────────────────────────────────────────────────────────────── */

/// Publie l'état de ce PC **et** récupère le cercle en retour : le battement de cœur
/// sert aussi de lecture, ce qui divise par deux le nombre de requêtes.
pub fn publish(config_dir: &Path, presence: &Presence) -> Result<Circle, String> {
    call(
        config_dir,
        "PUT",
        "/v1/presence",
        Some(serde_json::to_value(presence).map_err(|e| e.to_string())?),
    )
}

/// Disparaît immédiatement de la vue des amis (mode invisible, fermeture de Torii).
pub fn clear_presence(config_dir: &Path) -> Result<(), String> {
    let _: serde_json::Value = call(config_dir, "DELETE", "/v1/presence", None)?;
    Ok(())
}

/* ── Réglages de partage ───────────────────────────────────────────────────── */

/// Rythme du battement de cœur. Le serveur périme une présence à 90 s : trois
/// battements manqués suffisent donc à passer hors ligne.
const HEARTBEAT: Duration = Duration::from_secs(30);

/// Réglages de présence, persistés dans `social_prefs.json`.
///
/// 🔑 `share_presence` est **faux par défaut**. Rien ne quitte la machine tant que
/// l'utilisateur ne l'a pas explicitement activé : une présence dit à quelle heure on
/// est devant son PC, ce n'est pas une donnée qu'on diffuse par défaut.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct SocialPrefs {
    pub share_presence: bool,
    /// Minutes d'inactivité avant de passer en « absent ».
    pub away_after_minutes: u32,
}

impl Default for SocialPrefs {
    fn default() -> Self {
        SocialPrefs {
            share_presence: false,
            away_after_minutes: 10,
        }
    }
}

fn prefs_path(config_dir: &Path) -> PathBuf {
    config_dir.join("social_prefs.json")
}

pub fn load_prefs(config_dir: &Path) -> SocialPrefs {
    std::fs::read_to_string(prefs_path(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_prefs(config_dir: &Path, prefs: &SocialPrefs) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(prefs_path(config_dir), json).map_err(|e| e.to_string())
}

/* ── Battement de cœur ─────────────────────────────────────────────────────── */

/// Clé de jeu commune à toutes les plateformes : minuscules, caractères alphanumériques
/// seulement. « THE WITCHER 3: WILD HUNT™ » (Epic) et « The Witcher 3: Wild Hunt »
/// (Steam) donnent la même clé — c'est ce qui permet de voir que deux amis jouent au
/// même jeu depuis deux launchers différents.
///
/// ⚠️ Repli en attendant que l'identifiant IGDB soit persisté : deux jeux au titre
/// rigoureusement identique seraient confondus (rare, et sans conséquence ici puisque
/// la clé ne sert qu'à rapprocher un affichage).
pub fn game_key(title: &str) -> String {
    let slug: String = title.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    format!("title:{slug}")
}

/// Démarre le fil qui publie la présence et récupère le cercle (un seul aller-retour).
pub fn spawn_heartbeat(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut publishing = false;
        loop {
            std::thread::sleep(HEARTBEAT);
            publishing = beat(&app, publishing);
        }
    });
}

/// Un battement. Renvoie `true` si une présence est actuellement publiée — ce qui
/// permet de l'effacer proprement si le partage vient d'être coupé.
fn beat(app: &tauri::AppHandle, publishing: bool) -> bool {
    let Ok(dir) = app.path().app_config_dir() else {
        return publishing;
    };
    // Pas de compte connecté : le service social est simplement inactif.
    if secrets::load(&dir).torii_token.is_none() {
        return false;
    }

    let prefs = load_prefs(&dir);
    if !prefs.share_presence {
        // Partage coupé alors qu'on publiait : on disparaît tout de suite plutôt que
        // d'attendre les 90 s de péremption.
        if publishing {
            let _ = clear_presence(&dir);
        }
        return false;
    }

    // Un jeu réduit au silence est traité comme si rien ne tournait.
    let muted = id_set::PRESENCE_MUTED.load(&dir);
    let game = procwatch::current_game(app).filter(|(id, _, _)| !muted.contains(id));

    let Some(presence) = presence_for(&prefs, game, procwatch::idle_seconds()) else {
        if publishing {
            let _ = clear_presence(&dir);
        }
        return false;
    };

    match publish(&dir, &presence) {
        Ok(circle) => {
            let _ = app.emit("torii-circle", circle);
            true
        }
        // Panne réseau ou session révoquée : on retentera au prochain battement, sans
        // rien casser côté interface.
        Err(_) => publishing,
    }
}

/// Décide ce qu'on publie. `None` = **ne rien publier du tout**.
///
/// Isolé du reste pour être testable : c'est ici que se joue la promesse faite à
/// l'utilisateur (rien ne part sans son accord, un jeu réduit au silence n'apparaît
/// jamais), et ce genre de garantie doit être vérifiable autrement qu'à l'œil.
fn presence_for(
    prefs: &SocialPrefs,
    game: Option<(String, String, i64)>,
    idle_seconds: u64,
) -> Option<Presence> {
    if !prefs.share_presence {
        return None;
    }
    Some(match game {
        Some((_, title, since)) => Presence {
            status: "in-game".into(),
            game_key: Some(game_key(&title)),
            game_title: Some(title),
            since: Some(since),
        },
        // Une partie en cours prime sur l'inactivité : on peut suivre une cinématique
        // sans toucher au clavier pendant dix minutes.
        None if idle_seconds >= u64::from(prefs.away_after_minutes) * 60 => Presence {
            status: "away".into(),
            ..Default::default()
        },
        None => Presence {
            status: "online".into(),
            ..Default::default()
        },
    })
}

/// Nom de la machine, pour distinguer les sessions dans la liste des appareils.
fn whoami_host() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jeu() -> Option<(String, String, i64)> {
        Some(("steam:292030".into(), "THE WITCHER 3: WILD HUNT™".into(), 1_787_000_000))
    }

    /// Le partage est coupé par défaut, et rien ne doit sortir tant qu'il l'est —
    /// même avec un jeu en cours.
    #[test]
    fn rien_ne_part_sans_accord() {
        let prefs = SocialPrefs::default();
        assert!(!prefs.share_presence, "le partage doit être coupé par défaut");
        assert!(presence_for(&prefs, None, 0).is_none());
        assert!(presence_for(&prefs, jeu(), 0).is_none());
    }

    #[test]
    fn etats_publies() {
        let prefs = SocialPrefs {
            share_presence: true,
            away_after_minutes: 10,
        };

        let p = presence_for(&prefs, jeu(), 0).unwrap();
        assert_eq!(p.status, "in-game");
        assert_eq!(p.game_title.as_deref(), Some("THE WITCHER 3: WILD HUNT™"));
        assert_eq!(p.since, Some(1_787_000_000));

        // Sans jeu : « en ligne », puis « absent » passé le délai d'inactivité.
        assert_eq!(presence_for(&prefs, None, 60).unwrap().status, "online");
        assert_eq!(presence_for(&prefs, None, 599).unwrap().status, "online");
        assert_eq!(presence_for(&prefs, None, 600).unwrap().status, "away");

        // Une partie prime sur l'inactivité (cinématique, jeu au tour par tour…).
        assert_eq!(presence_for(&prefs, jeu(), 9999).unwrap().status, "in-game");
    }

    /// La clé doit être la même quel que soit le launcher d'où vient le titre.
    #[test]
    fn cle_de_jeu_cross_launcher() {
        assert_eq!(
            game_key("THE WITCHER 3: WILD HUNT™"),
            game_key("The Witcher 3: Wild Hunt")
        );
        assert_eq!(game_key("Portal 2"), "title:portal2");
        assert_ne!(game_key("Portal"), game_key("Portal 2"));
    }

    #[test]
    fn prefs_roundtrip() {
        let dir = std::env::temp_dir().join(format!("torii-socialprefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!load_prefs(&dir).share_presence);

        save_prefs(&dir, &SocialPrefs { share_presence: true, away_after_minutes: 3 }).unwrap();
        let back = load_prefs(&dir);
        assert!(back.share_presence);
        assert_eq!(back.away_after_minutes, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
