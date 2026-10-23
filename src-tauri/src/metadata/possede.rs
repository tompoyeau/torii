//! Fiche d'un jeu de la bibliothèque : ce que Steam en dit, pour tout jeu qu'on peut
//! relier à Steam avec certitude — y compris un jeu acheté ailleurs (Control sur Epic).
//!
//! Deux sources sans clé, hors du quota partagé du comparateur : la fiche Steam (la même
//! que celle de la Boutique, `fiche::fiche`) et les annonces officielles de l'éditeur
//! (`ISteamNews`). Les succès et les contenus additionnels possédés dépendent du compte
//! Steam : ils ne valent que pour un jeu possédé SUR Steam.
//!
//! 🔑 LE LIEN AVEC STEAM N'EST JAMAIS DEVINÉ PAR LE TITRE. Chercher « Control » sur Steam
//! ramène « CONTROL Resonant », « League of Legends » ramène « CONVERGENCE: A League of
//! Legends Story » : deux autres jeux. Une fiche fausse est pire qu'une fiche courte. Liens
//! admis : le jeu est aussi sur Steam dans la bibliothèque (l'interface passe alors son
//! appid), la table Battle.net, ou l'identifiant Steam qu'IGDB associe à un nom exact
//! (`igdb::steam_appid`).

use super::fiche::{self, FicheSteam};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

const NEWS: &str = "https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/";
/// Adresse que Steam substitue au jeton `{STEAM_CLAN_IMAGE}` des annonces.
const IMAGES_CLAN: &str = "https://clan.akamai.steamstatic.com/images/";
const NB_ACTUALITES: u32 = 4;
const LONGUEUR_EXTRAIT: usize = 180;

/// Ce que la fiche d'un jeu possédé ajoute à ce que la bibliothèque sait déjà.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct FichePossede {
    pub app_id: u64,
    pub steam: Option<FicheSteam>,
    pub actualites: Vec<Actualite>,
    /// Tous les contenus additionnels, ceux qu'on possède d'abord : l'interface en montre 5,
    /// puis davantage à la demande (leurs prix sont demandés par lots). Vide quand
    /// le jeu n'est pas possédé sur Steam, ou que la session ne dit pas ce qu'on possède :
    /// une liste de prix sans savoir ce qu'on a déjà ne servirait à rien.
    pub dlc: Vec<u64>,
    pub dlc_possedes: Vec<u64>,
}

/// Une annonce de l'éditeur sur Steam (mise à jour, événement, notes de version).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Actualite {
    pub titre: String,
    pub url: String,
    /// Secondes Unix.
    pub date: i64,
    /// Le début du texte, sans mise en forme. Dans la langue de l'éditeur : Steam ne
    /// traduit pas les annonces.
    pub extrait: String,
    pub image: Option<String>,
}

/// L'appid Steam d'un jeu de la bibliothèque qui n'y est pas déjà relié, si un lien sûr
/// existe : table Battle.net, sinon IGDB sur un nom exact.
pub fn lien_steam(id: &str, titre: &str, plateforme: &str, dir: &Path) -> Option<u64> {
    if plateforme == "steam" {
        return id.strip_prefix("steam:")?.parse().ok();
    }
    if plateforme == "battlenet" {
        let code = id.strip_prefix("battlenet:").unwrap_or_default();
        if let Some(appid) = crate::accounts::battlenet::steam_appid(code) {
            return appid.parse().ok();
        }
    }
    super::igdb::steam_appid(id, titre, dir)
}

/// La fiche Steam et les annonces, demandées en parallèle.
pub fn fiche_possede(appid: u64, dir: &Path) -> FichePossede {
    let (steam, actualites) = std::thread::scope(|s| {
        let a = s.spawn(|| actualites(appid));
        let f = fiche::fiche(appid, dir);
        (f, a.join().ok().flatten().unwrap_or_default())
    });
    FichePossede { app_id: appid, steam, actualites, ..Default::default() }
}

/// Renseigne les contenus additionnels d'un jeu possédé sur Steam, d'après les appids que
/// le compte possède (`None` : on ne sait pas, on ne montre rien).
pub fn marquer_dlc(f: &mut FichePossede, possedes: Option<&HashSet<u64>>) {
    let (Some(steam), Some(possedes)) = (&f.steam, possedes) else {
        return;
    };
    f.dlc_possedes = steam.dlc.iter().copied().filter(|d| possedes.contains(d)).collect();
    f.dlc = possedes_d_abord(&steam.dlc, &f.dlc_possedes);
}

/// Ceux qu'on possède d'abord, puis les autres dans l'ordre de Steam.
fn possedes_d_abord(tous: &[u64], possedes: &[u64]) -> Vec<u64> {
    let mut out = possedes.to_vec();
    out.extend(tous.iter().copied().filter(|d| !possedes.contains(d)));
    out
}

/// Les dernières annonces officielles du jeu. Le fil de Steam mêle aux annonces de
/// l'éditeur des articles de presse (PCGamesN, Rock Paper Shotgun…) : on ne garde que le
/// fil des annonces, filtré par Steam lui-même.
fn actualites(appid: u64) -> Option<Vec<Actualite>> {
    let url = format!(
        "{NEWS}?appid={appid}&count={NB_ACTUALITES}&maxlength=0&feeds=steam_community_announcements"
    );
    let racine: Value = ureq::get(&url).timeout(Duration::from_secs(15)).call().ok()?.into_json().ok()?;
    Some(racine["appnews"]["newsitems"].as_array()?.iter().filter_map(lire_actualite).collect())
}

fn lire_actualite(n: &Value) -> Option<Actualite> {
    let titre = fiche::texte_brut(n["title"].as_str()?);
    let url = n["url"].as_str().filter(|u| u.starts_with("https://"))?.to_string();
    let contenu = n["contents"].as_str().unwrap_or_default();
    (!titre.is_empty()).then(|| Actualite {
        titre,
        url,
        date: n["date"].as_i64().unwrap_or(0),
        extrait: extrait(contenu),
        image: image(contenu),
    })
}

/// La première image de l'annonce, si elle vient des serveurs de Steam.
fn image(contenu: &str) -> Option<String> {
    contenu
        .split(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | '"' | '\'' | '<' | '>' | '=' | '(' | ')'))
        .filter_map(|m| {
            let url = match m.strip_prefix("{STEAM_CLAN_IMAGE}/") {
                Some(chemin) => format!("{IMAGES_CLAN}{chemin}"),
                None => m.to_string(),
            };
            let bas = url.to_ascii_lowercase();
            let est_image = [".jpg", ".jpeg", ".png", ".gif", ".webp"].iter().any(|e| bas.ends_with(e));
            (est_image && fiche::image_steam(&url)).then_some(url)
        })
        .next()
}

/// Le début du texte : sans BBCode (`[b]`, `[url=…]`, `[previewyoutube=…]`), sans HTML,
/// sans jeton d'image ni lien, coupé sur un mot.
fn extrait(contenu: &str) -> String {
    let mut sans_bbcode = String::with_capacity(contenu.len());
    let mut dans_balise = false;
    for c in contenu.chars() {
        match c {
            '[' => dans_balise = true,
            ']' if dans_balise => {
                dans_balise = false;
                sans_bbcode.push(' ');
            }
            _ if !dans_balise => sans_bbcode.push(c),
            _ => {}
        }
    }
    let texte = fiche::texte_brut(&sans_bbcode);
    let mots: Vec<&str> = texte
        .split_whitespace()
        .filter(|m| !m.starts_with("{STEAM_CLAN_IMAGE}") && !m.starts_with("http"))
        .collect();
    tronquer(&mots.join(" "), LONGUEUR_EXTRAIT)
}

fn tronquer(texte: &str, max: usize) -> String {
    if texte.chars().count() <= max {
        return texte.to_string();
    }
    let coupe: String = texte.chars().take(max).collect();
    let coupe = coupe.rsplit_once(' ').map_or(coupe.as_str(), |(debut, _)| debut);
    format!("{}…", coupe.trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Annonce réelle de Hunt: Showdown 1896 (20/10/2026), raccourcie.
    #[test]
    fn lit_une_annonce() {
        let n = json!({
            "title": "Twitch Drops Dates | Blood Testament",
            "url": "https://steamstore-a.akamaihd.net/news/externalpost/steam_community_announcements/1846651706696131",
            "date": 1_792_500_000,
            "contents": "[img]{STEAM_CLAN_IMAGE}/30897473/c98e8aaa00dde106cce0fa1770d02c4a9ccbe12b.jpg[/img] 🎁 [b]Twitch Drops are back![/b]  From Oct 26th to Nov 9th, you&#39;ll have another chance. [url=https://twitch.tv/x]Link[/url]"
        });
        let a = lire_actualite(&n).unwrap();
        assert_eq!(a.titre, "Twitch Drops Dates | Blood Testament");
        assert_eq!(
            a.image.as_deref(),
            Some("https://clan.akamai.steamstatic.com/images/30897473/c98e8aaa00dde106cce0fa1770d02c4a9ccbe12b.jpg")
        );
        assert_eq!(a.extrait, "🎁 Twitch Drops are back! From Oct 26th to Nov 9th, you'll have another chance. Link");
    }

    #[test]
    fn une_annonce_sans_image_ni_adresse_sure() {
        let n = json!({ "title": "CONTROL Resonant Is Out Now!", "url": "https://store.steampowered.com/news/1", "date": 1,
            "contents": "[p]Manhattan is on the brink.[/p][previewyoutube=\"WhQm-ExRz60;full\"][/previewyoutube] <img src=\"https://ailleurs.example/x.jpg\">" });
        let a = lire_actualite(&n).unwrap();
        assert_eq!(a.image, None, "une image hors des serveurs de Steam est écartée");
        assert_eq!(a.extrait, "Manhattan is on the brink.");
        assert!(lire_actualite(&json!({ "title": "x", "url": "http://non-chiffre" })).is_none());
    }

    #[test]
    fn un_extrait_long_est_coupe_sur_un_mot() {
        let long = "mot ".repeat(80);
        let e = extrait(&long);
        assert!(e.ends_with("mot…") && e.chars().count() <= LONGUEUR_EXTRAIT + 1);
    }

    #[test]
    fn les_contenus_possedes_passent_devant() {
        assert_eq!(possedes_d_abord(&[1, 2, 3, 4, 5], &[4, 2]), [4, 2, 1, 3, 5]);
        assert_eq!(possedes_d_abord(&[1, 2], &[]), [1, 2]);
        assert_eq!(possedes_d_abord(&[1, 2], &[1, 2]), [1, 2]);
    }

    #[test]
    fn sans_possession_connue_aucun_contenu_n_est_montre() {
        let steam = FicheSteam { dlc: vec![10, 20, 30], ..Default::default() };
        let mut f = FichePossede { steam: Some(steam), ..Default::default() };
        marquer_dlc(&mut f, None);
        assert!(f.dlc.is_empty());
        marquer_dlc(&mut f, Some(&HashSet::from([30, 99])));
        assert_eq!((f.dlc, f.dlc_possedes), (vec![30, 10, 20], vec![30]));
    }

    #[test]
    fn un_jeu_steam_est_relie_a_lui_meme() {
        assert_eq!(lien_steam("steam:594650", "Hunt", "steam", Path::new(".")), Some(594650));
    }
}
