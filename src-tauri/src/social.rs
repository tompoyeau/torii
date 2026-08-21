//! Client du service social de Torii (comptes, amis, présence).
//!
//! Le serveur vit dans `server/` ; son contrat est décrit dans `server/README.md`.
//!
//! 🔑 Le jeton de session est rangé dans les identifiants chiffrés (`credentials.dat`,
//! DPAPI) comme les jetons des launchers : c'est un secret de longue durée, il n'a rien
//! à faire dans un fichier en clair ni dans le `localStorage` de la WebView.

use crate::accounts::secrets;
use crate::journal;
use crate::platforms::id_set;
use crate::procwatch;
use crate::toast;
use std::collections::HashMap;
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

/// Compte connecté, plus l'indication d'une inscription qui vient d'avoir lieu.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignIn {
    pub account: Account,
    /// Vrai si le compte vient d'être créé : le front propose alors un pseudo.
    pub created: bool,
}

/// Échange le code contre une session et la persiste (chiffrée). Renvoie le compte.
pub fn verify(config_dir: &Path, email: &str, code: &str) -> Result<SignIn, String> {
    #[derive(Deserialize)]
    struct Verified {
        token: String,
        account: Account,
        #[serde(default)]
        created: bool,
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
    Ok(SignIn {
        account: verified.account,
        created: verified.created,
    })
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

/// Ce qu'on laisse voir de soi aux amis.
pub const PRESENCE_OFFLINE: &str = "offline";
pub const PRESENCE_ONLINE: &str = "online";
pub const PRESENCE_DETAILED: &str = "detailed";

/// Réglages de présence, persistés dans `social_prefs.json`.
///
/// 🔑 Le mode par défaut est **hors ligne**. Rien ne quitte la machine tant que
/// l'utilisateur n'a pas choisi autre chose : une présence dit à quelle heure on est
/// devant son PC, ce n'est pas une donnée qu'on diffuse par défaut.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct SocialPrefs {
    /// `offline` | `online` | `detailed`. Absent = déduit de `share_presence`.
    pub presence_mode: Option<String>,
    /// ⚠️ Ancien réglage booléen, gardé pour une seule raison : ne pas rendre invisibles,
    /// sans les prévenir, ceux qui avaient déjà activé le partage. Le front écrit
    /// toujours `presence_mode`.
    pub share_presence: bool,
    /// Minutes d'inactivité avant de passer en « absent ».
    pub away_after_minutes: u32,
    /// Afficher un bandeau quand un ami commence à jouer.
    pub notify_friend_launch: bool,
    /// 🔑 Le rapprochement Steam ↔ Torii n'a lieu qu'UNE fois, et ce drapeau s'en
    /// souvient. Sans lui, « lier par défaut » et « j'ai éteint la visibilité » sont
    /// indiscernables — on rallumerait à chaque démarrage ce que la personne vient
    /// d'éteindre. Un défaut se propose ; il ne se réimpose pas.
    pub steam_auto_linked: bool,
}

impl Default for SocialPrefs {
    fn default() -> Self {
        SocialPrefs {
            presence_mode: None,
            share_presence: false,
            away_after_minutes: 10,
            notify_friend_launch: true,
            steam_auto_linked: false,
        }
    }
}

impl SocialPrefs {
    /// Mode effectif, en tenant compte de l'ancien réglage booléen.
    pub fn mode(&self) -> &str {
        match self.presence_mode.as_deref() {
            Some(PRESENCE_ONLINE) => PRESENCE_ONLINE,
            Some(PRESENCE_DETAILED) => PRESENCE_DETAILED,
            Some(PRESENCE_OFFLINE) => PRESENCE_OFFLINE,
            // Absent ou inconnu : on retombe sur l'ancien booléen.
            _ if self.share_presence => PRESENCE_DETAILED,
            _ => PRESENCE_OFFLINE,
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
        // Qui jouait à quoi au dernier battement : id d'ami → jeu. Sert à distinguer
        // « vient de lancer » de « jouait déjà », la seule chose qui mérite un bandeau.
        let mut parties: HashMap<String, String> = HashMap::new();
        // 🔑 Le premier cercle reçu ne notifie RIEN : au démarrage, tous ceux qui jouent
        // paraîtraient venir de commencer, et Torii cracherait une volée de bandeaux.
        let mut amorce = true;
        loop {
            std::thread::sleep(HEARTBEAT);
            publishing = beat(&app, publishing, &mut parties, &mut amorce);
        }
    });
}

/// Repère les amis qui **viennent** de lancer une partie et affiche un bandeau.
///
/// La comparaison porte sur le couple (ami, jeu) : changer de jeu compte comme un
/// nouveau lancement, rester sur le même n'en est pas un.
fn signaler_lancements(
    app: &tauri::AppHandle,
    config_dir: &Path,
    circle: &Circle,
    parties: &mut HashMap<String, String>,
    amorce: &mut bool,
) {
    let mut courant: HashMap<String, String> = HashMap::new();
    for ami in &circle.friends {
        if ami.status != "in-game" {
            continue;
        }
        let jeu = ami.game_title.clone().unwrap_or_default();
        if jeu.is_empty() {
            continue;
        }
        let nouveau = parties.get(&ami.id) != Some(&jeu);
        if nouveau {
            // Journalisé dans les deux cas : « pourquoi je n'ai pas eu de bandeau ? » est
            // une question qu'on ne peut pas trancher sans trace écrite.
            if *amorce {
                journal::write(
                    config_dir,
                    "INFO",
                    &format!("{} joue déjà à {jeu} au démarrage — pas de bandeau", ami.display_name),
                );
            } else {
                journal::write(
                    config_dir,
                    "INFO",
                    &format!("bandeau : {} lance {jeu}", ami.display_name),
                );
                toast::show(app, &format!("{} joue", ami.display_name), &jeu);
            }
        }
        courant.insert(ami.id.clone(), jeu);
    }
    *parties = courant;
    *amorce = false;
}

/// Un battement. Renvoie `true` si une présence est actuellement publiée — ce qui
/// permet de l'effacer proprement si le partage vient d'être coupé.
fn beat(
    app: &tauri::AppHandle,
    publishing: bool,
    parties: &mut HashMap<String, String>,
    amorce: &mut bool,
) -> bool {
    let Ok(dir) = app.path().app_config_dir() else {
        return publishing;
    };
    // Pas de compte connecté : le service social est simplement inactif.
    if secrets::load(&dir).torii_token.is_none() {
        return false;
    }

    let prefs = load_prefs(&dir);
    if prefs.mode() == PRESENCE_OFFLINE {
        // Partage coupé alors qu'on publiait : on disparaît tout de suite plutôt que
        // d'attendre les 90 s de péremption.
        if publishing {
            let _ = clear_presence(&dir);
        }
        // 🔑 Invisible ≠ aveugle. On continue de LIRE le cercle : sans ça, quelqu'un en
        // mode invisible ne verrait plus ses amis ni leurs lancements — il serait puni
        // d'avoir voulu se cacher, ce que personne n'attend d'un mode « invisible ».
        if let Ok(circle) = circle(&dir) {
            if prefs.notify_friend_launch {
                signaler_lancements(app, &dir, &circle, parties, amorce);
            }
            let _ = app.emit("torii-circle", circle);
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
            if prefs.notify_friend_launch {
                signaler_lancements(app, &dir, &circle, parties, amorce);
            }
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
    let absent = idle_seconds >= u64::from(prefs.away_after_minutes) * 60;
    // Sans jeu : « en ligne », ou « absent » passé le délai d'inactivité.
    let sans_jeu = Presence {
        status: if absent { "away".into() } else { "online".into() },
        ..Default::default()
    };

    match prefs.mode() {
        PRESENCE_OFFLINE => None,
        // « En ligne seulement » : les amis savent que tu es là, jamais à quoi tu joues.
        // On publie donc `online` même en pleine partie — surtout pas `in-game` sans
        // titre, qui reviendrait à annoncer « je joue à quelque chose que je te cache ».
        PRESENCE_ONLINE => Some(sans_jeu),
        _ => Some(match game {
            Some((_, title, since)) => Presence {
                status: "in-game".into(),
                game_key: Some(game_key(&title)),
                game_title: Some(title),
                since: Some(since),
            },
            // Une partie en cours prime sur l'inactivité : on peut suivre une
            // cinématique sans toucher au clavier pendant dix minutes.
            None => sans_jeu,
        }),
    }
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

    fn prefs(mode: &str) -> SocialPrefs {
        SocialPrefs {
            presence_mode: Some(mode.into()),
            away_after_minutes: 10,
            ..Default::default()
        }
    }

    /// Le partage est coupé par défaut, et rien ne doit sortir tant qu'il l'est —
    /// même avec un jeu en cours.
    #[test]
    fn rien_ne_part_sans_accord() {
        let defaut = SocialPrefs::default();
        assert_eq!(defaut.mode(), PRESENCE_OFFLINE, "hors ligne par défaut");
        assert!(presence_for(&defaut, None, 0).is_none());
        assert!(presence_for(&defaut, jeu(), 0).is_none());
        assert!(presence_for(&prefs(PRESENCE_OFFLINE), jeu(), 0).is_none());
    }

    /// « En ligne seulement » ne doit JAMAIS laisser filtrer le jeu, ni même le fait
    /// qu'une partie est en cours.
    #[test]
    fn mode_en_ligne_ne_dit_rien_du_jeu() {
        let p = presence_for(&prefs(PRESENCE_ONLINE), jeu(), 0).unwrap();
        assert_eq!(p.status, "online");
        assert!(p.game_title.is_none() && p.game_key.is_none() && p.since.is_none());
        // L'inactivité reste visible : c'est une information sur toi, pas sur ton jeu.
        assert_eq!(
            presence_for(&prefs(PRESENCE_ONLINE), jeu(), 9999).unwrap().status,
            "away"
        );
    }

    /// Quelqu'un qui avait activé l'ancien réglage booléen ne doit pas se retrouver
    /// invisible après la mise à jour.
    #[test]
    fn ancien_reglage_conserve() {
        let herite = SocialPrefs {
            presence_mode: None,
            share_presence: true,
            ..Default::default()
        };
        assert_eq!(herite.mode(), PRESENCE_DETAILED);
        assert_eq!(presence_for(&herite, jeu(), 0).unwrap().status, "in-game");
    }

    #[test]
    fn etats_publies() {
        let prefs = prefs(PRESENCE_DETAILED);

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

    /// Le repérage des lancements ne doit signaler qu'un vrai changement — c'est la
    /// différence entre une notification utile et une notification par minute.
    #[test]
    fn lancements_reperes() {
        fn ami(id: &str, statut: &str, jeu: Option<&str>) -> Friend {
            Friend {
                id: id.into(),
                display_name: id.into(),
                steam_id: None,
                status: statut.into(),
                game_key: None,
                game_title: jeu.map(str::to_string),
                since: None,
            }
        }
        /// Rejoue un tour et rend les couples (ami, jeu) qui auraient été annoncés.
        fn tour(
            amis: Vec<Friend>,
            parties: &mut HashMap<String, String>,
            amorce: &mut bool,
        ) -> Vec<(String, String)> {
            let mut annonces = Vec::new();
            let mut courant = HashMap::new();
            for a in &amis {
                if a.status != "in-game" {
                    continue;
                }
                let jeu = a.game_title.clone().unwrap_or_default();
                if jeu.is_empty() {
                    continue;
                }
                if !*amorce && parties.get(&a.id) != Some(&jeu) {
                    annonces.push((a.id.clone(), jeu.clone()));
                }
                courant.insert(a.id.clone(), jeu);
            }
            *parties = courant;
            *amorce = false;
            annonces
        }

        let mut parties = HashMap::new();
        let mut amorce = true;

        // Premier tour : quelqu'un joue déjà — on n'annonce rien.
        let t1 = tour(vec![ami("bob", "in-game", Some("Hadès"))], &mut parties, &mut amorce);
        assert!(t1.is_empty(), "le premier cercle ne doit rien annoncer");

        // Il joue toujours au même jeu : toujours rien.
        let t2 = tour(vec![ami("bob", "in-game", Some("Hadès"))], &mut parties, &mut amorce);
        assert!(t2.is_empty(), "une partie qui continue n'est pas un lancement");

        // Un second ami démarre : une seule annonce.
        let t3 = tour(
            vec![
                ami("bob", "in-game", Some("Hadès")),
                ami("alice", "in-game", Some("Portal 2")),
            ],
            &mut parties,
            &mut amorce,
        );
        assert_eq!(t3, vec![("alice".to_string(), "Portal 2".to_string())]);

        // Bob change de jeu : ça compte comme un nouveau lancement.
        let t4 = tour(
            vec![
                ami("bob", "in-game", Some("Celeste")),
                ami("alice", "in-game", Some("Portal 2")),
            ],
            &mut parties,
            &mut amorce,
        );
        assert_eq!(t4, vec![("bob".to_string(), "Celeste".to_string())]);

        // Bob s'arrête puis reprend le même jeu : deux tours, une annonce au retour.
        tour(vec![ami("bob", "online", None)], &mut parties, &mut amorce);
        let t6 = tour(vec![ami("bob", "in-game", Some("Celeste"))], &mut parties, &mut amorce);
        assert_eq!(t6, vec![("bob".to_string(), "Celeste".to_string())]);
    }

    #[test]
    /// Un fichier écrit avant l'arrivée du bandeau n'a pas le champ. Sans `#[serde(default)]`
    /// + un `Default` à `true`, tous les comptes existants se retrouveraient sans
    /// notifications sans l'avoir demandé — silencieusement.
    #[test]
    fn notification_activee_pour_les_anciens_reglages() {
        let ancien = r#"{"sharePresence": true, "awayAfterMinutes": 10}"#;
        let prefs: SocialPrefs = serde_json::from_str(ancien).expect("lecture");
        assert!(prefs.notify_friend_launch);
        assert_eq!(prefs.mode(), PRESENCE_DETAILED);
    }

    #[test]
    fn prefs_roundtrip() {
        let dir = std::env::temp_dir().join(format!("torii-socialprefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load_prefs(&dir).mode(), PRESENCE_OFFLINE);

        save_prefs(
            &dir,
            &SocialPrefs {
                presence_mode: Some(PRESENCE_DETAILED.into()),
                away_after_minutes: 3,
                ..Default::default()
            },
        )
        .unwrap();
        let back = load_prefs(&dir);
        assert_eq!(back.mode(), PRESENCE_DETAILED);
        assert_eq!(back.away_after_minutes, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
