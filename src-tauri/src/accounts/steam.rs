use crate::models::GameDto;
use crate::platforms;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

const CDN: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps";

fn cover_url(appid: u64) -> Option<String> {
    Some(format!("{CDN}/{appid}/library_600x900.jpg"))
}
fn hero_url(appid: u64) -> Option<String> {
    Some(format!("{CDN}/{appid}/library_hero.jpg"))
}

const BROWSER_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// Récupère la bibliothèque possédée via la **session de login** (cookie), sans clé API.
/// Lit l'endpoint `dynamicstore/userdata` (liste des appids possédés + wishlist).
pub fn owned_from_session(config_dir: &Path, cookie: &str) -> Vec<GameDto> {
    let Some(json) = fetch_userdata(cookie) else {
        return Vec::new();
    };
    let owned = json["rgOwnedApps"].as_array().cloned().unwrap_or_default();
    if owned.is_empty() {
        return Vec::new();
    }
    // On mémorise la wishlist pour le futur comparateur de prix.
    if let Some(list) = json["rgWishlist"].as_array() {
        let ids: Vec<u64> = list.iter().filter_map(Value::as_u64).collect();
        let _ = std::fs::write(
            config_dir.join("steam_wishlist.json"),
            serde_json::to_string(&ids).unwrap_or_default(),
        );
    }

    // Les noms ne sont pas fournis ici (GetAppList keyless retiré par Steam) :
    // titre provisoire « App <id> », résolu ensuite par l'enrichissement (appdetails).
    owned
        .iter()
        .filter_map(Value::as_u64)
        .map(|appid| GameDto {
            id: format!("steam:{appid}"),
            title: format!("App {appid}"),
            platform: "steam".into(),
            installed: false,
            owned: true,
            cover_url: cover_url(appid),
            hero_url: hero_url(appid),
            launch_target: appid.to_string(),
            ..Default::default()
        })
        .collect()
}

/// GET authentifié gérant les redirections de Steam **à la manière d'un navigateur** :
/// on réinjecte les cookies posés (`steamCountry`, `Steam_Language`…) ET on **suit le
/// `Location`**. Deux redirections coexistent :
///  - même URL après pose de cookies (dynamicstore/userdata) → la réinjection suffit ;
///  - 🔑 `/profiles/<id64>/…` → `/id/<vanity>/…` pour les comptes ayant une **URL
///    personnalisée** : là il FAUT suivre le `Location`. Sans ça on rejouait la même URL
///    `/profiles/…` en boucle → réponse vide (symptôme : liste d'amis absente uniquement
///    pour ces comptes-là, alors que la biblio/Family — qui passe par api.steampowered.com —
///    fonctionne).
fn fetch_text(url: &str, cookie: &str, referer: &str) -> Option<String> {
    let agent = ureq::builder()
        .redirects(0)
        .timeout(Duration::from_secs(25))
        .build();
    let mut cookie = cookie.to_string();
    let mut url = url.to_string();

    for _ in 0..6 {
        let resp = agent
            .get(&url)
            .set("User-Agent", BROWSER_UA)
            .set("Referer", referer)
            .set("Accept-Encoding", "identity")
            .set("Cookie", &cookie)
            .call()
            .ok()?;

        if (300..400).contains(&resp.status()) {
            // Cookie jar : Steam exige ces cookies sur la requête rejouée.
            for set_cookie in resp.all("set-cookie") {
                if let Some(pair) = set_cookie.split(';').next() {
                    if pair.contains('=') {
                        cookie.push_str("; ");
                        cookie.push_str(pair.trim());
                    }
                }
            }
            // Suit la redirection (absolue ou relative à l'origine courante).
            match resp.header("location") {
                None => return None,
                Some(loc) => {
                    url = if loc.starts_with("http") {
                        loc.to_string()
                    } else {
                        let origin = url
                            .find("://")
                            .and_then(|i| url[i + 3..].find('/').map(|j| url[..i + 3 + j].to_string()))
                            .unwrap_or_else(|| url.clone());
                        if loc.starts_with('/') {
                            format!("{origin}{loc}")
                        } else {
                            format!("{origin}/{loc}")
                        }
                    };
                }
            }
            continue;
        }
        return resp.into_string().ok();
    }
    None
}

fn fetch_userdata(cookie: &str) -> Option<Value> {
    let body = fetch_text(
        "https://store.steampowered.com/dynamicstore/userdata/",
        cookie,
        "https://store.steampowered.com/",
    )?;
    serde_json::from_str(&body).ok()
}

/// Bibliothèque possédée via la **page communautaire des jeux** (XML) : uniquement
/// des jeux (pas de DLC), avec leurs noms et le temps de jeu, en un seul appel.
/// Nécessite le cookie de session du domaine `steamcommunity.com`.
pub fn owned_from_community(steam_id: &str, cookie: &str) -> Vec<GameDto> {
    // 1. Charge la page des jeux (authentifiée) pour en extraire le jeton WebAPI.
    let page = format!("https://steamcommunity.com/profiles/{steam_id}/games/?tab=all");
    let Some(html) = fetch_text(&page, cookie, "https://steamcommunity.com/") else {
        return Vec::new();
    };
    let Some(token) = extract_webapi_token(&html) else {
        return Vec::new();
    };
    // 2. Comme Playnite : on privilégie la **bibliothèque familiale** (tes jeux +
    //    ceux partagés par tes proches). Sinon, tes jeux possédés seuls.
    let family = family_library(steam_id, &token);
    if !family.is_empty() {
        return family;
    }
    owned_via_api(steam_id, &format!("access_token={token}"))
}

/// Bibliothèque partagée du groupe familial Steam (jeux possédés + partagés).
fn family_library(steam_id: &str, token: &str) -> Vec<GameDto> {
    let group = get_json(&format!(
        "https://api.steampowered.com/IFamilyGroupsService/GetFamilyGroupForUser/v1/\
         ?access_token={token}&steamid={steam_id}"
    ));
    let Some(family_id) = group.and_then(|j| {
        j["response"]["family_groupid"]
            .as_str()
            .map(String::from)
            .filter(|id| id != "0")
    }) else {
        return Vec::new();
    };

    let apps = get_json(&format!(
        "https://api.steampowered.com/IFamilyGroupsService/GetSharedLibraryApps/v1/\
         ?access_token={token}&family_groupid={family_id}&include_own=true\
         &include_excluded=true&include_free=false&max_apps=5000"
    ));
    apps.and_then(|json| json["response"]["apps"].as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| parse_family_app(a, steam_id))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_family_app(app: &Value, me: &str) -> Option<GameDto> {
    let appid = app["appid"].as_u64()?;
    let name = app["name"].as_str()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let playtime = app["rt_playtime"].as_u64().unwrap_or(0) as u32;
    let last_played = app["rt_last_played"].as_i64().filter(|&t| t > 0);
    let owners: Vec<String> = app["owner_steamids"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|o| o.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let owned_by_me = owners.iter().any(|o| o == me);

    Some(GameDto {
        id: format!("steam:{appid}"),
        title: name,
        platform: "steam".into(),
        installed: false,
        owned: owned_by_me,
        family_shared: !owned_by_me,
        family_owners: owners,
        playtime_minutes: (playtime > 0).then_some(playtime),
        cover_url: cover_url(appid),
        hero_url: hero_url(appid),
        launch_target: appid.to_string(),
        last_played,
        app_type: Some("game".into()),
        ..Default::default()
    })
}

/// Récupère un **WebAPIToken** (JWT de session, ~24 h) à partir de NOTRE page
/// communautaire des jeux. Réutilisable pour interroger l'API `IPlayerService` sans clé,
/// y compris pour les profils d'amis (voir [`owned_with_token`]). Un seul appel réseau,
/// à mutualiser entre plusieurs requêtes.
pub fn web_api_token(my_steam_id: &str, cookie: &str) -> Option<String> {
    let page = format!("https://steamcommunity.com/profiles/{my_steam_id}/games/?tab=all");
    let html = fetch_text(&page, cookie, "https://steamcommunity.com/")?;
    extract_webapi_token(&html)
}

/// Bibliothèque possédée d'un utilisateur **quelconque** (nous ou un ami) via
/// `GetOwnedGames`, avec un token obtenu par [`web_api_token`]. ⚠️ Pour un ami, ne renvoie
/// des jeux que si son profil expose « Détails des jeux » (public, ou amis-seulement
/// puisqu'on est amis) ; sinon liste vide = profil privé, indistinguable d'un compte sans jeu.
pub fn owned_with_token(steam_id: &str, token: &str) -> Vec<GameDto> {
    owned_via_api(steam_id, &format!("access_token={token}"))
}

/// Wishlist Steam via `IWishlistService/GetWishlist` — même auth sans clé que
/// `GetOwnedGames` (le WebAPIToken de session). Renvoie les appids souhaités, triés par
/// priorité. Les noms/jaquettes se résolvent ensuite via le CDN + enrichissement, comme
/// les jeux possédés.
pub fn wishlist(steam_id: &str, token: &str) -> Vec<u64> {
    let url = format!(
        "https://api.steampowered.com/IWishlistService/GetWishlist/v1/\
         ?access_token={token}&steamid={steam_id}"
    );
    let Some(mut items) = get_json(&url).and_then(|j| j["response"]["items"].as_array().cloned())
    else {
        return Vec::new();
    };
    // Tri par priorité croissante (0 = pas de priorité définie → en dernier).
    items.sort_by_key(|i| match i["priority"].as_u64() {
        Some(0) | None => u64::MAX,
        Some(p) => p,
    });
    items.iter().filter_map(|i| i["appid"].as_u64()).collect()
}

/// Extrait le jeton WebAPI (JWT) embarqué dans une page communautaire.
/// La page contient  `\"WebAPIToken\":\"<jwt>\"`  (guillemets échappés une fois).
fn extract_webapi_token(html: &str) -> Option<String> {
    let after = &html[html.find("WebAPIToken")?..];
    let start = after.find(":\\\"")? + 3;
    let value = &after[start..];
    let end = value.find("\\\"")?;
    Some(value[..end].to_string())
}

/// Appelle `IPlayerService/GetOwnedGames` avec une clé (`key=…`) ou un jeton de
/// session (`access_token=…`). Renvoie les jeux possédés (pas les DLC) + temps de jeu.
fn owned_via_api(steam_id: &str, auth: &str) -> Vec<GameDto> {
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?{auth}\
         &steamid={steam_id}&include_appinfo=1&include_played_free_games=1&format=json"
    );
    get_json(&url)
        .and_then(|json| json["response"]["games"].as_array().cloned())
        .map(|arr| arr.iter().filter_map(parse_owned).collect())
        .unwrap_or_default()
}

/// Un ami Steam avec sa présence, envoyé au frontend.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub steam_id: String,
    pub name: String,
    pub avatar_url: String,
    /// "in-game" | "online" | "away" | "busy" | "snooze" | "offline"
    pub state: String,
    /// Jeu en cours (si en jeu).
    pub game_name: Option<String>,
    /// URL du profil communautaire.
    pub profile_url: String,
}

/// Liste d'amis Steam + présence, par **scrape de la page communautaire** des amis
/// (sans clé API : le token web ne donne pas accès à `GetFriendList`). Utilise le
/// cookie de session. La page groupe les amis en « en jeu / en ligne / hors ligne »
/// avec avatar, pseudo et jeu en cours.
pub fn friends(steam_id: &str, cookie: &str) -> Vec<Friend> {
    let url = format!("https://steamcommunity.com/profiles/{steam_id}/friends/");
    let Some(html) = fetch_text(&url, cookie, "https://steamcommunity.com/") else {
        return Vec::new();
    };
    parse_friends_page(&html)
}

/// Extrait la sous-chaîne entre `start` et `end` (première occurrence).
fn slice_between<'a>(hay: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = hay.find(start)? + start.len();
    let rest = &hay[i..];
    let j = rest.find(end)?;
    Some(&rest[..j])
}

/// Décode les quelques entités HTML rencontrées dans les pseudos/jeux.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}

/// Parse la page communautaire des amis en blocs `friend_block_v2`.
fn parse_friends_page(html: &str) -> Vec<Friend> {
    let mut out = Vec::new();
    // Chaque bloc d'ami commence par `data-steamid="<id64>"`. On découpe dessus :
    // le segment suivant contient tout le bloc (statut, avatar, pseudo, jeu).
    for seg in html.split("data-steamid=\"").skip(1) {
        let steam_id: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
        if steam_id.len() != 17 {
            continue;
        }
        // Statut : classe de l'overlay d'avatar (`friend_block_link_overlay <état>`).
        let state = ["in-game", "online", "away", "busy", "snooze"]
            .iter()
            .find(|s| seg.contains(&format!("friend_block_link_overlay {s}")))
            .map(|s| (*s).to_string())
            .unwrap_or_else(|| "offline".into());
        let name = slice_between(seg, "friend_block_content\">", "<br")
            .map(decode_entities)
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let game_name = slice_between(seg, "friend_game_link\">", "</span>")
            .map(decode_entities)
            .filter(|s| !s.is_empty());
        // Avatar (medium → full pour une meilleure définition).
        let avatar_url = seg
            .find("player_avatar")
            .and_then(|i| slice_between(&seg[i..], "src=\"", "\""))
            .map(|s| s.replace("_medium", "_full"))
            .unwrap_or_default();
        let profile_url = seg
            .find("selectable_overlay")
            .and_then(|i| slice_between(&seg[i..], "href=\"", "\""))
            .unwrap_or_default()
            .to_string();

        out.push(Friend {
            steam_id,
            name,
            avatar_url,
            state,
            game_name,
            profile_url,
        });
    }
    out
}

/// Le profil Steam de l'utilisateur connecté (pseudo + avatar), pour l'en-tête.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SteamProfile {
    pub steam_id: String,
    pub name: String,
    pub avatar_url: String,
    pub profile_url: String,
}

/// Récupère le profil de l'utilisateur (pseudo + avatar) par scrape de sa page
/// communautaire publique. Le cookie de session couvre aussi les profils privés.
pub fn profile(steam_id: &str, cookie: &str) -> Option<SteamProfile> {
    let url = format!("https://steamcommunity.com/profiles/{steam_id}/");
    let html = fetch_text(&url, cookie, "https://steamcommunity.com/")?;
    parse_profile(steam_id, &html)
}

/// Extrait pseudo + avatar de la page profil (SSR). Pseudo via `actual_persona_name`
/// (repli `og:title`, préfixé « Steam Community :: »), avatar via `og:image` (`_full`).
fn parse_profile(steam_id: &str, html: &str) -> Option<SteamProfile> {
    let name = slice_between(html, "actual_persona_name\">", "</span>")
        .map(decode_entities)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            slice_between(html, "og:title\" content=\"", "\"")
                .map(|s| decode_entities(s.trim_start_matches("Steam Community :: ")))
        })
        .filter(|s| !s.is_empty())?;
    let avatar_url = slice_between(html, "og:image\" content=\"", "\"")
        .unwrap_or_default()
        .to_string();
    Some(SteamProfile {
        steam_id: steam_id.to_string(),
        name,
        avatar_url,
        profile_url: format!("https://steamcommunity.com/profiles/{steam_id}/"),
    })
}

/// Un succès Steam (débloqué ou non), pour la fiche détail.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub name: String,
    pub description: String,
    pub icon: String,
    pub unlocked: bool,
    /// Texte de déblocage localisé (ex. « Débloqué le 30 aout 2023 à 10h28 »), si débloqué.
    pub unlocked_at: Option<String>,
}

/// Les succès d'un jeu Steam pour l'utilisateur, envoyés au frontend.
#[derive(serde::Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameAchievements {
    pub unlocked: u32,
    pub total: u32,
    pub items: Vec<Achievement>,
}

/// Succès Steam d'un jeu pour l'utilisateur connecté, par **scrape de la page perso**
/// des succès (`ISteamUserStats/GetPlayerAchievements` exige une clé API ; le WebAPIToken
/// ne marche PAS pour `ISteamUserStats`). Donne nom, description, icône et date de
/// déblocage par succès. Le total exact (succès cachés inclus) vient du % global public.
/// Nécessite le cookie de session (couvre aussi les profils privés, c'est le nôtre).
/// `None` = jeu sans succès ou page indisponible.
pub fn achievements(steam_id: &str, appid: u64, cookie: &str) -> Option<GameAchievements> {
    let url = format!(
        "https://steamcommunity.com/profiles/{steam_id}/stats/{appid}/achievements/?l=french"
    );
    let html = fetch_text(&url, cookie, "https://steamcommunity.com/")?;
    let items = parse_achievements(&html);
    if items.is_empty() {
        return None;
    }
    let unlocked = items.iter().filter(|a| a.unlocked).count() as u32;
    // Total exact (succès cachés non listés inclus) via le % global public, sans clé.
    // Replis : la barre de progression de la page, puis le nombre de succès affichés.
    let total = global_total(appid)
        .filter(|&t| t >= unlocked)
        .or_else(|| total_from_bar(&html, unlocked))
        .unwrap_or(0)
        .max(items.len() as u32);
    Some(GameAchievements { unlocked, total, items })
}

/// Nombre de joueurs en ce moment sur un jeu Steam, via `GetNumberOfCurrentPlayers`
/// (public, sans clé). `None` si indisponible (jeu sans stats, appid inconnu…).
pub fn current_players(appid: u64) -> Option<u32> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetNumberOfCurrentPlayers/v1/?appid={appid}"
    );
    let json = get_json(&url)?;
    // result==1 = donnée valide ; sinon player_count peut être 0 par défaut → on écarte.
    (json["response"]["result"].as_u64() == Some(1))
        .then(|| json["response"]["player_count"].as_u64())
        .flatten()
        .map(|n| n as u32)
}

/// Total des succès du jeu via `GetGlobalAchievementPercentagesForApp` (public, sans clé).
fn global_total(appid: u64) -> Option<u32> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/\
         GetGlobalAchievementPercentagesForApp/v2/?gameid={appid}"
    );
    let n = get_json(&url)?["achievementpercentages"]["achievements"]
        .as_array()?
        .len();
    (n > 0).then_some(n as u32)
}

/// Total dérivé de la largeur de la barre de progression (« width: X% ») + le nb débloqué.
fn total_from_bar(html: &str, unlocked: u32) -> Option<u32> {
    let pct: f64 = slice_between(html, "achieveBarProgress\" style=\"width: ", "%")?
        .trim()
        .parse()
        .ok()?;
    (pct > 0.0).then(|| (unlocked as f64 * 100.0 / pct).round() as u32)
}

/// Parse les blocs `achieveRow` de la page perso des succès. Débloqués d'abord
/// (ordre de la page conservé), puis verrouillés.
fn parse_achievements(html: &str) -> Vec<Achievement> {
    let mut out = Vec::new();
    for chunk in html.split("class=\"achieveRow\"").skip(1) {
        // Steam ajoute une ligne récapitulative « N succès cachés restants »
        // (`achieveHiddenBox`, sans icône) : elle n'est pas un vrai succès → ignorée.
        if chunk.contains("achieveHiddenBox") {
            continue;
        }
        let name = match slice_between(chunk, "<h3 class=\"ellipsis\">", "</h3>") {
            Some(n) => decode_entities(n),
            None => continue,
        };
        if name.is_empty() {
            continue;
        }
        let icon = slice_between(chunk, "src=\"", "\"").unwrap_or_default().to_string();
        let description = slice_between(chunk, "<h5>", "</h5>")
            .map(decode_entities)
            .unwrap_or_default();
        let unlocked = chunk.contains("achieveUnlockTime");
        let unlocked_at = unlocked
            .then(|| slice_between(chunk, "achieveUnlockTime\">", "</div>").map(clean_unlock_time))
            .flatten()
            .filter(|s| !s.is_empty());
        out.push(Achievement { name, description, icon, unlocked, unlocked_at });
    }
    out.sort_by_key(|a| !a.unlocked); // stable : débloqués (false) avant verrouillés (true)
    out
}

/// Nettoie le contenu du bloc de date : retire les balises (`<br/>`) et normalise les espaces.
fn clean_unlock_time(s: &str) -> String {
    let mut txt = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => txt.push(c),
            _ => {}
        }
    }
    txt.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{extract_webapi_token, parse_achievements, parse_friends_page, parse_profile};

    #[test]
    fn extracts_webapi_token() {
        let html = r#"...Data = ["{\"WebAPIToken\":\"eyABC.def-ghi_123\",\"other\":1}"]..."#;
        assert_eq!(extract_webapi_token(html).as_deref(), Some("eyABC.def-ghi_123"));
    }

    // Bloc calqué sur la vraie page communautaire des amis (1 en jeu, 1 en ligne).
    #[test]
    fn parses_friends_page() {
        let html = r##"
        <div class="selectable friend_block_v2 persona in-game  " id="fr_1" data-steamid="76561198206344635" data-miniprofile="246078907" data-search="sterben ; soundpad ; ">
          <a class="selectable_overlay" data-container="#fr_1" href="https://steamcommunity.com/id/lenyben"></a>
          <div class="player_avatar friend_block_link_overlay in-game"><img src="https://av/aaa_medium.jpg"></div>
          <div class="friend_block_content">Sterben<br><span class="friend_small_text"><span class="friend_game_link">Soundpad</span></span></div>
        </div>
        <div class="selectable friend_block_v2 persona online  " id="fr_2" data-steamid="76561198243042658" data-miniprofile="282776930" data-search="ecrevisse ;  ; ">
          <a class="selectable_overlay" data-container="#fr_2" href="https://steamcommunity.com/profiles/76561198243042658"></a>
          <div class="player_avatar friend_block_link_overlay online"><img src="https://av/bbb_medium.jpg"></div>
          <div class="friend_block_content">Ecrevisse<br><span class="friend_small_text"></span></div>
        </div>"##;
        let f = parse_friends_page(html);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].steam_id, "76561198206344635");
        assert_eq!(f[0].name, "Sterben");
        assert_eq!(f[0].state, "in-game");
        assert_eq!(f[0].game_name.as_deref(), Some("Soundpad"));
        assert_eq!(f[0].avatar_url, "https://av/aaa_full.jpg"); // _medium → _full
        assert_eq!(f[1].name, "Ecrevisse");
        assert_eq!(f[1].state, "online");
        assert_eq!(f[1].game_name, None);
    }

    // Bloc calqué sur la vraie page perso des succès : 1 débloqué, 1 verrouillé,
    // et la ligne récapitulative « succès cachés » (à ignorer).
    #[test]
    fn parses_achievements_page() {
        let html = r##"
        <div role="button" class="achieveRow">
            <div class="achieveImgHolder"><img src="https://cdn/apps/1/aaa.jpg"></div>
            <div class="achieveTxtHolder"><div class="achieveTxt">
                <h3 class="ellipsis">Fuite de l&#39;Avernus</h3>
                <h5>Prendre le contrôle du nautilo&amp;ide.</h5>
            </div>
            <div class="achieveUnlockTime">
                Débloqué le 30 aout 2023 à 10h28<br/>
            </div></div>
        </div>
        <div role="button" class="achieveRow">
            <div class="achieveImgHolder"><img src="https://cdn/apps/1/bbb.jpg"></div>
            <div class="achieveTxtHolder"><div class="achieveTxt">
                <h3 class="ellipsis">Talent Show</h3>
                <h5>Encore verrouillé.</h5>
            </div></div>
        </div>
        <div role="button" class="achieveRow">
            <div class="achieveHiddenBox"><span>+9</span></div>
            <div class="achieveTxtHolder"><div class="achieveTxt">
                <h3 class="ellipsis">9 succès cachés restants</h3>
                <h5>Révélés une fois débloqués</h5>
            </div></div>
        </div>"##;
        let a = parse_achievements(html);
        assert_eq!(a.len(), 2); // la ligne "cachés restants" est ignorée
        // Débloqué d'abord (tri stable).
        assert_eq!(a[0].name, "Fuite de l'Avernus"); // entité &#39; décodée
        assert_eq!(a[0].description, "Prendre le contrôle du nautilo&ide."); // &amp; décodé
        assert_eq!(a[0].icon, "https://cdn/apps/1/aaa.jpg");
        assert!(a[0].unlocked);
        assert_eq!(a[0].unlocked_at.as_deref(), Some("Débloqué le 30 aout 2023 à 10h28"));
        assert_eq!(a[1].name, "Talent Show");
        assert!(!a[1].unlocked);
        assert_eq!(a[1].unlocked_at, None);
    }

    // SSR calqué sur une vraie page profil Steam.
    #[test]
    fn parses_profile_page() {
        let html = r#"
        <meta property="og:title" content="Steam Community :: PomPoteau">
        <meta property="og:image" content="https://av/3604ac_full.jpg">
        <div class="persona_name"><span class="actual_persona_name">PomPoteau</span></div>"#;
        let p = parse_profile("76561198258753323", html).unwrap();
        assert_eq!(p.name, "PomPoteau");
        assert_eq!(p.avatar_url, "https://av/3604ac_full.jpg");
        assert_eq!(p.profile_url, "https://steamcommunity.com/profiles/76561198258753323/");
    }

    // Repli sur og:title (préfixe retiré) si actual_persona_name absent.
    #[test]
    fn parses_profile_fallback_title() {
        let html = r#"<meta property="og:title" content="Steam Community :: Jean-Kevin">
        <meta property="og:image" content="https://av/x_full.jpg">"#;
        let p = parse_profile("76561198000000000", html).unwrap();
        assert_eq!(p.name, "Jean-Kevin");
    }
}

/// Détecte le SteamID64 du compte le plus récemment connecté (`loginusers.vdf`).
pub fn detect_steam_id() -> Option<String> {
    let steam = platforms::steam::steam_path()?;
    let text = std::fs::read_to_string(steam.join("config").join("loginusers.vdf")).ok()?;
    let mut current: Option<String> = None;
    for line in text.lines() {
        let toks: Vec<&str> = line.split('"').collect();
        if toks.len() < 2 {
            continue;
        }
        let key = toks[1];
        if key.len() == 17 && key.chars().all(|c| c.is_ascii_digit()) {
            current = Some(key.to_string());
        } else if key.eq_ignore_ascii_case("MostRecent") && toks.len() >= 4 && toks[3] == "1" {
            if current.is_some() {
                return current;
            }
        }
    }
    current // à défaut, le dernier compte listé
}

/// Récupère les jeux possédés via une **clé API** (chemin avancé/optionnel).
pub fn owned_games(api_key: &str, steam_id: &str) -> Vec<GameDto> {
    owned_via_api(steam_id, &format!("key={api_key}"))
}

/// Régénère un **cookie de session web frais** à partir du refresh token, sans
/// reconnexion. Le cookie web (`steamLoginSecure`) expire en ~24 h alors que le
/// refresh token vit ~200 j → on redérive un access token à chaque scan si besoin.
///
/// Régénère un cookie web `steamLoginSecure` frais à partir du refresh token (~200 j),
/// sans reconnexion. ✅ validé en live.
///
/// 🔑 `GenerateAccessTokenForApp` NE marche PAS ici : il exige un token d'audience
/// « client » ; notre refresh token web a `aud=[web, renew, derive]` → AccessDenied
/// (x-eresult 15). La bonne voie = rejouer le flux de login : POST `jwt/finalizelogin`
/// avec `nonce=<refresh_token>` → `transfer_info` (URLs `settoken`) → POST le `settoken`
/// communautaire → le `Set-Cookie` renvoie le `steamLoginSecure` frais.
pub fn refresh_web_cookie(refresh_token: &str, steam_id: &str) -> Option<String> {
    refresh_domain_cookie(refresh_token, steam_id, "steamcommunity.com")
}

/// Comme [`refresh_web_cookie`] mais pour le domaine **store** (`store.steampowered.com`),
/// requis par l'API wishlist du store (`addtowishlist`).
pub fn refresh_store_cookie(refresh_token: &str, steam_id: &str) -> Option<String> {
    refresh_domain_cookie(refresh_token, steam_id, "store.steampowered.com")
}

/// Régénère un cookie `steamLoginSecure` frais pour un **domaine** donné (communautaire ou
/// store) à partir du refresh token, en rejouant le flux de login (finalizelogin → settoken).
fn refresh_domain_cookie(refresh_token: &str, steam_id: &str, domain: &str) -> Option<String> {
    let sessionid = random_sessionid();

    // 1) finalizelogin : échange le refresh token contre les URLs settoken par domaine.
    let fin: Value = ureq::post("https://login.steampowered.com/jwt/finalizelogin")
        .timeout(Duration::from_secs(15))
        .set("Cookie", &format!("sessionid={sessionid}"))
        .set("Origin", "https://steamcommunity.com")
        .set("Referer", "https://steamcommunity.com/")
        .send_form(&[
            ("nonce", refresh_token),
            ("sessionid", &sessionid),
            ("redir", "https://steamcommunity.com/login/home/?goto="),
        ])
        .ok()?
        .into_json()
        .ok()?;

    // 2) On prend l'entrée du domaine demandé.
    let comm = fin["transfer_info"]
        .as_array()?
        .iter()
        .find(|t| t["url"].as_str().is_some_and(|u| u.contains(domain)))?;
    let url = comm["url"].as_str()?;
    let nonce = comm["params"]["nonce"].as_str()?;
    let auth = comm["params"]["auth"].as_str()?;

    // 3) settoken pose le cookie via un Set-Cookie ; `redirects(0)` pour le lire sans suivre.
    let resp = ureq::builder()
        .redirects(0)
        .build()
        .post(url)
        .timeout(Duration::from_secs(15))
        .send_form(&[("nonce", nonce), ("auth", auth), ("steamID", steam_id)])
        .ok()?;

    for set_cookie in resp.all("set-cookie") {
        if let Some(rest) = set_cookie.strip_prefix("steamLoginSecure=") {
            let value = rest.split(';').next().unwrap_or("");
            if !value.is_empty() {
                return Some(format!("steamLoginSecure={value}"));
            }
        }
    }
    None
}

/// Ajoute (`add=true`) ou retire un jeu de la **vraie wishlist Steam**, via l'endpoint
/// AJAX du store (`store.steampowered.com/api/addtowishlist` / `removefromwishlist`) —
/// le même que le bouton « Ajouter à la liste de souhaits » du site. Utilise le cookie
/// de session **store** (`steamLoginSecure`) + un `sessionid` (double-submit CSRF).
/// Renvoie `true` si Steam confirme. ⚠️ API interne non documentée → à valider en réel.
pub fn set_wishlist(appid: u64, add: bool, store_cookie: &str) -> Result<bool, String> {
    let sessionid = random_sessionid();
    let endpoint = if add { "addtowishlist" } else { "removefromwishlist" };
    let resp = ureq::post(&format!("https://store.steampowered.com/api/{endpoint}"))
        .timeout(Duration::from_secs(15))
        .set("User-Agent", BROWSER_UA)
        .set("Cookie", &format!("{store_cookie}; sessionid={sessionid}"))
        .set("Origin", "https://store.steampowered.com")
        .set("Referer", &format!("https://store.steampowered.com/app/{appid}/"))
        .set("X-Requested-With", "XMLHttpRequest")
        .send_form(&[("appid", &appid.to_string()), ("sessionid", &sessionid)])
        .map_err(|e| e.to_string())?;
    let v: Value = resp.into_json().unwrap_or(Value::Null);
    Ok(v["success"].as_bool().unwrap_or(false))
}

/// `sessionid` aléatoire (24 hex) requis par `finalizelogin` (appariement CSRF ; il
/// suffit qu'il corresponde entre le cookie et le champ de formulaire).
fn random_sessionid() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:024x}")
}

fn parse_owned(game: &Value) -> Option<GameDto> {
    let appid = game["appid"].as_u64()?;
    let name = game["name"].as_str()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let playtime = game["playtime_forever"].as_u64().unwrap_or(0) as u32;
    let last_played = game["rtime_last_played"].as_i64().filter(|&t| t > 0);

    Some(GameDto {
        id: format!("steam:{appid}"),
        title: name,
        platform: "steam".into(),
        installed: false,
        owned: true,
        playtime_minutes: (playtime > 0).then_some(playtime),
        cover_url: cover_url(appid),
        hero_url: hero_url(appid),
        launch_target: appid.to_string(),
        last_played,
        app_type: Some("game".into()),
        ..Default::default()
    })
}

fn get_json(url: &str) -> Option<Value> {
    ureq::get(url)
        .timeout(Duration::from_secs(15))
        .call()
        .ok()?
        .into_json()
        .ok()
}
