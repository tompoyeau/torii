pub mod accounts;
pub mod metadata;
pub mod models;
pub mod procwatch;
pub mod social;
pub mod platforms;

use models::{GameDto, GameMeta};
use platforms::manual::ManualInput;
use serde::Serialize;
use tauri::{Emitter, Manager};

/// Dernier résultat de `scan_library`, partagé avec les commandes d'enrichissement.
/// 🔑 Un scan n'est PAS une lecture locale : il rejoue toute la séquence réseau des
/// comptes (page communautaire Steam, refresh + produits GOG — qui **fait tourner** le
/// refresh token —, refresh + assets + catalogue Epic). Le refaire pour connaître la
/// liste des jeux doublait le trafic au démarrage et ouvrait une course sur le token GOG.
#[derive(Default)]
struct LastScan(std::sync::Mutex<Vec<GameDto>>);

/// État des connexions de comptes, exposé au frontend (sans secrets).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    steam_connected: bool,
    steam_id: Option<String>,
    gog_connected: bool,
    epic_connected: bool,
    ea_connected: bool,
    battlenet_connected: bool,
}

impl Settings {
    /// Dérive l'état exposé au front à partir des identifiants stockés.
    /// `config_dir` sert à détecter l'état EA (snapshot de bibliothèque sur disque).
    fn from_creds(c: &accounts::secrets::Credentials, config_dir: &std::path::Path) -> Self {
        Settings {
            steam_connected: c.steam_login_secure.is_some()
                || c.steam_community.is_some()
                || c.steam_api_key.is_some(),
            steam_id: c.steam_id.clone(),
            gog_connected: c.gog_refresh_token.is_some(),
            epic_connected: c.epic_refresh_token.is_some(),
            ea_connected: accounts::ea::is_connected(config_dir),
            battlenet_connected: accounts::battlenet::is_connected(config_dir),
        }
    }
}

/// Enregistre (ou efface) la clé API Steam et auto-détecte le SteamID.
#[tauri::command]
async fn set_steam_key(app: tauri::AppHandle, key: String) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut creds = accounts::secrets::load(&dir);
        let key = key.trim();
        creds.steam_api_key = (!key.is_empty()).then(|| key.to_string());
        if creds.steam_api_key.is_some() && creds.steam_id.is_none() {
            creds.steam_id = accounts::steam::detect_steam_id();
        }
        accounts::secrets::save(&dir, &creds)?;
        Ok(Settings::from_creds(&creds, &dir))
    })
    .await
    .map_err(|e| e.to_string())?
}

// ─────────────────────────────────────────────────────────────────────────────
// Connexion aux comptes de launchers
//
// Les cinq flux (Steam, GOG, Epic, EA, Battle.net) partagent le même squelette :
// ouvrir une fenêtre de login → attendre que l'utilisateur se connecte → en extraire
// un secret (cookie, code OAuth, jeton) → fermer la fenêtre → persister. Seules
// l'attente et l'extraction changent, d'où les briques communes ci-dessous.
//
// 🔑 Toutes les opérations sur une WebviewWindow doivent se faire sur le **thread
// principal** (exigence WebView2) ; l'attente, elle, tourne sur un thread dédié pour
// ne pas bloquer le rendu.
// ─────────────────────────────────────────────────────────────────────────────

/// Résultat d'une sonde sur la fenêtre de login.
enum Probe<T> {
    /// Fenêtre fermée (par l'utilisateur ou par nous) : on abandonne.
    Closed,
    /// Rien à récupérer pour l'instant : on repassera.
    Pending,
    Found(T),
}

/// Ouvre une fenêtre de connexion à un launcher.
///
/// `script` est injecté à chaque navigation (les captures de code/jeton passent par le
/// titre du document, que `on_title` reçoit). Les popups sont autorisées : les logins
/// sociaux (Google, Steam, Discord…) passent par `window.open` et une `WebviewWindow`
/// Tauri les ignore par défaut — le clic ne ferait alors rien.
fn open_login_window(
    app: &tauri::AppHandle,
    label: &'static str,
    title: &'static str,
    url: &str,
    size: (f64, f64),
    script: Option<&'static str>,
    on_title: impl Fn(String) + Send + 'static,
) -> Result<(), String> {
    let app = app.clone();
    let url = url.to_string();
    let inner = app.clone();
    inner
        .run_on_main_thread(move || {
            // Même raison que dans `close_login_window` : `destroy()` détruit vraiment,
            // là où `close()` passerait par la règle du tray — soit en tuant l'application,
            // soit en laissant une fenêtre cachée qui bloquerait la réutilisation du label.
            if let Some(win) = app.get_webview_window(label) {
                let _ = win.destroy();
            }
            let Ok(parsed) = url.parse() else { return };
            let mut builder =
                tauri::WebviewWindowBuilder::new(&app, label, tauri::WebviewUrl::External(parsed))
                    .title(title)
                    .inner_size(size.0, size.1)
                    .on_new_window(|_url, _features| tauri::webview::NewWindowResponse::Allow)
                    .on_document_title_changed(move |_win, t| on_title(t));
            if let Some(js) = script {
                builder = builder.initialization_script(js);
            }
            let _ = builder.build();
        })
        .map_err(|e| e.to_string())
}

/// Ferme la fenêtre de login si elle est encore ouverte.
/// Ferme une fenêtre de connexion.
///
/// 🔑 `destroy()` et non `close()` : `close()` **demande** la fermeture, ce qui passe par
/// `CloseRequested` et donc par la règle « fermer = réduire dans le tray » de la fenêtre
/// principale. On ne demande rien ici — on a fini avec cette fenêtre. Deuxième garde-fou,
/// indépendant du filtre sur le label dans `on_window_event`.
fn close_login_window(app: &tauri::AppHandle, label: &'static str) {
    let inner = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = inner.get_webview_window(label) {
            let _ = win.destroy();
        }
    });
}

/// Interroge la fenêtre de login **sur le thread principal** et renvoie ce que `probe`
/// en tire. Un aller-retour trop lent est traité comme « rien encore » (on repassera),
/// jamais comme une fermeture.
fn probe_login_window<T: Send + 'static>(
    app: &tauri::AppHandle,
    label: &'static str,
    probe: impl FnOnce(&tauri::WebviewWindow) -> Option<T> + Send + 'static,
) -> Probe<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    let inner = app.clone();
    let posted = app.run_on_main_thread(move || {
        let result = match inner.get_webview_window(label) {
            None => Probe::Closed,
            Some(win) => match probe(&win) {
                Some(value) => Probe::Found(value),
                None => Probe::Pending,
            },
        };
        let _ = tx.send(result);
    });
    if posted.is_err() {
        return Probe::Closed;
    }
    rx.recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or(Probe::Pending)
}

/// Sonde la fenêtre une fois par seconde jusqu'à obtenir une valeur, la fermeture de
/// la fenêtre, ou l'expiration du délai.
fn poll_login_window<T: Send + 'static>(
    app: &tauri::AppHandle,
    label: &'static str,
    max_secs: u32,
    probe: fn(&tauri::WebviewWindow) -> Option<T>,
) -> Option<T> {
    for _ in 0..max_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        match probe_login_window(app, label, probe) {
            Probe::Closed => return None,
            Probe::Found(value) => return Some(value),
            Probe::Pending => {}
        }
    }
    None
}

/// Attend une valeur captée par le script injecté (remontée via le titre du document),
/// tant que la fenêtre reste ouverte et dans la limite de `max_secs`.
fn wait_for_capture(
    app: &tauri::AppHandle,
    label: &'static str,
    rx: std::sync::mpsc::Receiver<String>,
    max_secs: u32,
) -> Option<String> {
    for _ in 0..max_secs {
        if let Ok(value) = rx.try_recv() {
            return Some(value);
        }
        if matches!(
            probe_login_window(app, label, |_win| Some(())),
            Probe::Closed
        ) {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    None
}

/// Renvoie un canal et la fonction de titre à brancher sur la fenêtre : tout titre
/// commençant par `prefix` est publié dans le canal (c'est ainsi que les scripts
/// injectés font remonter un code ou un jeton).
fn capture_channel(
    prefix: &'static str,
) -> (
    std::sync::mpsc::Receiver<String>,
    impl Fn(String) + Send + 'static,
) {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    (rx, move |title: String| {
        if let Some(value) = title.strip_prefix(prefix) {
            let _ = tx.send(value.to_string());
        }
    })
}

/// État des comptes tel que persisté sur disque.
fn settings_from_disk(dir: &std::path::Path) -> Settings {
    Settings::from_creds(&accounts::secrets::load(dir), dir)
}

/// Efface des identifiants (déconnexion) et renvoie l'état des comptes à jour.
fn forget_credentials(
    app: &tauri::AppHandle,
    edit: impl FnOnce(&mut accounts::secrets::Credentials),
) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    edit(&mut creds);
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

// --- Steam ---------------------------------------------------------------------

const STEAM_LOGIN_LABEL: &str = "steam-login";

/// Lit le couple de cookies de session Steam d'un domaine dans la fenêtre de login.
fn steam_cookie(win: &tauri::WebviewWindow, domain: &str) -> Option<(String, Option<String>)> {
    let url: tauri::Url = domain.parse().ok()?;
    let mut secure = None;
    let mut sessionid = None;
    for c in win.cookies_for_url(url).ok()? {
        match c.name() {
            "steamLoginSecure" => secure = Some(c.value().to_string()),
            "sessionid" => sessionid = Some(c.value().to_string()),
            _ => {}
        }
    }
    secure.map(|s| (s, sessionid))
}

fn steam_store_cookie(win: &tauri::WebviewWindow) -> Option<(String, Option<String>)> {
    steam_cookie(win, "https://store.steampowered.com")
}

fn steam_community_cookie(win: &tauri::WebviewWindow) -> Option<(String, Option<String>)> {
    steam_cookie(win, "https://steamcommunity.com")
}

fn to_cookie_header(secure: String, sessionid: Option<String>) -> String {
    match sessionid {
        Some(sid) => format!("steamLoginSecure={secure}; sessionid={sid}"),
        None => format!("steamLoginSecure={secure}"),
    }
}

fn steam_id_from_cookie(secure: &str) -> Option<String> {
    secure
        .split(|c| c == '|' || c == '%')
        .next()
        .filter(|s| s.len() == 17 && s.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_string)
}

/// Script passif injecté dans la fenêtre de login Steam : intercepte la requête
/// `jwt/finalizelogin` (fetch/XHR) et en extrait le `nonce` (= refresh token
/// Steam, ~200 j), qu'il fait remonter via le titre du document. Toujours
/// transparent (rappelle les fonctions natives) → ne perturbe pas le login.
const STEAM_CAPTURE_JS: &str = r#"
(function () {
  function stash(v) { if (v) { try { document.title = 'ludo-steam-rt:' + v; } catch (e) {} } }
  // Corps de `finalizelogin` : `nonce=<refresh_token>`.
  function fromBody(body) {
    try {
      if (!body) return null;
      if (typeof body === 'string') { var m = /(?:^|&)nonce=([^&]+)/.exec(body); return m ? decodeURIComponent(m[1]) : null; }
      if (typeof body.get === 'function') { return body.get('nonce'); }
    } catch (e) {}
    return null;
  }
  // Réponse de `PollAuthSessionStatus` (ou tout JSON) : champ `refresh_token`.
  function fromText(t) {
    try { var m = /"refresh_token"\s*:\s*"([^"]+)"/.exec(t || ''); return m ? m[1] : null; } catch (e) { return null; }
  }
  function watched(u) {
    return u.indexOf('finalizelogin') !== -1 || u.indexOf('PollAuthSessionStatus') !== -1;
  }
  var of = window.fetch;
  if (of) {
    window.fetch = function (input, init) {
      var u = (typeof input === 'string') ? input : (input && input.url) || '';
      try { if (u.indexOf('finalizelogin') !== -1) { var n = fromBody(init && init.body); if (n) stash(n); } } catch (e) {}
      var p = of.apply(this, arguments);
      try {
        if (watched(u)) {
          p.then(function (r) { r.clone().text().then(function (t) { var rt = fromText(t); if (rt) stash(rt); }).catch(function () {}); }).catch(function () {});
        }
      } catch (e) {}
      return p;
    };
  }
  var oo = XMLHttpRequest.prototype.open, os = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function (m, u) { this.__lu = u; return oo.apply(this, arguments); };
  XMLHttpRequest.prototype.send = function (b) {
    try {
      var u = '' + (this.__lu || '');
      if (u.indexOf('finalizelogin') !== -1) { var n = fromBody(b); if (n) stash(n); }
      if (watched(u)) {
        this.addEventListener('load', function () { try { var rt = fromText(this.responseText); if (rt) stash(rt); } catch (e) {} });
      }
    } catch (e) {}
    return os.apply(this, arguments);
  };
})();
"#;

/// Ouvre la fenêtre de connexion Steam officielle et récupère la session
/// (cookie `steamLoginSecure`) une fois l'utilisateur connecté — sans clé API.
/// Capte aussi le refresh token (~200 j) pour éviter de se reconnecter chaque jour.
/// `async` pour ne pas bloquer le thread principal (rendu de la WebView).
#[tauri::command]
async fn connect_steam(app: tauri::AppHandle) -> Result<Settings, String> {
    let (rt_rx, on_title) = capture_channel("ludo-steam-rt:");

    // Tant qu'aucun refresh token n'est stocké, on force un login FRAIS : Steam n'émet
    // le refresh token (~200 j) que lors d'une vraie saisie identifiants + Steam Guard,
    // jamais sur une session « mémorisée ». Une fois le token capté, les connexions
    // suivantes redeviennent mémorisées (l'auto-refresh gère l'expiration ~24 h).
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let force_fresh = accounts::secrets::load(&dir).steam_refresh_token.is_none();

    open_login_window(
        &app,
        STEAM_LOGIN_LABEL,
        "Connexion Steam",
        "https://store.steampowered.com/login/",
        (500.0, 740.0),
        Some(STEAM_CAPTURE_JS),
        on_title,
    )?;

    // Login frais : on vide les données de navigation (déconnexion Steam garantie) puis
    // on recharge la page de login → Steam redemande les identifiants → refresh token émis.
    if force_fresh {
        let clear_app = app.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let a = clear_app.clone();
            let _ = clear_app.run_on_main_thread(move || {
                if let Some(win) = a.get_webview_window(STEAM_LOGIN_LABEL) {
                    let _ = win.clear_all_browsing_data();
                }
            });
            std::thread::sleep(std::time::Duration::from_millis(900));
            let b = clear_app.clone();
            let _ = clear_app.run_on_main_thread(move || {
                if let Some(win) = b.get_webview_window(STEAM_LOGIN_LABEL) {
                    let _ = win.eval("location.href='https://store.steampowered.com/login/';");
                }
            });
        })
        .await;
    }

    // Sur un thread dédié : on attend le login (cookie store), puis on propage la
    // session vers le domaine communautaire pour en capter aussi le cookie.
    let poll_app = app.clone();
    let captured = tauri::async_runtime::spawn_blocking(move || {
        // Phase 1 : cookie côté store (= login réussi). Jusqu'à ~2 min.
        let (store_secure, store_sid) =
            poll_login_window(&poll_app, STEAM_LOGIN_LABEL, 120, steam_store_cookie)?;
        let steam_id = steam_id_from_cookie(&store_secure);

        // Phase 2 : on navigue la fenêtre vers la communauté (propage la session),
        // puis on lit son cookie. Optionnel : si ça échoue, on gardera le store seul.
        let nav_inner = poll_app.clone();
        let _ = poll_app.run_on_main_thread(move || {
            if let Some(win) = nav_inner.get_webview_window(STEAM_LOGIN_LABEL) {
                let _ = win
                    .eval("window.location.href='https://steamcommunity.com/my/games/?tab=all';");
            }
        });
        let community = poll_login_window(&poll_app, STEAM_LOGIN_LABEL, 25, steam_community_cookie);

        // Le refresh token a été capté pendant le login (finalizelogin) ; on draine
        // le canal (le dernier reçu, au cas où plusieurs titres seraient passés).
        let refresh_token = rt_rx.try_iter().last();

        Some((store_secure, store_sid, steam_id, community, refresh_token))
    })
    .await
    .map_err(|e| e.to_string())?;

    close_login_window(&app, STEAM_LOGIN_LABEL);

    let Some((store_secure, store_sid, steam_id, community, refresh_token)) = captured else {
        return Err("Connexion Steam non détectée (délai dépassé ou fenêtre fermée).".into());
    };
    let steam_id = steam_id.or_else(accounts::steam::detect_steam_id);

    let mut creds = accounts::secrets::load(&dir);
    creds.steam_login_secure = Some(to_cookie_header(store_secure, store_sid));
    creds.steam_community = community.map(|(sec, sid)| to_cookie_header(sec, sid));
    creds.steam_id = steam_id.clone();
    creds.steam_refresh_token = refresh_token;

    // 🔑 Cookie communautaire GARANTI frais, dérivé du refresh token (rejeu de login),
    // indépendant de l'état du WebView2. Sans ça, un profil WebView2 hérité d'une ancienne
    // install peut retourner un vieux `steamLoginSecure` communautaire (HttpOnly, non purgé
    // fiablement par clear_all_browsing_data) → cookie pourri, liste d'amis vide même après
    // reconnexion. On remplace donc le cookie capté par un cookie régénéré (repli : le capté).
    if let (Some(rt), Some(id)) = (creds.steam_refresh_token.as_deref(), creds.steam_id.as_deref())
    {
        if let Some(fresh) = accounts::steam::refresh_web_cookie(rt, id) {
            creds.steam_community = Some(fresh.clone());
            creds.steam_login_secure = Some(fresh);
        }
    }
    accounts::secrets::save(&dir, &creds)?;

    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte Steam (efface session, clé et SteamID).
#[tauri::command]
fn disconnect_steam(app: tauri::AppHandle) -> Result<Settings, String> {
    forget_credentials(&app, |c| {
        c.steam_login_secure = None;
        c.steam_community = None;
        c.steam_api_key = None;
        c.steam_id = None;
        c.steam_refresh_token = None;
    })
}

// --- GOG -----------------------------------------------------------------------

const GOG_LOGIN_LABEL: &str = "gog-login";
const GOG_AUTH_URL: &str = "https://auth.gog.com/auth?client_id=46899977096215655\
    &redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient\
    &response_type=code&layout=client2";

/// GOG ne pousse rien dans le titre : on surveille l'URL de la fenêtre et on récupère
/// le paramètre `code` une fois arrivé sur la page de redirection `on_login_success`.
fn gog_code(win: &tauri::WebviewWindow) -> Option<String> {
    let url = win.url().ok()?;
    if !url.path().contains("on_login_success") {
        return None;
    }
    url.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
}

/// Ouvre la fenêtre de connexion GOG officielle (OAuth) et échange le code
/// obtenu contre un refresh token stocké localement. Pas de clé, pas de mot de
/// passe transmis à Torii. `async` pour ne pas bloquer le rendu de la WebView.
#[tauri::command]
async fn connect_gog(app: tauri::AppHandle) -> Result<Settings, String> {
    open_login_window(
        &app,
        GOG_LOGIN_LABEL,
        "Connexion GOG",
        GOG_AUTH_URL,
        (500.0, 740.0),
        None,
        |_title| {},
    )?;

    let poll_app = app.clone();
    let code = tauri::async_runtime::spawn_blocking(move || {
        poll_login_window(&poll_app, GOG_LOGIN_LABEL, 180, gog_code)
    })
    .await
    .map_err(|e| e.to_string())?;

    close_login_window(&app, GOG_LOGIN_LABEL);

    let Some(code) = code else {
        return Err("Connexion GOG non détectée (délai dépassé ou fenêtre fermée).".into());
    };
    let tokens = accounts::gog::exchange_code(&code).ok_or("Échec de l'échange du code GOG.")?;

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.gog_refresh_token = Some(tokens.refresh_token);
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte GOG (efface le refresh token).
#[tauri::command]
fn disconnect_gog(app: tauri::AppHandle) -> Result<Settings, String> {
    forget_credentials(&app, |c| c.gog_refresh_token = None)
}

// --- Epic ----------------------------------------------------------------------

const EPIC_LOGIN_LABEL: &str = "epic-login";

/// Script injecté dans la fenêtre de login Epic : sur la page de redirection,
/// il extrait le `authorizationCode` (rendu en JSON) et le place dans le titre
/// du document, que Rust capte via `on_document_title_changed`.
const EPIC_CAPTURE_JS: &str = r#"
(function () {
  function grab() {
    try {
      if (location.pathname.indexOf('/id/api/redirect') === -1) return false;
      var t = (document.body && document.body.innerText) || '';
      var m = t.match(/"authorizationCode"\s*:\s*"([0-9a-fA-F]+)"/);
      if (m) { document.title = 'ludo-epic:' + m[1]; return true; }
    } catch (e) {}
    return false;
  }
  if (!grab()) {
    var n = 0, iv = setInterval(function () {
      if (grab() || ++n > 120) clearInterval(iv);
    }, 500);
  }
})();
"#;

/// Ouvre la fenêtre de connexion Epic officielle (OAuth) et capte le code
/// d'autorisation via le titre de la page, puis l'échange contre un refresh token.
#[tauri::command]
async fn connect_epic(app: tauri::AppHandle) -> Result<Settings, String> {
    let (rx, on_title) = capture_channel("ludo-epic:");

    open_login_window(
        &app,
        EPIC_LOGIN_LABEL,
        "Connexion Epic Games",
        &accounts::epic::login_url(),
        (500.0, 740.0),
        Some(EPIC_CAPTURE_JS),
        on_title,
    )?;

    // Attend le code (via le canal) ou la fermeture de la fenêtre, jusqu'à ~3 min.
    let poll_app = app.clone();
    let code = tauri::async_runtime::spawn_blocking(move || {
        wait_for_capture(&poll_app, EPIC_LOGIN_LABEL, rx, 180)
    })
    .await
    .map_err(|e| e.to_string())?;

    close_login_window(&app, EPIC_LOGIN_LABEL);

    let Some(code) = code else {
        return Err("Connexion Epic non détectée (délai dépassé ou fenêtre fermée).".into());
    };
    let tokens = accounts::epic::exchange_code(&code).ok_or("Échec de l'échange du code Epic.")?;

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.epic_refresh_token = Some(tokens.refresh_token);
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte Epic (efface le refresh token).
#[tauri::command]
fn disconnect_epic(app: tauri::AppHandle) -> Result<Settings, String> {
    forget_credentials(&app, |c| c.epic_refresh_token = None)
}

// --- EA ------------------------------------------------------------------------

const EA_LOGIN_LABEL: &str = "ea-login";

/// Injecté dans la fenêtre de login EA. Le formulaire de login est hébergé sur
/// `accounts.ea.com` ; une fois connecté, la fenêtre revient sur `www.ea.com`. On
/// navigue alors vers l'endpoint token (`response_type=token`, `prompt=none`) qui
/// renvoie un JSON `{access_token…}` (les cookies de session sont maintenant posés),
/// et on capte le token via le titre de la page.
const EA_CAPTURE_JS: &str = r#"
(function () {
  var LOGIN_URL = 'https://www.ea.com/login';
  var TOKEN_URL = 'https://accounts.ea.com/connect/auth?client_id=ORIGIN_JS_SDK&response_type=token&redirect_uri=nucleus:rest&prompt=none';
  var acted = false;
  function step() {
    try {
      var body = (document.body && document.body.innerText) || '';
      var m = body.match(/"access_token"\s*:\s*"([^"]+)"/);
      if (m) { document.title = 'ludo-ea:' + m[1]; return true; } // token capté

      var onToken = location.href.indexOf('response_type=token') !== -1;
      // Endpoint token SANS session (login_required) → aller au formulaire de login.
      if (onToken && !acted && body.length > 0 && body.indexOf('access_token') === -1) {
        acted = true; window.location = LOGIN_URL; return true;
      }
      // Revenu sur www.ea.com après connexion (hors /login) → aller chercher le token.
      if (!onToken && !acted && location.hostname === 'www.ea.com' &&
          location.pathname.indexOf('/login') === -1) {
        acted = true; window.location = TOKEN_URL; return true;
      }
    } catch (e) {}
    return false;
  }
  if (!step()) {
    var n = 0, iv = setInterval(function () {
      if (step() || ++n > 360) clearInterval(iv);
    }, 500);
  }
})();
"#;

/// Ouvre la fenêtre de connexion EA, capte l'access token via le titre de page, puis
/// récupère la bibliothèque possédée (API Juno) et la met en cache sur disque.
#[tauri::command]
async fn connect_ea(app: tauri::AppHandle) -> Result<Settings, String> {
    let (rx, on_title) = capture_channel("ludo-ea:");

    // On démarre sur l'endpoint token (prompt=none) : si la session existe déjà
    // (reconnexion) → token direct, sinon le JS redirige vers le login.
    open_login_window(
        &app,
        EA_LOGIN_LABEL,
        "Connexion EA",
        accounts::ea::TOKEN_ENDPOINT,
        (500.0, 760.0),
        Some(EA_CAPTURE_JS),
        on_title,
    )?;

    // Attend le token (via le canal) ou la fermeture de la fenêtre, jusqu'à ~3 min.
    let poll_app = app.clone();
    let token = tauri::async_runtime::spawn_blocking(move || {
        wait_for_capture(&poll_app, EA_LOGIN_LABEL, rx, 180)
    })
    .await
    .map_err(|e| e.to_string())?;

    close_login_window(&app, EA_LOGIN_LABEL);

    let Some(token) = token else {
        return Err("Connexion EA non détectée (délai dépassé ou fenêtre fermée).".into());
    };

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    // La récupération (entitlements + détails) peut prendre quelques secondes → hors thread UI.
    let games = tauri::async_runtime::spawn_blocking(move || accounts::ea::fetch_library(&token))
        .await
        .map_err(|e| e.to_string())?;
    if games.is_empty() {
        return Err("Aucun jeu EA récupéré (token invalide ou API indisponible).".into());
    }
    accounts::ea::save_library(&dir, &games);

    Ok(settings_from_disk(&dir))
}

/// Déconnecte EA : supprime le snapshot en cache ET efface la session web EA (ouvre
/// brièvement la page de logout, auto-fermée) pour autoriser un vrai changement de compte.
#[tauri::command]
async fn disconnect_ea(app: tauri::AppHandle) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    accounts::ea::disconnect(&dir);

    // Page de déconnexion utilisateur EA (efface la session sans paramètre OAuth ;
    // `connect/logout` exigeait un redirect_uri enregistré → erreur 102111).
    let _ = open_login_window(
        &app,
        EA_LOGIN_LABEL,
        "Déconnexion EA",
        "https://www.ea.com/logout",
        (440.0, 320.0),
        None,
        |_title| {},
    );

    // Laisse le logout s'exécuter, puis referme la fenêtre.
    let close_app = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        close_login_window(&close_app, EA_LOGIN_LABEL);
    })
    .await;

    Ok(settings_from_disk(&dir))
}

// --- Battle.net ----------------------------------------------------------------

const BNET_LOGIN_LABEL: &str = "battlenet-login";

/// Assemble **tous** les cookies de `account.battle.net` en header `Cookie`.
/// Chaîne vide tant qu'aucun cookie n'est encore posé.
fn bnet_cookie_header(win: &tauri::WebviewWindow) -> Option<String> {
    let url: tauri::Url = "https://account.battle.net".parse().ok()?;
    let header = win
        .cookies_for_url(url)
        .map(|cookies| {
            cookies
                .iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();
    Some(header)
}

/// Ouvre la fenêtre de connexion Battle.net ; une fois connecté, lit les cookies de
/// session et récupère la bibliothèque possédée (`games-and-subs`), mise en cache disque.
#[tauri::command]
async fn connect_battlenet(app: tauri::AppHandle) -> Result<Settings, String> {
    open_login_window(
        &app,
        BNET_LOGIN_LABEL,
        "Connexion Battle.net",
        "https://account.battle.net/",
        (520.0, 760.0),
        None,
        |_title| {},
    )?;

    // Sonde les cookies puis tente l'API jusqu'à obtenir des jeux (= connecté), ~3 min.
    // L'appel réseau reste sur ce thread : seule la lecture des cookies passe par le
    // thread principal.
    let poll_app = app.clone();
    let games = tauri::async_runtime::spawn_blocking(move || {
        for _ in 0..90 {
            match probe_login_window(&poll_app, BNET_LOGIN_LABEL, bnet_cookie_header) {
                Probe::Closed => return None, // fenêtre fermée
                Probe::Found(header) if !header.is_empty() => {
                    let games = accounts::battlenet::fetch_library(&header);
                    if !games.is_empty() {
                        return Some(games);
                    }
                }
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        None
    })
    .await
    .map_err(|e| e.to_string())?;

    close_login_window(&app, BNET_LOGIN_LABEL);

    let Some(games) = games else {
        return Err("Connexion Battle.net non détectée (délai dépassé ou fenêtre fermée).".into());
    };

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    accounts::battlenet::save_library(&dir, &games);
    Ok(settings_from_disk(&dir))
}

/// Déconnecte Battle.net (supprime le snapshot en cache).
#[tauri::command]
fn disconnect_battlenet(app: tauri::AppHandle) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    accounts::battlenet::disconnect(&dir);
    Ok(settings_from_disk(&dir))
}

/// Métadonnée IGDB résolue pour un jeu (renvoyée au front pour fusion réactive).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MetaUpdate {
    id: String,
    genre: Option<String>,
    description: Option<String>,
    cover_url: Option<String>,
    hero_url: Option<String>,
    developer: Option<String>,
    year: Option<i64>,
    screenshots: Vec<String>,
}

impl MetaUpdate {
    fn new(id: String, m: metadata::igdb::IgdbMeta) -> Self {
        MetaUpdate {
            id,
            genre: m.genre,
            description: m.description,
            cover_url: m.cover_url,
            hero_url: m.hero_url,
            developer: m.developer,
            year: m.year,
            screenshots: m.screenshots,
        }
    }
}

/// Remplit la métadonnée descriptive de toute la bibliothèque via IGDB (proxy) :
/// genre, description, captures, jaquette (repli), hero, studio, année. Match exact
/// par appid Steam en masse + par nom pour les autres launchers. Les résultats
/// arrivent par lots (événement `igdb-batch`) pour un affichage progressif ; la
/// valeur de retour est l'ensemble complet. Cache disque (1er remplissage seulement).
#[tauri::command]
async fn enrich_igdb(app: tauri::AppHandle) -> Result<Vec<MetaUpdate>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    // Réutilise le scan que `scan_library` vient de faire : re-scanner ici rejouerait
    // toute la séquence réseau des comptes (cf. LastScan). Repli sur un scan si l'état
    // est vide (commande appelée sans scan préalable).
    let scanned = app
        .state::<LastScan>()
        .0
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let emitter = app.clone();
    let updates = tauri::async_runtime::spawn_blocking(move || {
        let games = if scanned.is_empty() {
            platforms::scan_all(Some(&dir))
        } else {
            scanned
        };
        metadata::igdb::fill_metadata(&games, &dir, |batch| {
            let ups: Vec<MetaUpdate> = batch
                .iter()
                .map(|(id, m)| MetaUpdate::new(id.clone(), m.clone()))
                .collect();
            let _ = emitter.emit("igdb-batch", ups);
        })
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(updates
        .into_iter()
        .map(|(id, m)| MetaUpdate::new(id, m))
        .collect())
}

/// Revendeurs masqués par l'utilisateur (vide si le dossier de config est illisible).
fn excluded_stores(app: &tauri::AppHandle) -> std::collections::HashSet<String> {
    app.path()
        .app_config_dir()
        .map(|dir| platforms::id_set::EXCLUDED_STORES.load(&dir))
        .unwrap_or_default()
}

/// Boutique — vitrine : une page de jeux mis en avant / en promo (ITAD), selon le tri
/// (`featured`, `savings`, `price`, `recent`, `rating`). Découverte pure : indépendant
/// de la bibliothèque de l'utilisateur. Les revendeurs masqués sont écartés du choix
/// de la meilleure offre.
#[tauri::command]
async fn store_deals(
    app: tauri::AppHandle,
    page: u32,
    sort: String,
) -> Result<Vec<metadata::store::StoreItem>, String> {
    let excluded = excluded_stores(&app);
    tauri::async_runtime::spawn_blocking(move || metadata::store::deals(page, &sort, &excluded))
        .await
        .map_err(|e| e.to_string())
}

/// Boutique — recherche de jeux par titre (renvoie le prix le plus bas de chacun,
/// hors revendeurs masqués).
#[tauri::command]
async fn store_search(
    app: tauri::AppHandle,
    query: String,
) -> Result<Vec<metadata::store::StoreItem>, String> {
    let excluded = excluded_stores(&app);
    tauri::async_runtime::spawn_blocking(move || metadata::store::search(&query, &excluded))
        .await
        .map_err(|e| e.to_string())
}

/// Boutique — suggestions d'autocomplétion (léger, sans prix) au fil de la frappe.
#[tauri::command]
async fn store_suggest(query: String) -> Result<Vec<metadata::store::Suggestion>, String> {
    tauri::async_runtime::spawn_blocking(move || metadata::store::suggest(&query))
        .await
        .map_err(|e| e.to_string())
}

/// Boutique — fiche produit : comparatif de prix multi-boutiques + enrichissement
/// descriptif IGDB (visuels, description, genre, captures).
#[tauri::command]
async fn store_game(
    app: tauri::AppHandle,
    game_id: String,
) -> Result<Option<metadata::store::StoreGame>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || metadata::store::game(&game_id, &dir))
        .await
        .map_err(|e| e.to_string())
}

/// Liste d'amis Steam + présence (en ligne / en jeu), via la session stockée.
/// Renvoie une liste vide si Steam n'est pas connecté.
#[tauri::command]
async fn steam_friends(app: tauri::AppHandle) -> Result<Vec<accounts::steam::Friend>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || accounts::steam_friends_list(&dir))
        .await
        .map_err(|e| e.to_string())
}

/// Profil Steam de l'utilisateur connecté (pseudo + avatar), pour l'en-tête.
/// `None` si Steam n'est pas connecté.
#[tauri::command]
async fn steam_me(app: tauri::AppHandle) -> Result<Option<accounts::steam::SteamProfile>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || accounts::steam_me(&dir))
        .await
        .map_err(|e| e.to_string())
}

/// Succès Steam d'un jeu (`appid`) pour l'utilisateur connecté : nom, description, icône,
/// date de déblocage par succès + total (succès cachés inclus). `None` si Steam non
/// connecté, jeu sans succès, ou page indisponible.
#[tauri::command]
async fn steam_achievements(
    app: tauri::AppHandle,
    appid: u64,
) -> Result<Option<accounts::steam::GameAchievements>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || accounts::steam_achievements(&dir, appid))
        .await
        .map_err(|e| e.to_string())
}

/// Nombre de joueurs en ce moment sur un jeu Steam (`appid`), via l'API publique
/// (sans clé, sans session). `None` si indisponible.
#[tauri::command]
async fn steam_current_players(appid: u64) -> Result<Option<u32>, String> {
    tauri::async_runtime::spawn_blocking(move || accounts::steam::current_players(appid))
        .await
        .map_err(|e| e.to_string())
}

/// Wishlist **unifiée** : wishlist Steam native ∪ wishlist Torii (universelle, tout jeu),
/// dédupliquée et enrichie de prix (ITAD). Les entrées Torius déjà présentes sur Steam
/// (même appid) ne sont pas doublées.
#[tauri::command]
async fn wishlist_all(app: tauri::AppHandle) -> Result<Vec<metadata::store::WishlistItem>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let excluded = excluded_stores(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let steam_appids = accounts::steam_wishlist_appids(&dir);
        let steam_set: std::collections::HashSet<u64> = steam_appids.iter().copied().collect();
        let mut items = metadata::store::wishlist(&steam_appids, &excluded);
        let extra: Vec<(String, u64, String, Option<String>)> = platforms::wishlist::load(&dir)
            .into_iter()
            .filter(|e| e.steam_appid.map_or(true, |a| !steam_set.contains(&a)))
            .map(|e| (e.id, e.steam_appid.unwrap_or(0), e.title, e.cover_url))
            .collect();
        items.extend(metadata::store::wishlist_custom(&extra, &excluded));
        items
    })
    .await
    .map_err(|e| e.to_string())
}

/// Ids de la wishlist Torii (pour refléter l'état des boutons « ♥ » côté Boutique). Local, rapide.
#[tauri::command]
fn wishlist_ids(app: tauri::AppHandle) -> Vec<String> {
    let dir = match app.path().app_config_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    platforms::wishlist::load(&dir).into_iter().map(|e| e.id).collect()
}

/// Ajoute un jeu à la wishlist Torii (universelle). L'appid Steam est résolu automatiquement
/// (via ITAD) : si le jeu existe sur Steam, il est **en bonus** ajouté à la vraie wishlist
/// Steam. Renvoie `true` si le push Steam a réussi.
#[tauri::command]
async fn wishlist_add(
    app: tauri::AppHandle,
    id: String,
    title: String,
    cover_url: Option<String>,
) -> Result<bool, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let steam_appid = metadata::store::steam_appid_for(&id);
        let _ = platforms::wishlist::add(
            &dir,
            platforms::wishlist::WishEntry { id, steam_appid, title, cover_url },
        );
        match steam_appid {
            Some(a) => accounts::steam_set_wishlist(&dir, a, true),
            None => false,
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Retire un jeu de la wishlist Torii (et de Steam si le jeu y était, via l'appid mémorisé).
#[tauri::command]
async fn wishlist_remove(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let appid = platforms::wishlist::load(&dir)
            .into_iter()
            .find(|e| e.id == id)
            .and_then(|e| e.steam_appid);
        let _ = platforms::wishlist::remove(&dir, &id);
        if let Some(a) = appid {
            accounts::steam_set_wishlist(&dir, a, false);
        }
    })
    .await
    .map_err(|e| e.to_string())
}

/// Jeux en commun avec les amis Steam : ma bibliothèque croisée avec celle de chaque ami
/// (dont le profil est lisible). `force` recalcule en ignorant le cache disque. Renvoie une
/// charge vide si Steam n'est pas connecté.
#[tauri::command]
async fn friends_common(
    app: tauri::AppHandle,
    force: bool,
) -> Result<accounts::friends_games::FriendsCommon, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        accounts::friends_games::compute(&dir, force).unwrap_or_default()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Renvoie l'état des connexions de comptes.
#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Settings {
    let dir = app.path().app_config_dir().ok();
    let creds = dir
        .as_ref()
        .map(|d| accounts::secrets::load(d))
        .unwrap_or_default();
    let path = dir.as_deref().unwrap_or_else(|| std::path::Path::new(""));
    Settings::from_creds(&creds, path)
}

/// Affiche une notification système (utilisée pour les baisses de prix de la wishlist).
#[tauri::command]
fn notify_user(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

/// Vide les caches de métadonnées/jaquettes/prix (fichiers `*cache*.json` de l'app),
/// sans toucher aux identifiants, favoris, masqués, wishlist ni snapshots de bibliothèque.
/// Renvoie le nombre de fichiers supprimés.
#[tauri::command]
fn clear_caches(app: tauri::AppHandle) -> Result<u32, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut removed = 0u32;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("cache") && name.ends_with(".json") && std::fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

/// Indique si Torii est réglé pour démarrer automatiquement avec Windows.
#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// Active ou désactive le démarrage automatique de Torii avec Windows (clé de registre
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). Renvoie l'état effectif après coup.
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    let result = if enabled { manager.enable() } else { manager.disable() };
    result.map_err(|e| format!("Impossible de modifier le démarrage automatique : {e}"))?;
    Ok(manager.is_enabled().unwrap_or(enabled))
}

/// Agrège toutes les bibliothèques détectées (Steam, Epic, GOG, manuel).
/// ⚠️ `async` + `spawn_blocking` obligatoires : une commande synchrone s'exécute sur le
/// **thread principal** (cf. `body_blocking` de `tauri-macros`), donc tout le scan
/// (registre + réseau des comptes) y bloquerait la boucle d'événements — fenêtre figée,
/// tray inerte. Même piège que `connect_steam`.
#[tauri::command]
async fn scan_library(app: tauri::AppHandle) -> Vec<GameDto> {
    let config_dir = app.path().app_config_dir().ok();
    let games = tauri::async_runtime::spawn_blocking(move || {
        let games = platforms::scan_all(config_dir.as_deref());
        // Mémorisé sur disque : le prochain démarrage affiche cette liste tout de
        // suite, sans attendre le réseau des comptes (cf. `cached_library`).
        if let Some(dir) = config_dir.as_deref() {
            platforms::library_cache::save(dir, &games);
        }
        games
    })
    .await
    .unwrap_or_default();
    // Mémorisé pour `enrich_igdb` (évite un second scan complet, cf. LastScan).
    if let Ok(mut slot) = app.state::<LastScan>().0.lock() {
        *slot = games.clone();
    }
    // Un jeu installé depuis le dernier scan devient détectable immédiatement.
    procwatch::set_targets(&app, &games);
    games
}

/// Bibliothèque du dernier scan, relue du disque : instantanée, sans aucun accès
/// réseau. Sert à peupler l'écran dès le lancement pendant que `scan_library`
/// travaille en arrière-plan. Vide au tout premier lancement.
#[tauri::command]
async fn cached_library(app: tauri::AppHandle) -> Vec<GameDto> {
    let Ok(dir) = app.path().app_config_dir() else {
        return Vec::new();
    };
    tauri::async_runtime::spawn_blocking(move || platforms::library_cache::load(&dir))
        .await
        .unwrap_or_default()
}

/// Enrichit **un seul** jeu à la demande (à l'ouverture de sa vue détail) :
/// description, captures, développeur, année, genre. Résultat mis en cache disque.
#[tauri::command]
async fn enrich_game(
    app: tauri::AppHandle,
    id: String,
    platform: String,
    launch_target: String,
    title: String,
    installed: bool,
) -> Result<GameMeta, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let game = GameDto {
        id,
        platform,
        launch_target,
        title,
        installed,
        ..Default::default()
    };
    // Jusqu'à deux appels réseau (fiche + taille steamcmd) : hors thread principal.
    tauri::async_runtime::spawn_blocking(move || metadata::enrich_one(&game, &dir))
        .await
        .map_err(|e| e.to_string())
}

/// Masque ou réaffiche un jeu (liste d'exclusion) ; renvoie la liste des ids masqués.
#[tauri::command]
fn set_game_hidden(
    app: tauri::AppHandle,
    id: String,
    hidden: bool,
) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::id_set::HIDDEN.set(&dir, &id, hidden)
}

/// Épingle ou retire un jeu des favoris ; renvoie la liste des ids favoris à jour.
#[tauri::command]
fn set_game_favorite(
    app: tauri::AppHandle,
    id: String,
    favorite: bool,
) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::id_set::FAVORITES.set(&dir, &id, favorite)
}

/// Renvoie la liste des boutiques masquées par l'utilisateur (revendeurs exclus).
#[tauri::command]
fn get_excluded_stores(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut list: Vec<String> = platforms::id_set::EXCLUDED_STORES.load(&dir).into_iter().collect();
    list.sort();
    Ok(list)
}

/// Masque ou réaffiche une boutique (revendeur) ; renvoie la liste des exclus à jour.
#[tauri::command]
fn set_store_excluded(
    app: tauri::AppHandle,
    name: String,
    excluded: bool,
) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::id_set::EXCLUDED_STORES.set(&dir, &name, excluded)
}

/// Réaffiche toutes les boutiques (vide la liste d'exclusion) ; renvoie la liste vide.
#[tauri::command]
fn clear_excluded_stores(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::id_set::EXCLUDED_STORES.clear(&dir)
}

/// Lance un jeu selon sa plateforme et sa cible.
#[tauri::command]
fn launch_game(platform: String, target: String) -> Result<(), String> {
    platforms::launch(&platform, &target)
}

/// Déclenche l'installation d'un jeu possédé non installé (ouvre le launcher sur l'install).
#[tauri::command]
fn install_game(app: tauri::AppHandle, platform: String, target: String) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::install(&platform, &target, &dir)
}

/// Enregistre « maintenant » comme dernière session du jeu (déclenché au clic sur Jouer).
/// Fournit une date de dernière session pour les jeux sans stats de launcher. Renvoie
/// l'horodatage Unix posé (pour la mise à jour optimiste du front).
#[tauri::command]
fn record_launch(app: tauri::AppHandle, id: String) -> Result<i64, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::playhistory::record(&dir, &id)
}

/// Ouvre le dossier d'installation d'un jeu dans l'explorateur de fichiers.
/// Si le chemin pointe sur un fichier, ouvre son dossier parent.
#[tauri::command]
fn open_install_dir(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let target = if p.is_file() {
        p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or(path.clone())
    } else {
        path.clone()
    };
    if !std::path::Path::new(&target).exists() {
        return Err(format!("Dossier introuvable : {target}"));
    }
    tauri_plugin_opener::open_path(&target, None::<&str>)
        .map_err(|e| format!("Impossible d'ouvrir le dossier : {e}"))
}

/// Déclenche la désinstallation d'un jeu installé (délègue à l'UI native du launcher).
#[tauri::command]
fn uninstall_game(
    platform: String,
    target: String,
    install_dir: Option<String>,
) -> Result<(), String> {
    platforms::uninstall(&platform, &target, install_dir.as_deref())
}

/// Ajoute un jeu manuel et renvoie la liste des jeux manuels à jour.
#[tauri::command]
fn add_manual_game(app: tauri::AppHandle, input: ManualInput) -> Result<Vec<GameDto>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::manual::add(&dir, input)
}

/// Met à jour un jeu manuel existant (édition depuis sa fiche).
#[tauri::command]
fn update_manual_game(
    app: tauri::AppHandle,
    id: String,
    input: ManualInput,
) -> Result<Vec<GameDto>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::manual::update(&dir, &id, input)
}

/// Retire un jeu manuel par son id.
#[tauri::command]
fn remove_manual_game(app: tauri::AppHandle, id: String) -> Result<Vec<GameDto>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::manual::remove(&dir, &id)
}

/// Option « revenir à la fermeture du jeu » : Torii se réduit, et la fiche du jeu sera
/// rouverte quand la partie se termine.
///
/// La détection du process n'est plus faite ici : `procwatch` surveille déjà TOUS les
/// jeux en continu, et bien moins cher. Cette commande ne fait donc que désigner le jeu
/// dont la fermeture doit ramener la fenêtre au premier plan.
#[tauri::command]
fn start_game_watch(app: tauri::AppHandle, game_id: String, install_dir: String) {
    if !std::path::Path::new(&install_dir).is_dir() {
        return;
    }
    procwatch::arm(&app, game_id);
    // Torii se réduit pendant la partie.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.minimize();
    }
}

/* ── Service social : comptes, amis, présence ──────────────────────────────── */

/// Dossier de config — facteur commun de toutes les commandes sociales.
fn social_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// Exécute hors du thread principal : chacune de ces commandes fait un aller-retour
/// réseau, qui gèlerait l'interface si elle tournait là où elle est appelée.
macro_rules! offload {
    ($body:expr) => {
        tauri::async_runtime::spawn_blocking(move || $body)
            .await
            .map_err(|e| e.to_string())?
    };
}

/// Demande un code de connexion. Renvoie le code lui-même si le serveur est en mode
/// développement (aucun e-mail ne partira alors), sinon `null`.
#[tauri::command]
async fn torii_request_code(email: String) -> Result<Option<String>, String> {
    Ok(offload!(social::request_code(&email))?)
}

/// Vérifie le code et ouvre une session (persistée chiffrée).
#[tauri::command]
async fn torii_verify(
    app: tauri::AppHandle,
    email: String,
    code: String,
) -> Result<social::Account, String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::verify(&dir, &email, &code))?)
}

/// Compte connecté, ou `null` si aucune session valide.
#[tauri::command]
async fn torii_me(app: tauri::AppHandle) -> Result<Option<social::Account>, String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::me(&dir)))
}

#[tauri::command]
async fn torii_logout(app: tauri::AppHandle) -> Result<(), String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::logout(&dir))?)
}

#[tauri::command]
async fn torii_set_profile(
    app: tauri::AppHandle,
    display_name: Option<String>,
    steam_id: Option<String>,
    steam_discoverable: Option<bool>,
) -> Result<social::Account, String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::set_profile(
        &dir,
        display_name,
        steam_id,
        steam_discoverable
    ))?)
}

/// Amis, demandes reçues et demandes envoyées.
#[tauri::command]
async fn torii_circle(app: tauri::AppHandle) -> Result<social::Circle, String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::circle(&dir))?)
}

#[tauri::command]
async fn torii_invite(app: tauri::AppHandle, friend_code: String) -> Result<(), String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::invite(&dir, &friend_code))?)
}

/// Invite une personne trouvée par suggestion (identifiant plutôt que code d'ami).
#[tauri::command]
async fn torii_invite_account(app: tauri::AppHandle, account_id: String) -> Result<(), String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::invite_account(&dir, &account_id))?)
}

#[tauri::command]
async fn torii_respond(
    app: tauri::AppHandle,
    account_id: String,
    accept: bool,
) -> Result<(), String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::respond(&dir, &account_id, accept))?)
}

#[tauri::command]
async fn torii_remove_friend(app: tauri::AppHandle, account_id: String) -> Result<(), String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::remove_friend(&dir, &account_id))?)
}

/// Régénère son code d'ami : l'ancien cesse aussitôt de fonctionner.
#[tauri::command]
async fn torii_rotate_code(app: tauri::AppHandle) -> Result<String, String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::rotate_code(&dir))?)
}

/// Amis Steam déjà sur Torii (les deux comptes doivent être découvrables).
#[tauri::command]
async fn torii_suggestions(
    app: tauri::AppHandle,
    steam_ids: Vec<String>,
) -> Result<Vec<social::Person>, String> {
    let dir = social_dir(&app)?;
    Ok(offload!(social::suggestions(&dir, &steam_ids))?)
}

/// Réglages de partage de présence (partage coupé par défaut).
#[tauri::command]
fn torii_prefs(app: tauri::AppHandle) -> Result<social::SocialPrefs, String> {
    Ok(social::load_prefs(&social_dir(&app)?))
}

#[tauri::command]
fn torii_set_prefs(
    app: tauri::AppHandle,
    prefs: social::SocialPrefs,
) -> Result<social::SocialPrefs, String> {
    let dir = social_dir(&app)?;
    social::save_prefs(&dir, &prefs)?;
    Ok(prefs)
}

/// Jeux qu'on ne diffuse jamais aux amis (applications permanentes, jeux privés).
#[tauri::command]
fn torii_muted_games(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let dir = social_dir(&app)?;
    let mut list: Vec<String> = platforms::id_set::PRESENCE_MUTED.load(&dir).into_iter().collect();
    list.sort();
    Ok(list)
}

#[tauri::command]
fn torii_mute_game(app: tauri::AppHandle, id: String, muted: bool) -> Result<Vec<String>, String> {
    let dir = social_dir(&app)?;
    platforms::id_set::PRESENCE_MUTED.set(&dir, &id, muted)
}

/// Préférences liées à la fenêtre, lues côté Rust (au démarrage et à la fermeture)
/// donc persistées dans un fichier plutôt qu'en localStorage.
#[derive(Serialize, serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase", default)]
struct WindowPrefs {
    /// Démarrer réduit dans la zone de notification (fenêtre cachée au lancement).
    start_minimized: bool,
    /// Fermer la fenêtre la réduit dans le tray au lieu de quitter l'application.
    close_to_tray: bool,
}

impl Default for WindowPrefs {
    /// 🔑 `close_to_tray` est **vrai** par défaut : Torii doit continuer à détecter les
    /// parties une fois la fenêtre fermée, sinon « Récemment joué » et la présence
    /// s'arrêtent dès qu'on range la fenêtre — ce que personne n'associe à une fermeture.
    /// Qui veut vraiment quitter décoche la case, ou passe par « Quitter » dans le tray.
    fn default() -> Self {
        WindowPrefs {
            start_minimized: false,
            close_to_tray: true,
        }
    }
}

fn load_window_prefs(app: &tauri::AppHandle) -> WindowPrefs {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("window_prefs.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
fn get_window_prefs(app: tauri::AppHandle) -> WindowPrefs {
    load_window_prefs(&app)
}

#[tauri::command]
fn set_window_prefs(app: tauri::AppHandle, start_minimized: bool, close_to_tray: bool) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).ok();
    let prefs = WindowPrefs { start_minimized, close_to_tray };
    let json = serde_json::to_string(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("window_prefs.json"), json).map_err(|e| e.to_string())
}

/// Construit l'icône de la zone de notification (tray) avec son menu.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Ouvrir Torii", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("Torii")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => reveal_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

/// Affiche et met au premier plan la fenêtre principale (depuis le tray ou une 2e instance).
/// 🔑 Sur Windows, un process en arrière-plan ne peut pas voler le focus : on force le
/// passage au premier plan via un aller-retour `always_on_top`.
fn reveal_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        let _ = win.set_always_on_top(false);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(LastScan::default())
        .manage(procwatch::Watch::default())
        // ⚠️ DOIT être enregistré en premier. Empêche une 2e instance : quand l'app
        // tourne déjà (ex. réduite dans le tray via « fermer dans la zone de notification »)
        // et qu'on la relance, la nouvelle instance se ferme et la fenêtre existante est
        // ramenée au premier plan (au lieu d'ouvrir un doublon qui bloque l'ouverture).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // La 2e instance déclenche ce callback dans l'instance déjà en cours : on révèle
            // la fenêtre sur le **thread principal** (les opérations fenêtre depuis le thread
            // du callback échouent sinon — symptôme : « relancer ne rouvre pas, il faut le tray »).
            let handle = app.clone();
            let inner = handle.clone();
            let _ = handle.run_on_main_thread(move || reveal_window(&inner));
        }))
        .plugin(tauri_plugin_opener::init())
        // Sélecteurs de fichiers natifs (exécutable + jaquette d'un jeu manuel).
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        // Démarrage automatique avec Windows (clé HKCU\…\Run). L'état est piloté par le
        // toggle des Réglages et, si l'utilisateur l'a choisi, préréglé par l'installeur.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // Icône dans la zone de notification (tray).
            build_tray(app)?;
            // Détection des parties (y compris lancées hors de Torii) : un seul fil,
            // qui dort la plupart du temps. Voir `procwatch` pour le coût mesuré.
            procwatch::spawn(app.handle().clone());
            // Présence : ne publie rien tant qu'aucun compte n'est connecté ET que le
            // partage n'a pas été activé explicitement (cf. `social::SocialPrefs`).
            social::spawn_heartbeat(app.handle().clone());
            // La fenêtre est créée cachée (`visible:false`) pour éviter tout flash au
            // démarrage : on l'affiche seulement si « Démarrer minimisé » n'est PAS actif.
            // (Sinon elle reste dans le tray, sans clignotement.)
            if !load_window_prefs(&app.handle().clone()).start_minimized {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 🔑 Ne concerne QUE la fenêtre principale.
            //
            // Sans ce garde-fou, la règle ci-dessous s'appliquait aussi aux fenêtres de
            // connexion (`steam-login`, `gog-login`, `epic-login`) : à la fin d'un login,
            // `connect_steam` ferme sa fenêtre, ce qui déclenchait `CloseRequested` et
            // donc `exit(0)` — TOUTE l'application se fermait juste après la connexion.
            // Invisible pour qui a activé « fermer dans la zone de notification » (la
            // fenêtre était alors seulement cachée), fatal pour tous les autres, c'est-à-dire
            // pour chaque nouvelle installation. Signalé en production.
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if load_window_prefs(&window.app_handle().clone()).close_to_tray {
                    // « Fermer = réduire dans le tray » : on cache au lieu de quitter.
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    // Sinon on QUITTE vraiment. Sans ça, l'icône du tray garde le process
                    // vivant après destruction de la fenêtre → instance fantôme sans fenêtre
                    // qui bloque les relances (single-instance ne trouve plus de fenêtre à
                    // révéler). On force donc la sortie complète.
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            scan_library,
            cached_library,
            enrich_game,
            set_game_hidden,
            set_game_favorite,
            get_excluded_stores,
            set_store_excluded,
            clear_excluded_stores,
            launch_game,
            install_game,
            record_launch,
            uninstall_game,
            open_install_dir,
            add_manual_game,
            update_manual_game,
            remove_manual_game,
            connect_steam,
            disconnect_steam,
            connect_gog,
            disconnect_gog,
            connect_epic,
            disconnect_epic,
            connect_ea,
            disconnect_ea,
            connect_battlenet,
            disconnect_battlenet,
            enrich_igdb,
            store_deals,
            store_search,
            store_suggest,
            store_game,
            steam_friends,
            steam_me,
            steam_achievements,
            steam_current_players,
            wishlist_all,
            wishlist_ids,
            wishlist_add,
            wishlist_remove,
            friends_common,
            set_steam_key,
            get_settings,
            get_autostart,
            set_autostart,
            clear_caches,
            get_window_prefs,
            set_window_prefs,
            start_game_watch,
            torii_request_code,
            torii_verify,
            torii_me,
            torii_logout,
            torii_set_profile,
            torii_circle,
            torii_invite,
            torii_invite_account,
            torii_respond,
            torii_remove_friend,
            torii_rotate_code,
            torii_suggestions,
            torii_prefs,
            torii_set_prefs,
            torii_muted_games,
            torii_mute_game,
            notify_user
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
