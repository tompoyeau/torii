//! Bandeau de notification, en haut à droite de l'écran, au-dessus des autres fenêtres.
//!
//! C'est une vraie fenêtre : sans décoration, sans barre des tâches, toujours au premier
//! plan et **qui ne prend jamais le focus** — voler le clavier à quelqu'un qui joue serait
//! pire que de ne rien afficher.
//!
//! ⚠️ Limite honnête : un jeu en plein écran **exclusif** garde la main sur l'affichage et
//! le bandeau ne s'y superpose pas. Steam y parvient en s'injectant dans le jeu, ce que
//! Torii ne fait pas. En fenêtré sans bordure — le mode par défaut de la plupart des jeux
//! récents — ça fonctionne.

use crate::journal;
use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;

/// Identifiant de la fenêtre du n-ième bandeau.
///
/// 🔑 Un identifiant PAR bandeau, et non un identifiant fixe : chaque bandeau ferme LE
/// SIEN, sans dépendre du suivant ni le gêner.
fn label(generation: u64) -> String {
    format!("toast-{generation}")
}
/// Durée d'affichage. Assez pour lire deux lignes, assez peu pour ne pas gêner.
const DUREE_SECS: u64 = 6;
/// Taille logique du bandeau (pixels indépendants de la densité d'écran).
const LARGEUR: f64 = 330.0;
const HAUTEUR: f64 = 96.0;
/// Marge par rapport aux bords de l'écran.
const MARGE: f64 = 18.0;

/// Compteur de bandeaux affichés, pour donner à chacun son identifiant.
#[derive(Default)]
pub struct Toasts(pub Mutex<u64>);

#[derive(Serialize)]
struct Contenu<'a> {
    titre: &'a str,
    corps: &'a str,
}

/// Retire tous les bandeaux encore ouverts, sauf celui qu'on vient de créer.
///
/// Filet de sécurité : une fenêtre peut survivre à un rechargement à chaud pendant le
/// développement. Sans ce balayage, elle reste affichée **définitivement**.
fn balayer(app: &tauri::AppHandle, sauf: Option<&str>) {
    for (etiquette, fenetre) in app.webview_windows() {
        if etiquette.starts_with("toast-") && Some(etiquette.as_str()) != sauf {
            let _ = fenetre.destroy();
        }
    }
}

/// Retire les bandeaux au démarrage : rien ne justifie qu'il en reste un d'avant.
pub fn nettoyer_au_demarrage(app: &tauri::AppHandle) {
    balayer(app, None);
}

/// Affiche un bandeau. Sans effet si la fenêtre ne peut pas être créée — une notification
/// ratée ne doit jamais remonter comme une erreur, mais elle est toujours écrite au journal.
pub fn show(app: &tauri::AppHandle, titre: &str, corps: &str) {
    let generation = {
        let etat = app.state::<Toasts>();
        // Un verrou empoisonné rendait `show()` muet : plus aucun bandeau, plus aucune
        // trace. Le compteur n'est qu'un entier — on le récupère tel quel.
        let mut n = etat.0.lock().unwrap_or_else(|e| e.into_inner());
        *n += 1;
        *n
    };

    let charge = serde_json::to_string(&Contenu { titre, corps }).unwrap_or_else(|_| "{}".into());
    let script = format!("window.__TOAST__ = {charge};");
    let mien = label(generation);
    let resume = format!("{titre} — {corps}");

    // 🔑 TOUT se passe sur un fil dédié, jamais sur le fil principal.
    //
    // C'est LE défaut qui a fait perdre une soirée : appelée depuis une commande Tauri
    // synchrone, `show()` s'exécute sur le fil principal. Y construire une fenêtre WebView2
    // fige la boucle d'évènements — la création attend une réponse que seule cette boucle
    // pourrait délivrer. Résultat observé : la fenêtre existait (créée, invisible, jamais
    // positionnée), l'interface devenait blanche, et le code suivant ne s'exécutait
    // jamais — pas même la ligne de journal censée dire que ça avait marché.
    //
    // Depuis un autre fil, Tauri fait lui-même l'aller-retour vers la boucle sans la
    // bloquer. C'est la seule façon correcte de créer une fenêtre à la demande.
    let travail = app.clone();
    std::thread::spawn(move || {
        let noter = |message: String| {
            if let Ok(dir) = travail.path().app_config_dir() {
                journal::write(&dir, "BANDEAU", &message);
            }
        };
        noter(format!("nº {generation} : construction — {resume}"));

        let fenetre = tauri::WebviewWindowBuilder::new(
            &travail,
            &mien,
            tauri::WebviewUrl::App("toast.html".into()),
        )
        .initialization_script(&script)
        .inner_size(LARGEUR, HAUTEUR)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        // Ne jamais prendre le focus : le bandeau s'affiche pendant que la personne joue
        // ou écrit ailleurs.
        .focused(false)
        .resizable(false)
        .shadow(true)
        // Créée invisible : on la place d'abord, on la montre ensuite. Sinon elle
        // apparaît une fraction de seconde en haut à gauche avant de sauter à sa place.
        .visible(false)
        .build();

        let fenetre = match fenetre {
            Ok(f) => f,
            Err(e) => return noter(format!("nº {generation} : création impossible : {e}")),
        };

        // Tout bandeau plus ancien disparaît, quelle qu'en soit la raison.
        balayer(&travail, Some(&mien));

        // L'écran de référence : celui où se trouve la fenêtre principale — c'est là que
        // la personne regarde. À défaut, l'écran principal.
        let ecran = travail
            .get_webview_window("main")
            .and_then(|m| m.current_monitor().ok().flatten())
            .or_else(|| travail.primary_monitor().ok().flatten());

        match ecran {
            Some(ecran) => {
                // 🔑 L'origine de l'écran compte. Sur un montage à plusieurs écrans, celui
                // de gauche a des coordonnées NÉGATIVES : ignorer `position()` envoyait le
                // bandeau sur l'écran d'à côté, hors de vue.
                let echelle = ecran.scale_factor();
                let origine = ecran.position();
                let taille = ecran.size();
                let x = origine.x as f64 + taille.width as f64 - (LARGEUR + MARGE) * echelle;
                let y = origine.y as f64 + MARGE * echelle;
                let _ = fenetre.set_position(tauri::PhysicalPosition::new(x, y));
                noter(format!("nº {generation} : placé en {x:.0},{y:.0}"));
            }
            None => noter(format!("nº {generation} : écran introuvable, position par défaut")),
        }

        match fenetre.show() {
            Ok(()) => noter(format!("nº {generation} : affiché")),
            Err(e) => noter(format!("nº {generation} : affichage impossible : {e}")),
        }

        std::thread::sleep(std::time::Duration::from_secs(DUREE_SECS));

        // Chacun ferme le sien : plus de condition, donc plus de bandeau orphelin.
        match fenetre.destroy() {
            Ok(()) => noter(format!("nº {generation} : fermé")),
            Err(e) => noter(format!("nº {generation} : échec de la fermeture : {e}")),
        }
    });
}
