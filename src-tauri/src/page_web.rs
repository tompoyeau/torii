//! Page web intégrée : une WebView enfant posée DANS la fenêtre principale, à la place
//! du contenu, sous la barre que dessine l'interface (`PageWeb.vue`).
//!
//! 🔑 DANS LA FENÊTRE, PAS À CÔTÉ. Une fenêtre à part (l'ancienne `open_web_window`)
//! restait une sortie : une deuxième fenêtre dans la barre des tâches, sans la barre
//! latérale ni l'historique de Torii. Le but est de ne jamais quitter l'application, y
//! compris pour acheter un jeu chez un revendeur.
//!
//! 🔑 CE QUE LA PAGE PEUT FAIRE, ET CE QU'ELLE NE PEUT PAS
//! - Elle partage les cookies de Torii. ⚠️ Mais seul Steam y garde une session durable :
//!   celles d'Epic, EA, GOG et Battle.net ne survivent pas à la fermeture de Torii (ou
//!   ont été effacées par une reconnexion Steam, qui vide les données de navigation).
//!   Epic est donc connecté à la demande, par son code d'échange (`session_epic`).
//! - Elle n'a AUCUN accès aux commandes de Torii : la capability `default` ne déclare pas
//!   de `remote`, et Tauri refuse alors l'IPC à toute origine distante.
//! - Torii n'y ouvre que des adresses qu'il fournit lui-même (pages des launchers, liens
//!   d'achat du comparateur), jamais une adresse tapée ou reçue d'un ami. Une fois
//!   dedans, on navigue librement en HTTPS : un paiement passe par PayPal, Stripe ou la
//!   page de la banque, des domaines qu'aucune liste blanche ne peut prévoir. C'est la
//!   barre de Torii, qui affiche toujours le domaine courant, qui permet de vérifier où
//!   l'on est.
//! - Le reste (http, `steam://`, `mailto:`…) part vers Windows ou le navigateur.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Rect, Url, WebviewUrl};

use crate::locale;

/// Une seule page à la fois, mais chacune a son propre nom (`torii-page-3`) : rouvrir
/// une page après en avoir fermé une en crée une neuve, sans jamais retomber sur l'ancienne.
const PREFIXE: &str = "torii-page-";
static NUMERO: AtomicU32 = AtomicU32::new(1);

/// Nom de la page affichée, `None` s'il n'y en a pas.
///
/// 🔑 C'EST CE NOM QUI FAIT FOI, pas « existe-t-il une WebView qui s'appelle ainsi » : une
/// page fermée est oubliée ici aussitôt, même si sa destruction n'est pas encore achevée.
/// En test, une page rouverte après une fermeture était retombée sur l'ancienne, déjà
/// détruite : la place restait vide, sans erreur.
static PAGE: Mutex<Option<String>> = Mutex::new(None);

fn nom_courant() -> Option<String> {
    PAGE.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

fn page_courante(app: &AppHandle) -> Option<tauri::Webview> {
    app.get_webview(&nom_courant()?)
}

/// Une page encore en vie mais remplacée ne doit plus rien écrire dans l'état affiché.
fn est_courante(nom: &str) -> bool {
    nom_courant().as_deref() == Some(nom)
}
/// Événement envoyé à l'interface à chaque changement (adresse, titre, chargement, historique).
const EVENEMENT: &str = "page-web";

/// Protocoles qu'une page peut confier à Windows : ceux des launchers (« Installer » sur
/// la boutique Steam ouvre `steam://`), le courriel, et le HTTP non chiffré, qui part
/// dans le navigateur plutôt que de s'afficher sans cadenas dans Torii.
const PROTOCOLES_EXTERNES: [&str; 8] = [
    "http", "mailto", "steam", "com.epicgames.launcher", "goggalaxy", "uplay", "origin2", "battlenet",
];

/// Emplacement de la page, en pixels logiques depuis le coin de la zone cliente :
/// c'est ce que mesure `getBoundingClientRect` dans l'interface.
#[derive(Deserialize, Clone, Copy)]
pub struct Bornes {
    x: f64,
    y: f64,
    largeur: f64,
    hauteur: f64,
}

impl Bornes {
    fn rect(self) -> Rect {
        Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.largeur.max(1.0), self.hauteur.max(1.0)).into(),
        }
    }
}

/// Ce que la barre de Torii affiche de la page.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Etat {
    url: String,
    titre: String,
    chargement: bool,
    peut_reculer: bool,
    peut_avancer: bool,
}

const ETAT_VIDE: Etat = Etat {
    url: String::new(),
    titre: String::new(),
    chargement: false,
    peut_reculer: false,
    peut_avancer: false,
};

/// Les événements arrivent par morceaux (un titre, puis l'historique…) : on garde l'état
/// complet ici pour toujours envoyer un tout cohérent.
static ETAT: Mutex<Etat> = Mutex::new(ETAT_VIDE);

/// La page a-t-elle déjà ouvert sa session Epic ? Une fois par page : le code d'échange
/// pose les cookies, les pages Epic suivantes en profitent.
static SESSION_EPIC: AtomicBool = AtomicBool::new(false);

fn est_epic(url: &Url) -> bool {
    url.host_str().is_some_and(|h| h == "epicgames.com" || h.ends_with(".epicgames.com"))
}

/// Première arrivée chez Epic (lien direct, ou redirection d'un lien d'achat) : on
/// suspend cette navigation et on la reprend par l'adresse qui connecte la page au compte
/// Epic de Torii. Sans compte Epic connecté, ou si Epic ne répond pas, on reprend telle
/// quelle : la page s'ouvre, simplement sans session.
fn session_epic(app: &AppHandle, cible: &Url) -> bool {
    if !est_epic(cible) || SESSION_EPIC.swap(true, Ordering::SeqCst) {
        return true;
    }
    let app = app.clone();
    let cible = cible.clone();
    tauri::async_runtime::spawn(async move {
        let dossier = app.path().app_config_dir().ok();
        let adresse = cible.to_string();
        let connectee = tauri::async_runtime::spawn_blocking(move || {
            dossier.and_then(|d| crate::accounts::epic::url_connectee(&d, &adresse))
        })
        .await
        .ok()
        .flatten()
        .and_then(|u| u.parse::<Url>().ok());
        let Some(page) = page_courante(&app) else { return };
        let adresse = connectee.unwrap_or(cible);
        // Rien d'affiché encore (lien d'achat en pleine redirection) : la page n'a que sa
        // page blanche de départ. On la REMPLACE : naviguer l'empilerait, et « précédent »
        // ramènerait sur du blanc au lieu de revenir dans Torii.
        let vierge = {
            let e = ETAT.lock().unwrap_or_else(|p| p.into_inner());
            e.url.is_empty() || e.url == "about:blank"
        };
        match serde_json::to_string(adresse.as_str()) {
            Ok(js) if vierge => {
                let _ = page.eval(format!("location.replace({js})"));
            }
            _ => {
                let _ = page.navigate(adresse);
            }
        }
    });
    false
}

fn maj(app: &AppHandle, f: impl FnOnce(&mut Etat)) {
    let etat = {
        let mut e = ETAT.lock().unwrap_or_else(|p| p.into_inner());
        f(&mut e);
        e.clone()
    };
    let _ = app.emit_to("main", EVENEMENT, etat);
}

fn erreur(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// L'adresse qu'on demande d'ouvrir : HTTPS seulement.
fn verifier(url: &str) -> Result<Url, String> {
    let cible: Url = url.parse().map_err(|_| locale::tr("Adresse illisible.", "Unreadable address.").to_string())?;
    if cible.scheme() != "https" {
        return Err(locale::tr("Torii n'ouvre que des adresses sécurisées.", "Torii only opens secure addresses.").into());
    }
    Ok(cible)
}

/// La page peut-elle aller là ? Sinon, l'adresse part vers Windows quand c'est un
/// protocole connu, et nulle part sinon.
fn autoriser(url: &Url) -> bool {
    match url.scheme() {
        "https" | "about" | "blob" => true,
        schema if PROTOCOLES_EXTERNES.contains(&schema) => {
            let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
            false
        }
        _ => false,
    }
}

/// Ouvre une page dans Torii, ou y renvoie celle qui est déjà ouverte.
#[tauri::command]
pub async fn page_ouvrir(app: AppHandle, url: String, bornes: Bornes) -> Result<(), String> {
    let cible = verifier(&url)?;
    if let Some(page) = page_courante(&app) {
        page.set_bounds(bornes.rect()).map_err(erreur)?;
        // Déjà sur cette adresse (retour à une étape où la page n'avait pas bougé) : la
        // recharger perdrait ce qu'on y avait fait.
        if page.url().map(|u| u != cible).unwrap_or(true) {
            page.navigate(cible).map_err(erreur)?;
        }
        page.show().map_err(erreur)?;
        return Ok(());
    }

    let fenetre = app
        .get_window("main")
        .ok_or_else(|| locale::tr("Fenêtre principale introuvable.", "Main window not found.").to_string())?;
    *ETAT.lock().unwrap_or_else(|p| p.into_inner()) = ETAT_VIDE;
    SESSION_EPIC.store(false, Ordering::SeqCst);
    let nom = format!("{PREFIXE}{}", NUMERO.fetch_add(1, Ordering::SeqCst));
    // Posé AVANT la création : les premiers événements de chargement arrivent pendant.
    *PAGE.lock().unwrap_or_else(|p| p.into_inner()) = Some(nom.clone());

    let app_navigation = app.clone();
    let app_popup = app.clone();
    let app_chargement = app.clone();
    let app_titre = app.clone();
    let constructeur = WebviewBuilder::new(&nom, WebviewUrl::External(cible))
        .on_navigation(move |url| autoriser(url) && session_epic(&app_navigation, url))
        .on_new_window(move |url, options| {
            // Une vraie fenêtre surgissante (le site en fixe la taille) : PayPal, la page
            // de la banque… Elle garde le lien avec la page qui l'a ouverte, dont le
            // paiement a besoin pour se conclure.
            if options.size().is_some() {
                return NewWindowResponse::Allow;
            }
            // Un simple lien « nouvel onglet » : on le suit dans la page elle-même.
            // ⚠️ Hors du gestionnaire : naviguer pendant que WebView2 nous demande quoi
            // faire de la fenêtre, c'est rentrer dans son traitement en cours.
            if autoriser(&url) {
                let app = app_popup.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(page) = page_courante(&app) {
                        let _ = page.navigate(url);
                    }
                });
            }
            NewWindowResponse::Deny
        })
        .on_page_load(move |vue, charge| {
            if !est_courante(vue.label()) {
                return;
            }
            let url = charge.url().to_string();
            let en_cours = matches!(charge.event(), PageLoadEvent::Started);
            maj(&app_chargement, |e| {
                e.url = url;
                e.chargement = en_cours;
            });
        })
        .on_document_title_changed(move |vue, titre| {
            if !est_courante(vue.label()) {
                return;
            }
            // Écrit par le site : borné, une barre ne doit pas déborder.
            let titre: String = titre.chars().take(200).collect();
            maj(&app_titre, |e| e.titre = titre);
        });

    let page = match fenetre.add_child(
        constructeur,
        LogicalPosition::new(bornes.x, bornes.y),
        LogicalSize::new(bornes.largeur.max(1.0), bornes.hauteur.max(1.0)),
    ) {
        Ok(page) => page,
        Err(e) => {
            *PAGE.lock().unwrap_or_else(|p| p.into_inner()) = None;
            return Err(erreur(e));
        }
    };
    #[cfg(windows)]
    suivre_historique(&page, app.clone());
    let _ = page.set_focus();
    Ok(())
}

/// Recale la page sur l'emplacement que lui réserve l'interface (fenêtre redimensionnée).
#[tauri::command]
pub fn page_bornes(app: AppHandle, bornes: Bornes) -> Result<(), String> {
    match page_courante(&app) {
        Some(page) => page.set_bounds(bornes.rect()).map_err(erreur),
        None => Ok(()),
    }
}

/// Montre ou cache la page. 🔑 Elle passe AU-DESSUS de tout ce que dessine l'interface :
/// une fiche, les Paramètres ou une modale ouverts par-dessus resteraient invisibles.
#[tauri::command]
pub fn page_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    match page_courante(&app) {
        Some(page) if visible => page.show().map_err(erreur),
        Some(page) => page.hide().map_err(erreur),
        None => Ok(()),
    }
}

/// Ferme la page (on quitte l'écran « page web » de Torii).
#[tauri::command]
pub fn page_fermer(app: AppHandle) -> Result<(), String> {
    *ETAT.lock().unwrap_or_else(|p| p.into_inner()) = ETAT_VIDE;
    // Oubliée d'abord : une réouverture immédiate en créera une neuve quoi qu'il arrive.
    let nom = PAGE.lock().unwrap_or_else(|p| p.into_inner()).take();
    match nom.and_then(|n| app.get_webview(&n)) {
        Some(page) => page.close().map_err(erreur),
        None => Ok(()),
    }
}

/// Les boutons de la barre : précédent et suivant DANS la page, recharger.
#[tauri::command]
pub fn page_action(app: AppHandle, action: String) -> Result<(), String> {
    let Some(page) = page_courante(&app) else { return Ok(()) };
    match action.as_str() {
        "precedent" => page.eval("history.back()").map_err(erreur),
        "suivant" => page.eval("history.forward()").map_err(erreur),
        "recharger" => page.reload().map_err(erreur),
        _ => Err(format!("action inconnue : {action}")),
    }
}

/// Branche l'historique de WebView2 sur la barre : peut-on reculer, avancer, et où en
/// est-on. 🔑 L'adresse vient aussi d'ici et pas seulement du chargement des pages : la
/// boutique Steam change d'adresse sans recharger (`pushState`), et la barre afficherait
/// sinon une page qu'on a quittée.
#[cfg(windows)]
fn suivre_historique(page: &tauri::Webview, app: AppHandle) {
    use webview2_com::{HistoryChangedEventHandler, SourceChangedEventHandler};

    let nom = page.label().to_string();
    let _ = page.with_webview(move |natif| unsafe {
        let Ok(coeur) = natif.controller().CoreWebView2() else { return };
        let mut jeton = 0i64;
        let app_historique = app.clone();
        let nom_historique = nom.clone();
        let _ = coeur.add_HistoryChanged(
            &HistoryChangedEventHandler::create(Box::new(move |vue, _| {
                if let Some(vue) = vue.filter(|_| est_courante(&nom_historique)) {
                    lire_historique(&app_historique, &vue);
                }
                Ok(())
            })),
            &mut jeton,
        );
        let _ = coeur.add_SourceChanged(
            &SourceChangedEventHandler::create(Box::new(move |vue, _| {
                if let Some(vue) = vue.filter(|_| est_courante(&nom)) {
                    lire_historique(&app, &vue);
                }
                Ok(())
            })),
            &mut jeton,
        );
    });
}

#[cfg(windows)]
unsafe fn lire_historique(app: &AppHandle, vue: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2) {
    let mut recul = windows_core::BOOL::default();
    let mut avance = windows_core::BOOL::default();
    let _ = vue.CanGoBack(&mut recul);
    let _ = vue.CanGoForward(&mut avance);
    let mut source = windows_core::PWSTR::null();
    let url = if vue.Source(&mut source).is_ok() { webview2_com::take_pwstr(source) } else { String::new() };
    maj(app, |e| {
        e.peut_reculer = recul.as_bool();
        e.peut_avancer = avance.as_bool();
        if !url.is_empty() {
            e.url = url;
        }
    });
}
