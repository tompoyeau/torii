//! Veille de la bibliothèque : la tient à jour sans qu'on ait à la resynchroniser.
//!
//! Deux sources, deux rythmes :
//! - **Ce qui se lit sur cette machine** (jeux installés, cache Ubisoft, instantanés EA
//!   et Battle.net) : relu toutes les 10 s, pour quelques millisecondes. Un jeu installé,
//!   désinstallé ou dont le téléchargement se termine apparaît sans rien demander à
//!   personne : on recompose avec les possédés du dernier relevé en ligne.
//! - **Ce qu'il faut demander en ligne** (possédés Steam, GOG, Epic) : au démarrage et à F5
//!   (relevé demandé par l'interface), puis au retour sur la fenêtre, à la fermeture de la
//!   page web intégrée et toutes les 30 min (relevés automatiques). C'est ce qui fait
//!   apparaître un jeu acheté.
//!
//! Un relevé automatique publie la nouvelle bibliothèque par l'événement
//! `library-updated` ; le front la fond dans la sienne sans écran de chargement.

use crate::models::GameDto;
use crate::platforms::{self, Empreinte, ReleveLocal};
use crate::{journal, libsync, procwatch, LastScan};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Relecture de ce qui est installé.
const INTERVALLE_LOCAL: Duration = Duration::from_secs(10);
/// Relevé en ligne de fond, fenêtre ouverte ou non : un jeu acheté ailleurs finit par
/// arriver, et la bibliothèque que voient les amis avec lui.
const INTERVALLE_EN_LIGNE: Duration = Duration::from_secs(30 * 60);
/// Retour sur la fenêtre : on revient souvent de Steam ou du navigateur, où l'on vient
/// peut-être d'acheter. Mais un aller-retour entre deux fenêtres ne vaut pas un relevé.
const ECART_AU_RETOUR: Duration = Duration::from_secs(2 * 60);
/// Page web refermée : un achat a pu s'y conclure. On ne relève pas à chaque page.
const ECART_APRES_PAGE: Duration = Duration::from_secs(20);

/// 🔑 Un seul relevé à la fois, local compris. Deux relevés en ligne simultanés grillent
/// le jeton GOG (il tourne à chaque usage) ; et un relevé local qui finirait après un
/// relevé en ligne republierait les possédés d'avant.
static RELEVE: Mutex<()> = Mutex::new(());

struct Memoire {
    /// Possédés rapportés par le dernier relevé en ligne. `None` tant qu'il n'y en a pas
    /// eu : la veille n'a alors rien à recomposer, le premier scan est en route.
    en_ligne: Option<Vec<GameDto>>,
    /// Empreinte du relevé local qui a servi à la bibliothèque publiée.
    empreinte: Empreinte,
    dernier_en_ligne: Option<Instant>,
}

static MEMOIRE: Mutex<Memoire> = Mutex::new(Memoire {
    en_ligne: None,
    empreinte: Vec::new(),
    dernier_en_ligne: None,
});

fn memoire() -> MutexGuard<'static, Memoire> {
    MEMOIRE.lock().unwrap_or_else(|p| p.into_inner())
}

/// Qui a voulu ce relevé : ça décide s'il attend son tour et s'il fait foi.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Origine {
    /// L'interface (démarrage, F5, connexion ou déconnexion d'un compte) : attend que le
    /// relevé en cours se termine, et son résultat fait foi.
    Demande,
    /// Torii lui-même, pour la raison donnée (notée au journal) : renonce si un relevé
    /// tourne déjà, et ne laisse pas un compte devenu muet vider l'écran (voir
    /// [`garder_les_muets`]).
    Auto(&'static str),
}

fn prendre_la_main(origine: Origine) -> Option<MutexGuard<'static, ()>> {
    match origine {
        Origine::Demande => Some(RELEVE.lock().unwrap_or_else(|p| p.into_inner())),
        Origine::Auto(_) => match RELEVE.try_lock() {
            Ok(verrou) => Some(verrou),
            Err(TryLockError::Poisoned(p)) => Some(p.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        },
    }
}

/// Démarre la veille : un fil qui dort presque tout le temps.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(INTERVALLE_LOCAL);
        if depuis_le_dernier_en_ligne().is_some_and(|d| d >= INTERVALLE_EN_LIGNE) {
            relever_en_ligne(&app, Origine::Auto("relevé périodique"));
        } else {
            verifier_local(&app);
        }
    });
}

/// Temps écoulé depuis le dernier relevé en ligne (`None` : il n'y en a pas encore eu).
fn depuis_le_dernier_en_ligne() -> Option<Duration> {
    memoire().dernier_en_ligne.map(|t| t.elapsed())
}

/// La fenêtre principale reprend le focus.
pub fn au_retour(app: &AppHandle) {
    relever_en_fond_apres(app, ECART_AU_RETOUR, "retour sur la fenêtre");
}

/// La page web intégrée vient d'être refermée.
pub fn apres_page_web(app: &AppHandle) {
    relever_en_fond_apres(app, ECART_APRES_PAGE, "page web refermée");
}

/// Relevé automatique dans un fil à part, si le dernier date d'au moins `ecart`.
/// Appelé depuis le thread principal (événements de fenêtre) : ne bloque jamais.
fn relever_en_fond_apres(app: &AppHandle, ecart: Duration, pourquoi: &'static str) {
    if !depuis_le_dernier_en_ligne().is_some_and(|d| d >= ecart) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        relever_en_ligne(&app, Origine::Auto(pourquoi));
    });
}

/// Relevé complet, comptes en ligne compris. `None` si c'est un relevé automatique et
/// qu'un autre tournait déjà (celui-là publiera).
pub fn relever_en_ligne(app: &AppHandle, origine: Origine) -> Option<Vec<GameDto>> {
    let _verrou = prendre_la_main(origine)?;
    let debut = Instant::now();
    let dir = app.path().app_config_dir().ok();
    // Les tailles se remesurent à chaque relevé en ligne : celle d'un jeu mis à jour suit.
    platforms::oublier_tailles();

    let mut en_ligne = dir.as_deref().map(crate::accounts::owned_online).unwrap_or_default();
    if let Origine::Auto(_) = origine {
        let precedent = memoire().en_ligne.clone().unwrap_or_default();
        let (gardes, muets) = garder_les_muets(&precedent, en_ligne);
        en_ligne = gardes;
        if !muets.is_empty() {
            if let Some(dir) = dir.as_deref() {
                let quoi = format!("relevé automatique : aucun jeu rendu par {}, liste précédente gardée", muets.join(", "));
                journal::write(dir, "INFO", &quoi);
            }
        }
    }

    let local = ReleveLocal::lire(dir.as_deref());
    let empreinte = local.empreinte();
    let games = platforms::assembler(local, &en_ligne, dir.as_deref());
    {
        let mut m = memoire();
        m.en_ligne = Some(en_ligne);
        m.empreinte = empreinte;
        m.dernier_en_ligne = Some(Instant::now());
    }
    let auto = matches!(origine, Origine::Auto(_));
    if let (Origine::Auto(pourquoi), Some(dir)) = (origine, dir.as_deref()) {
        let quoi = format!("relevé en ligne ({pourquoi}) : {} jeux en {:.1} s", games.len(), debut.elapsed().as_secs_f32());
        journal::write(dir, "INFO", &quoi);
    }
    publier(app, dir.as_deref(), &games, auto);
    Some(games)
}

/// Un passage de la veille locale : recompose et publie si quelque chose a bougé.
fn verifier_local(app: &AppHandle) {
    // Un relevé tourne : c'est lui qui publiera l'état à jour.
    let Some(_verrou) = prendre_la_main(Origine::Auto("veille locale")) else { return };
    if memoire().en_ligne.is_none() {
        return;
    }
    let dir = app.path().app_config_dir().ok();
    let local = ReleveLocal::lire(dir.as_deref());
    let empreinte = local.empreinte();

    let (en_ligne, avant) = {
        let mut m = memoire();
        if m.empreinte == empreinte {
            return;
        }
        let avant = std::mem::replace(&mut m.empreinte, empreinte.clone());
        (m.en_ligne.clone().unwrap_or_default(), avant)
    };
    if let Some(dir) = dir.as_deref() {
        journal::write(dir, "INFO", &format!("bibliothèque : {}", changements(&avant, &empreinte)));
    }
    let games = platforms::assembler(local, &en_ligne, dir.as_deref());
    publier(app, dir.as_deref(), &games, true);
}

/// Ce qui a changé entre deux empreintes, pour le journal (« +steam:620, -gog:1207 »).
fn changements(avant: &Empreinte, apres: &Empreinte) -> String {
    let avant: BTreeSet<_> = avant.iter().collect();
    let apres: BTreeSet<_> = apres.iter().collect();
    let mut lignes: Vec<String> = apres
        .difference(&avant)
        .map(|(id, installe, _)| format!("+{id}{}", if *installe { "" } else { " (non installé)" }))
        .collect();
    lignes.extend(avant.difference(&apres).map(|(id, _, _)| format!("-{id}")));
    lignes.join(", ")
}

/// Un compte connecté qui ne rend plus aucun jeu lors d'un relevé automatique n'a pas
/// vendu sa bibliothèque : le réseau est coupé (réveil du PC), sa session a expiré… On
/// garde ce qu'il avait rendu la fois d'avant plutôt que de retirer d'un coup ses
/// centaines de jeux de l'écran. Renvoie aussi les plateformes ainsi conservées.
///
/// ⚠️ Réservé aux relevés automatiques. Un relevé demandé (F5, connexion, déconnexion)
/// fait foi : c'est par lui qu'un compte déconnecté perd ses jeux.
fn garder_les_muets(precedent: &[GameDto], mut frais: Vec<GameDto>) -> (Vec<GameDto>, Vec<String>) {
    let muets: Vec<String> = {
        let presentes: BTreeSet<&str> = frais.iter().map(|g| g.platform.as_str()).collect();
        let avant: BTreeSet<&str> = precedent.iter().map(|g| g.platform.as_str()).collect();
        avant.difference(&presentes).map(|p| p.to_string()).collect()
    };
    frais.extend(precedent.iter().filter(|g| muets.contains(&g.platform)).cloned());
    (frais, muets)
}

/// Rend une nouvelle bibliothèque à tous ceux qui en dépendent : cache disque (affiché
/// au prochain démarrage), enrichissement IGDB (`LastScan`), surveillant de process,
/// synchronisation vers le serveur, et l'interface si elle ne l'a pas demandée.
fn publier(app: &AppHandle, dir: Option<&Path>, games: &[GameDto], vers_l_interface: bool) {
    if let Some(dir) = dir {
        platforms::library_cache::save(dir, games);
    }
    if let Ok(mut slot) = app.state::<LastScan>().0.lock() {
        *slot = games.to_vec();
    }
    // Un jeu installé depuis le dernier relevé devient détectable immédiatement.
    procwatch::set_targets(app, games);

    // Si la synchronisation est active et que quelque chose a bougé, on dépose la
    // bibliothèque. En tâche de fond et sans jamais faire échouer le relevé : `sync` ne
    // renvoie une erreur que si l'envoi lui-même a échoué (éteint ou déconnecté sont des
    // situations normales, pas des pannes).
    if let Some(dir) = dir {
        let dir = dir.to_path_buf();
        let a_deposer = games.to_vec();
        std::thread::spawn(move || match libsync::sync(&dir, &a_deposer, false) {
            Ok(r) if r.uploaded => {
                journal::write(&dir, "INFO", &format!("bibliothèque synchronisée ({} jeux)", r.game_count));
            }
            Err(e) => journal::write(&dir, "ERREUR", &format!("synchronisation de la bibliothèque : {e}")),
            _ => {}
        });
    }

    if vers_l_interface {
        let _ = app.emit("library-updated", games);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jeu(id: &str) -> GameDto {
        GameDto {
            id: id.into(),
            platform: id.split(':').next().unwrap_or_default().into(),
            ..Default::default()
        }
    }

    fn ids(jeux: &[GameDto]) -> Vec<&str> {
        let mut ids: Vec<&str> = jeux.iter().map(|g| g.id.as_str()).collect();
        ids.sort();
        ids
    }

    #[test]
    fn un_compte_muet_garde_ses_jeux() {
        let precedent = [jeu("steam:1"), jeu("steam:2"), jeu("gog:3")];
        let (jeux, muets) = garder_les_muets(&precedent, vec![jeu("gog:3")]);
        assert_eq!(ids(&jeux), ["gog:3", "steam:1", "steam:2"]);
        assert_eq!(muets, ["steam"]);
    }

    #[test]
    fn un_compte_qui_repond_fait_foi() {
        // Un jeu remboursé disparaît : le compte a répondu, sa liste est la bonne.
        let precedent = [jeu("steam:1"), jeu("steam:2")];
        let (jeux, muets) = garder_les_muets(&precedent, vec![jeu("steam:2"), jeu("steam:9")]);
        assert_eq!(ids(&jeux), ["steam:2", "steam:9"]);
        assert!(muets.is_empty());
    }

    #[test]
    fn premier_releve_sans_precedent() {
        let (jeux, muets) = garder_les_muets(&[], vec![jeu("epic:a")]);
        assert_eq!(ids(&jeux), ["epic:a"]);
        assert!(muets.is_empty());
    }

    #[test]
    fn le_journal_dit_ce_qui_a_bouge() {
        let avant: Empreinte = vec![("gog:1".into(), true, "d:\\gog".into()), ("steam:1".into(), true, "e:\\a".into())];
        let apres: Empreinte = vec![("steam:1".into(), true, "e:\\a".into()), ("steam:2".into(), true, "e:\\b".into())];
        assert_eq!(changements(&avant, &apres), "+steam:2, -gog:1");
    }
}
