pub mod accounts;
pub mod metadata;
pub mod models;
pub mod platforms;

use models::{GameDto, GameMeta};
use platforms::manual::ManualInput;
use serde::Serialize;
use tauri::{Emitter, Manager};

/// Progression de l'enrichissement, émise vers le frontend.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnrichProgress {
    done: usize,
    total: usize,
}

/// Émis quand un jeu suivi se ferme (→ le front ouvre sa fiche).
#[derive(Clone, Serialize)]
struct GameExited {
    id: String,
}

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
fn set_steam_key(app: tauri::AppHandle, key: String) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    let key = key.trim();
    creds.steam_api_key = (!key.is_empty()).then(|| key.to_string());
    if creds.steam_api_key.is_some() && creds.steam_id.is_none() {
        creds.steam_id = accounts::steam::detect_steam_id();
    }
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

const STEAM_LOGIN_LABEL: &str = "steam-login";

enum CookieRead {
    Closed,
    Pending,
    Found(String, Option<String>),
}

/// Lit les cookies Steam d'un domaine **sur le thread principal** (requis par
/// WebView2) et renvoie le résultat via un canal. Non bloquant pour le rendu.
fn read_cookies_on_main(app: &tauri::AppHandle, domain: &str) -> CookieRead {
    let (tx, rx) = std::sync::mpsc::channel();
    let inner = app.clone();
    let domain = domain.to_string();
    let posted = app.run_on_main_thread(move || {
        let result = match inner.get_webview_window(STEAM_LOGIN_LABEL) {
            None => CookieRead::Closed,
            Some(win) => {
                let url: tauri::Url = domain.parse().unwrap();
                let mut secure = None;
                let mut sessionid = None;
                if let Ok(cookies) = win.cookies_for_url(url) {
                    for c in cookies {
                        match c.name() {
                            "steamLoginSecure" => secure = Some(c.value().to_string()),
                            "sessionid" => sessionid = Some(c.value().to_string()),
                            _ => {}
                        }
                    }
                }
                match secure {
                    Some(s) => CookieRead::Found(s, sessionid),
                    None => CookieRead::Pending,
                }
            }
        };
        let _ = tx.send(result);
    });
    if posted.is_err() {
        return CookieRead::Closed;
    }
    rx.recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or(CookieRead::Pending)
}

/// Sonde le cookie `steamLoginSecure` d'un domaine, jusqu'à `max_secs` secondes.
fn poll_cookie(
    app: &tauri::AppHandle,
    domain: &str,
    max_secs: u32,
) -> Option<(String, Option<String>)> {
    for _ in 0..max_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        match read_cookies_on_main(app, domain) {
            CookieRead::Closed => return None,
            CookieRead::Found(secure, sessionid) => return Some((secure, sessionid)),
            CookieRead::Pending => {}
        }
    }
    None
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
    // Canal pour remonter le refresh token capté par le script d'injection.
    let (rt_tx, rt_rx) = std::sync::mpsc::channel::<String>();

    // Tant qu'aucun refresh token n'est stocké, on force un login FRAIS : Steam n'émet
    // le refresh token (~200 j) que lors d'une vraie saisie identifiants + Steam Guard,
    // jamais sur une session « mémorisée ». Une fois le token capté, les connexions
    // suivantes redeviennent mémorisées (l'auto-refresh gère l'expiration ~24 h).
    let dir0 = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let force_fresh = accounts::secrets::load(&dir0).steam_refresh_token.is_none();

    // Ouverture de la fenêtre de login (sur le thread principal).
    let builder_app = app.clone();
    app.run_on_main_thread(move || {
        if let Some(win) = builder_app.get_webview_window(STEAM_LOGIN_LABEL) {
            let _ = win.close();
        }
        let url =
            tauri::WebviewUrl::External("https://store.steampowered.com/login/".parse().unwrap());
        let _ = tauri::WebviewWindowBuilder::new(&builder_app, STEAM_LOGIN_LABEL, url)
            .title("Connexion Steam")
            .inner_size(500.0, 740.0)
            .initialization_script(STEAM_CAPTURE_JS)
            .on_document_title_changed(move |_win, title| {
                if let Some(rt) = title.strip_prefix("ludo-steam-rt:") {
                    let _ = rt_tx.send(rt.to_string());
                }
            })
            .build();
    })
    .map_err(|e| e.to_string())?;

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
        let (store_secure, store_sid) = poll_cookie(&poll_app, "https://store.steampowered.com", 120)?;
        let steam_id = steam_id_from_cookie(&store_secure);

        // Phase 2 : on navigue la fenêtre vers la communauté (propage la session),
        // puis on lit son cookie. Optionnel : si ça échoue, on gardera le store seul.
        let nav_inner = poll_app.clone();
        let _ = poll_app.run_on_main_thread(move || {
            if let Some(win) = nav_inner.get_webview_window(STEAM_LOGIN_LABEL) {
                let _ = win.eval("window.location.href='https://steamcommunity.com/my/games/?tab=all';");
            }
        });
        let community = poll_cookie(&poll_app, "https://steamcommunity.com", 25);

        // Le refresh token a été capté pendant le login (finalizelogin) ; on draine
        // le canal (le dernier reçu, au cas où plusieurs titres seraient passés).
        let refresh_token = rt_rx.try_iter().last();

        Some((store_secure, store_sid, steam_id, community, refresh_token))
    })
    .await
    .map_err(|e| e.to_string())?;

    // Fermeture de la fenêtre de login.
    let closing_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = closing_app.get_webview_window(STEAM_LOGIN_LABEL) {
            let _ = win.close();
        }
    });

    let Some((store_secure, store_sid, steam_id, community, refresh_token)) = captured else {
        return Err("Connexion Steam non détectée (délai dépassé ou fenêtre fermée).".into());
    };
    let steam_id = steam_id.or_else(accounts::steam::detect_steam_id);

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    if let (Some(rt), Some(id)) = (creds.steam_refresh_token.as_deref(), creds.steam_id.as_deref()) {
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.steam_login_secure = None;
    creds.steam_community = None;
    creds.steam_api_key = None;
    creds.steam_id = None;
    creds.steam_refresh_token = None;
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

const GOG_LOGIN_LABEL: &str = "gog-login";
const GOG_AUTH_URL: &str = "https://auth.gog.com/auth?client_id=46899977096215655\
    &redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient\
    &response_type=code&layout=client2";

enum CodeRead {
    Closed,
    Pending,
    Found(String),
}

/// Lit l'URL courante de la fenêtre de login GOG **sur le thread principal**
/// (requis par WebView2) et en extrait le paramètre `code` une fois arrivé sur
/// la page de redirection `on_login_success`.
fn read_gog_code_on_main(app: &tauri::AppHandle) -> CodeRead {
    let (tx, rx) = std::sync::mpsc::channel();
    let inner = app.clone();
    let posted = app.run_on_main_thread(move || {
        let result = match inner.get_webview_window(GOG_LOGIN_LABEL) {
            None => CodeRead::Closed,
            Some(win) => match win.url() {
                Ok(url) if url.path().contains("on_login_success") => url
                    .query_pairs()
                    .find(|(k, _)| k == "code")
                    .map(|(_, v)| CodeRead::Found(v.into_owned()))
                    .unwrap_or(CodeRead::Pending),
                _ => CodeRead::Pending,
            },
        };
        let _ = tx.send(result);
    });
    if posted.is_err() {
        return CodeRead::Closed;
    }
    rx.recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or(CodeRead::Pending)
}

/// Sonde l'URL de la fenêtre GOG jusqu'à obtenir le code, la fermeture, ou l'expiration.
fn poll_gog_code(app: &tauri::AppHandle, max_secs: u32) -> Option<String> {
    for _ in 0..max_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        match read_gog_code_on_main(app) {
            CodeRead::Closed => return None,
            CodeRead::Found(code) => return Some(code),
            CodeRead::Pending => {}
        }
    }
    None
}

/// Ouvre la fenêtre de connexion GOG officielle (OAuth) et échange le code
/// obtenu contre un refresh token stocké localement. Pas de clé, pas de mot de
/// passe transmis à Ludo. `async` pour ne pas bloquer le rendu de la WebView.
#[tauri::command]
async fn connect_gog(app: tauri::AppHandle) -> Result<Settings, String> {
    let builder_app = app.clone();
    app.run_on_main_thread(move || {
        if let Some(win) = builder_app.get_webview_window(GOG_LOGIN_LABEL) {
            let _ = win.close();
        }
        let url = tauri::WebviewUrl::External(GOG_AUTH_URL.parse().unwrap());
        let _ = tauri::WebviewWindowBuilder::new(&builder_app, GOG_LOGIN_LABEL, url)
            .title("Connexion GOG")
            .inner_size(500.0, 740.0)
            // Les logins sociaux GOG (Google, Steam, Discord…) s'ouvrent via
            // `window.open` : sans ce handler, la popup est ignorée et le clic ne
            // fait rien. On autorise WebView2 à créer la fenêtre (session partagée).
            .on_new_window(|_url, _features| tauri::webview::NewWindowResponse::Allow)
            .build();
    })
    .map_err(|e| e.to_string())?;

    let poll_app = app.clone();
    let code = tauri::async_runtime::spawn_blocking(move || poll_gog_code(&poll_app, 180))
        .await
        .map_err(|e| e.to_string())?;

    let closing_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = closing_app.get_webview_window(GOG_LOGIN_LABEL) {
            let _ = win.close();
        }
    });

    let Some(code) = code else {
        return Err("Connexion GOG non détectée (délai dépassé ou fenêtre fermée).".into());
    };
    let tokens =
        accounts::gog::exchange_code(&code).ok_or("Échec de l'échange du code GOG.")?;

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.gog_refresh_token = Some(tokens.refresh_token);
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte GOG (efface le refresh token).
#[tauri::command]
fn disconnect_gog(app: tauri::AppHandle) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.gog_refresh_token = None;
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

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
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let builder_app = app.clone();
    app.run_on_main_thread(move || {
        if let Some(win) = builder_app.get_webview_window(EPIC_LOGIN_LABEL) {
            let _ = win.close();
        }
        let url = tauri::WebviewUrl::External(accounts::epic::login_url().parse().unwrap());
        let _ = tauri::WebviewWindowBuilder::new(&builder_app, EPIC_LOGIN_LABEL, url)
            .title("Connexion Epic Games")
            .inner_size(500.0, 740.0)
            .initialization_script(EPIC_CAPTURE_JS)
            .on_new_window(|_url, _features| tauri::webview::NewWindowResponse::Allow)
            .on_document_title_changed(move |_win, title| {
                if let Some(code) = title.strip_prefix("ludo-epic:") {
                    let _ = tx.send(code.to_string());
                }
            })
            .build();
    })
    .map_err(|e| e.to_string())?;

    // Attend le code (via le canal) ou la fermeture de la fenêtre, jusqu'à ~3 min.
    let poll_app = app.clone();
    let code = tauri::async_runtime::spawn_blocking(move || {
        for _ in 0..180 {
            if let Ok(code) = rx.try_recv() {
                return Some(code);
            }
            let (tx2, rx2) = std::sync::mpsc::channel();
            let inner = poll_app.clone();
            let _ = poll_app.run_on_main_thread(move || {
                let _ = tx2.send(inner.get_webview_window(EPIC_LOGIN_LABEL).is_some());
            });
            let open = rx2
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap_or(true);
            if !open {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        None
    })
    .await
    .map_err(|e| e.to_string())?;

    let closing_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = closing_app.get_webview_window(EPIC_LOGIN_LABEL) {
            let _ = win.close();
        }
    });

    let Some(code) = code else {
        return Err("Connexion Epic non détectée (délai dépassé ou fenêtre fermée).".into());
    };
    let tokens =
        accounts::epic::exchange_code(&code).ok_or("Échec de l'échange du code Epic.")?;

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.epic_refresh_token = Some(tokens.refresh_token);
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte Epic (efface le refresh token).
#[tauri::command]
fn disconnect_epic(app: tauri::AppHandle) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut creds = accounts::secrets::load(&dir);
    creds.epic_refresh_token = None;
    accounts::secrets::save(&dir, &creds)?;
    Ok(Settings::from_creds(&creds, &dir))
}

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
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let builder_app = app.clone();
    app.run_on_main_thread(move || {
        if let Some(win) = builder_app.get_webview_window(EA_LOGIN_LABEL) {
            let _ = win.close();
        }
        // On démarre sur l'endpoint token (prompt=none) : si la session existe déjà
        // (reconnexion) → token direct, sinon le JS redirige vers le login.
        let url = tauri::WebviewUrl::External(accounts::ea::TOKEN_ENDPOINT.parse().unwrap());
        let _ = tauri::WebviewWindowBuilder::new(&builder_app, EA_LOGIN_LABEL, url)
            .title("Connexion EA")
            .inner_size(500.0, 760.0)
            .initialization_script(EA_CAPTURE_JS)
            .on_new_window(|_url, _features| tauri::webview::NewWindowResponse::Allow)
            .on_document_title_changed(move |_win, title| {
                if let Some(tok) = title.strip_prefix("ludo-ea:") {
                    let _ = tx.send(tok.to_string());
                }
            })
            .build();
    })
    .map_err(|e| e.to_string())?;

    // Attend le token (via le canal) ou la fermeture de la fenêtre, jusqu'à ~3 min.
    let poll_app = app.clone();
    let token = tauri::async_runtime::spawn_blocking(move || {
        for _ in 0..180 {
            if let Ok(tok) = rx.try_recv() {
                return Some(tok);
            }
            let (tx2, rx2) = std::sync::mpsc::channel();
            let inner = poll_app.clone();
            let _ = poll_app.run_on_main_thread(move || {
                let _ = tx2.send(inner.get_webview_window(EA_LOGIN_LABEL).is_some());
            });
            let open = rx2
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap_or(true);
            if !open {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        None
    })
    .await
    .map_err(|e| e.to_string())?;

    let closing_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = closing_app.get_webview_window(EA_LOGIN_LABEL) {
            let _ = win.close();
        }
    });

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

    let creds = accounts::secrets::load(&dir);
    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte EA : supprime le snapshot en cache ET efface la session web EA (ouvre
/// brièvement la page de logout, auto-fermée) pour autoriser un vrai changement de compte.
#[tauri::command]
async fn disconnect_ea(app: tauri::AppHandle) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    accounts::ea::disconnect(&dir);

    // Ouvre la page de logout EA (efface le cookie de session côté webview).
    let open_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = open_app.get_webview_window(EA_LOGIN_LABEL) {
            let _ = win.close();
        }
        // Page de déconnexion utilisateur EA (efface la session sans paramètre OAuth ;
        // `connect/logout` exigeait un redirect_uri enregistré → erreur 102111).
        let logout = "https://www.ea.com/logout";
        if let Ok(url) = logout.parse() {
            let _ = tauri::WebviewWindowBuilder::new(
                &open_app,
                EA_LOGIN_LABEL,
                tauri::WebviewUrl::External(url),
            )
            .title("Déconnexion EA")
            .inner_size(440.0, 320.0)
            .build();
        }
    });

    // Laisse le logout s'exécuter, puis referme la fenêtre.
    let close_app = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let inner = close_app.clone();
        let _ = close_app.run_on_main_thread(move || {
            if let Some(win) = inner.get_webview_window(EA_LOGIN_LABEL) {
                let _ = win.close();
            }
        });
    })
    .await;

    let creds = accounts::secrets::load(&dir);
    Ok(Settings::from_creds(&creds, &dir))
}

const BNET_LOGIN_LABEL: &str = "battlenet-login";

/// Lit **tous** les cookies de `account.battle.net` dans la fenêtre de login (thread
/// principal, requis par WebView2) et les assemble en header `Cookie`. `None` si la
/// fenêtre est fermée ; `Some("")` tant qu'aucun cookie n'est encore posé.
fn read_bnet_cookies(app: &tauri::AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let inner = app.clone();
    let posted = app.run_on_main_thread(move || {
        let header = inner.get_webview_window(BNET_LOGIN_LABEL).map(|win| {
            let url: tauri::Url = "https://account.battle.net".parse().unwrap();
            win.cookies_for_url(url)
                .map(|cookies| {
                    cookies
                        .iter()
                        .map(|c| format!("{}={}", c.name(), c.value()))
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_default()
        });
        let _ = tx.send(header);
    });
    if posted.is_err() {
        return None;
    }
    rx.recv_timeout(std::time::Duration::from_secs(3))
        .unwrap_or(Some(String::new()))
}

/// Ouvre la fenêtre de connexion Battle.net ; une fois connecté, lit les cookies de
/// session et récupère la bibliothèque possédée (`games-and-subs`), mise en cache disque.
#[tauri::command]
async fn connect_battlenet(app: tauri::AppHandle) -> Result<Settings, String> {
    let builder_app = app.clone();
    app.run_on_main_thread(move || {
        if let Some(win) = builder_app.get_webview_window(BNET_LOGIN_LABEL) {
            let _ = win.close();
        }
        let url = tauri::WebviewUrl::External("https://account.battle.net/".parse().unwrap());
        let _ = tauri::WebviewWindowBuilder::new(&builder_app, BNET_LOGIN_LABEL, url)
            .title("Connexion Battle.net")
            .inner_size(520.0, 760.0)
            .on_new_window(|_url, _features| tauri::webview::NewWindowResponse::Allow)
            .build();
    })
    .map_err(|e| e.to_string())?;

    // Sonde les cookies puis tente l'API jusqu'à obtenir des jeux (= connecté), ~3 min.
    let poll_app = app.clone();
    let games = tauri::async_runtime::spawn_blocking(move || {
        for _ in 0..90 {
            match read_bnet_cookies(&poll_app) {
                None => return None, // fenêtre fermée
                Some(header) if !header.is_empty() => {
                    let games = accounts::battlenet::fetch_library(&header);
                    if !games.is_empty() {
                        return Some(games);
                    }
                }
                Some(_) => {}
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        None
    })
    .await
    .map_err(|e| e.to_string())?;

    let closing_app = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(win) = closing_app.get_webview_window(BNET_LOGIN_LABEL) {
            let _ = win.close();
        }
    });

    let Some(games) = games else {
        return Err("Connexion Battle.net non détectée (délai dépassé ou fenêtre fermée).".into());
    };

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    accounts::battlenet::save_library(&dir, &games);
    let creds = accounts::secrets::load(&dir);
    Ok(Settings::from_creds(&creds, &dir))
}

/// Déconnecte Battle.net (supprime le snapshot en cache).
#[tauri::command]
fn disconnect_battlenet(app: tauri::AppHandle) -> Result<Settings, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    accounts::battlenet::disconnect(&dir);
    let creds = accounts::secrets::load(&dir);
    Ok(Settings::from_creds(&creds, &dir))
}

/// Une jaquette résolue pour un jeu (renvoyée au front pour fusion réactive).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverUpdate {
    id: String,
    cover_url: String,
}

/// Remplit les jaquettes manquantes de TOUTE la bibliothèque (n'importe quel launcher)
/// via une recherche Steam Store par titre (publique, sans clé), mise en cache disque.
/// Renvoie les jaquettes résolues ; le front les applique de façon réactive à la grille.
#[tauri::command]
async fn enrich_covers(app: tauri::AppHandle) -> Result<Vec<CoverUpdate>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let updates = tauri::async_runtime::spawn_blocking(move || {
        let games = platforms::scan_all(Some(&dir));
        metadata::fill_covers(&games, &dir)
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(updates
        .into_iter()
        .map(|(id, cover_url)| CoverUpdate { id, cover_url })
        .collect())
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
    let emitter = app.clone();
    let updates = tauri::async_runtime::spawn_blocking(move || {
        let games = platforms::scan_all(Some(&dir));
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

/// Boutique — vitrine : une page de jeux mis en avant / en promo (CheapShark),
/// selon le tri (`featured`, `savings`, `price`, `recent`, `rating`). Découverte
/// pure : indépendant de la bibliothèque de l'utilisateur.
#[tauri::command]
async fn store_deals(
    page: u32,
    sort: String,
) -> Result<Vec<metadata::store::StoreItem>, String> {
    tauri::async_runtime::spawn_blocking(move || metadata::store::deals(page, &sort))
        .await
        .map_err(|e| e.to_string())
}

/// Boutique — recherche de jeux par titre (renvoie le prix le plus bas de chacun).
#[tauri::command]
async fn store_search(query: String) -> Result<Vec<metadata::store::StoreItem>, String> {
    tauri::async_runtime::spawn_blocking(move || metadata::store::search(&query))
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

/// Wishlist Steam enrichie de prix (ITAD) : meilleur prix actuel + plus bas historique
/// par jeu. Vide si Steam n'est pas connecté.
#[tauri::command]
async fn steam_wishlist(
    app: tauri::AppHandle,
) -> Result<Vec<metadata::store::WishlistItem>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let appids = accounts::steam_wishlist_appids(&dir);
        metadata::store::wishlist(&appids)
    })
    .await
    .map_err(|e| e.to_string())
}

/// Wishlist **unifiée** : wishlist Steam native ∪ wishlist Torii (universelle, tout jeu),
/// dédupliquée et enrichie de prix (ITAD). Les entrées Torius déjà présentes sur Steam
/// (même appid) ne sont pas doublées.
#[tauri::command]
async fn wishlist_all(app: tauri::AppHandle) -> Result<Vec<metadata::store::WishlistItem>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let steam_appids = accounts::steam_wishlist_appids(&dir);
        let steam_set: std::collections::HashSet<u64> = steam_appids.iter().copied().collect();
        let mut items = metadata::store::wishlist(&steam_appids);
        let extra: Vec<(String, u64, String, Option<String>)> = platforms::wishlist::load(&dir)
            .into_iter()
            .filter(|e| e.steam_appid.map_or(true, |a| !steam_set.contains(&a)))
            .map(|e| (e.id, e.steam_appid.unwrap_or(0), e.title, e.cover_url))
            .collect();
        items.extend(metadata::store::wishlist_custom(&extra));
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
#[tauri::command]
fn scan_library(app: tauri::AppHandle) -> Vec<GameDto> {
    let config_dir = app.path().app_config_dir().ok();
    platforms::scan_all(config_dir.as_deref())
}

/// Scanne puis enrichit chaque jeu avec des métadonnées en ligne (cache disque).
/// Plus lent que `scan_library` : à appeler en arrière-plan après l'affichage.
#[tauri::command]
fn enrich_metadata(app: tauri::AppHandle) -> Result<Vec<GameDto>, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut games = platforms::scan_all(Some(&config_dir));
    let emitter = app.clone();
    metadata::enrich(&mut games, &config_dir, move |done, total| {
        let _ = emitter.emit("enrich-progress", EnrichProgress { done, total });
    });
    // On écarte les non-jeux (DLC, bandes-son, vidéos…) une fois leur type connu,
    // sauf s'ils sont installés (ex: une démo installée reste visible).
    games.retain(|g| {
        g.installed || g.app_type.as_deref().map_or(true, |t| t == "game")
    });
    Ok(games)
}

/// Enrichit **un seul** jeu à la demande (à l'ouverture de sa vue détail) :
/// description, captures, développeur, année, genre. Résultat mis en cache disque.
#[tauri::command]
fn enrich_game(
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
    Ok(metadata::enrich_one(&game, &dir))
}

/// Masque ou réaffiche un jeu (liste d'exclusion) ; renvoie la liste des ids masqués.
#[tauri::command]
fn set_game_hidden(
    app: tauri::AppHandle,
    id: String,
    hidden: bool,
) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::hidden::set(&dir, &id, hidden)
}

/// Épingle ou retire un jeu des favoris ; renvoie la liste des ids favoris à jour.
#[tauri::command]
fn set_game_favorite(
    app: tauri::AppHandle,
    id: String,
    favorite: bool,
) -> Result<Vec<String>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::favorites::set(&dir, &id, favorite)
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

/// Retire un jeu manuel par son id.
#[tauri::command]
fn remove_manual_game(app: tauri::AppHandle, id: String) -> Result<Vec<GameDto>, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    platforms::manual::remove(&dir, &id)
}

/// Arme le suivi d'une session de jeu : minimise Torii, attend l'apparition puis la
/// disparition d'un process situé sous `install_dir` (le jeu), et à sa fermeture
/// restaure la fenêtre au premier plan et émet `game-exited` (le front ouvre la fiche).
/// Ne concerne que les jeux **installés lancés depuis Torii** (dossier connu).
#[tauri::command]
fn start_game_watch(app: tauri::AppHandle, game_id: String, install_dir: String) {
    let dir = std::path::PathBuf::from(&install_dir);
    if !dir.is_dir() {
        return;
    }
    // Torii se réduit pendant la partie.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.minimize();
    }
    std::thread::spawn(move || game_watch_loop(app, game_id, dir));
}

/// `true` si un process en cours a son exécutable sous `dir` (le jeu tourne).
fn process_running_in(sys: &sysinfo::System, dir: &std::path::Path) -> bool {
    sys.processes()
        .values()
        .any(|p| p.exe().map(|e| e.starts_with(dir)).unwrap_or(false))
}

/// Boucle de surveillance d'une session (thread dédié).
fn game_watch_loop(app: tauri::AppHandle, game_id: String, dir: std::path::PathBuf) {
    use std::time::{Duration, Instant};
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    let mut sys = System::new();
    let poll = Duration::from_secs(3);
    let refresh = |sys: &mut System| {
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new().with_exe(sysinfo::UpdateKind::Always),
        );
    };

    // 1) Attendre que le jeu démarre (le launcher met quelques secondes) — max 2 min.
    let start = Instant::now();
    let mut seen = false;
    while start.elapsed() < Duration::from_secs(120) {
        std::thread::sleep(poll);
        refresh(&mut sys);
        if process_running_in(&sys, &dir) {
            seen = true;
            break;
        }
    }
    // Lancement non détecté (exe hors dossier, échec…) : on ne force rien.
    if !seen {
        return;
    }

    // 2) Attendre la fermeture du jeu.
    loop {
        std::thread::sleep(poll);
        refresh(&mut sys);
        if !process_running_in(&sys, &dir) {
            break;
        }
    }

    // 3) Retour : restaurer, premier plan, agrandir, puis le front ouvre la fiche.
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.maximize();
    }
    let _ = app.emit("game-exited", GameExited { id: game_id });
}

/// Préférences liées à la fenêtre, lues côté Rust (au démarrage et à la fermeture)
/// donc persistées dans un fichier plutôt qu'en localStorage.
#[derive(Serialize, serde::Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "camelCase", default)]
struct WindowPrefs {
    /// Démarrer réduit dans la zone de notification (fenêtre cachée au lancement).
    start_minimized: bool,
    /// Fermer la fenêtre la réduit dans le tray au lieu de quitter l'application.
    close_to_tray: bool,
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
            enrich_metadata,
            enrich_game,
            set_game_hidden,
            set_game_favorite,
            launch_game,
            install_game,
            record_launch,
            uninstall_game,
            open_install_dir,
            add_manual_game,
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
            enrich_covers,
            enrich_igdb,
            store_deals,
            store_search,
            store_suggest,
            store_game,
            steam_friends,
            steam_me,
            steam_achievements,
            steam_current_players,
            steam_wishlist,
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
            notify_user
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
