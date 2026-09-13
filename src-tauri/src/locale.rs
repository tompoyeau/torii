//! Langue et région commerciale, côté natif.
//!
//! 🔑 **POURQUOI UN ÉTAT GLOBAL, ET NON UN `State` TAURI.** Les paramètres de langue
//! partent dans des requêtes HTTP émises au fond de `metadata/` et de `platforms/` —
//! des fonctions libres, appelées depuis des tâches de fond, parfois plusieurs niveaux
//! sous la commande qui les a déclenchées. Faire descendre un `&State` jusque-là
//! obligerait à modifier la signature d'une trentaine de fonctions dont aucune ne parle
//! de Tauri, uniquement pour transporter deux chaînes qui ne changent jamais en cours
//! de requête. Le verrou global dit la même chose sans contaminer les signatures.
//!
//! ⚠️ **CE N'EST PAS LA SOURCE DE VÉRITÉ.** Elle est dans `localStorage`, côté interface,
//! avec les autres préférences. Ce module en est une copie que le front pousse au
//! démarrage et à chaque changement (commande `set_locale`).
//!
//! 🔑 **L'ANGLAIS PAR DÉFAUT, SAUF POUR QUI UTILISAIT DÉJÀ TORII.** Une nouvelle
//! installation démarre en anglais, quelle que soit la langue de Windows. Mais Torii a été
//! français pendant vingt versions, sans aucun réglage de langue : basculer ses
//! utilisateurs en anglais au détour d'une mise à jour serait les trahir. Une installation
//! antérieure est donc reconnue au démarrage (`installation_anterieure`) et reste en
//! français. Voir `charger`.
//!
//! 🔑 **TOUTE LA LOGIQUE EST DANS `Locale`, PAS DANS LES FONCTIONS GLOBALES.** Celles du
//! bas ne font que lire le verrou et déléguer. C'est ce qui rend ce module testable : un
//! test qui écrirait dans le verrou global changerait la langue des autres tests, qui
//! tournent en parallèle dans le même processus — un cache de métadonnées lu au mauvais
//! moment pointerait alors sur le fichier anglais. Les tests ci-dessous n'instancient
//! donc que des `Locale` bien à eux.

use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Langue {
    Fr,
    En,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locale {
    pub langue: Langue,
    /// Code ISO 3166-1 alpha-2. Vide = jamais renseigné → `region()` répond `US`.
    region: String,
}

impl Locale {
    /// Le défaut d'une installation neuve : l'anglais. La région vide retombe sur `US`
    /// dans `region()` — elle n'est de toute façon lue qu'après que l'interface a poussé
    /// celle de Windows.
    const fn defaut() -> Self {
        Locale {
            langue: Langue::En,
            region: String::new(),
        }
    }

    /// Interprète ce que l'interface a envoyé.
    ///
    /// Tout ce qui n'est pas reconnu retombe sur l'anglais, jamais sur une erreur : une
    /// préférence illisible ne doit pas empêcher l'application de marcher.
    pub fn analyser(langue: &str, region: &str) -> Self {
        let region = region.trim().to_ascii_uppercase();
        Locale {
            langue: match langue {
                "fr" => Langue::Fr,
                _ => Langue::En,
            },
            region: if region.len() == 2 && region.chars().all(|c| c.is_ascii_uppercase()) {
                region
            } else {
                String::new()
            },
        }
    }

    /// Le pays de tarification. `US` si aucun n'a été reçu — le repli le plus lisible
    /// à l'international (voir `regionDuSysteme` côté interface, qui fait le même choix).
    pub fn region(&self) -> &str {
        if self.region.is_empty() {
            "US"
        } else {
            &self.region
        }
    }

    /// Valeur du paramètre `l=` du Steam Store (`appdetails`, `storesearch`, succès).
    pub fn steam_langue(&self) -> &'static str {
        match self.langue {
            Langue::Fr => "french",
            Langue::En => "english",
        }
    }

    /// Valeur du paramètre `locale=` de l'API GOG.
    pub fn gog_locale(&self) -> &'static str {
        match self.langue {
            Langue::Fr => "fr-FR",
            Langue::En => "en-US",
        }
    }

    /// Instant Gaming affiche des prix en euros, sans équivalent local.
    ///
    /// 🔑 Hors zone euro, on ne le propose pas : une ligne en euros au milieu d'une liste
    /// en livres ou en dollars ne se compare à rien, et c'est la comparaison qu'on vient
    /// chercher. Mieux vaut une offre de moins qu'une offre trompeuse. L'interface
    /// explique l'absence à l'écran plutôt que de laisser croire à une panne.
    ///
    /// ⚠️ « Zone euro » au sens du COMPARATEUR, pas de la monnaie officielle : il tarifie la
    /// Suisse, la Suède, la Norvège, le Danemark ou la Tchéquie en euros (relevé du
    /// 13 septembre 2026). C'est la devise des autres offres qui compte, puisque c'est à
    /// elles qu'Instant Gaming est comparé. Liste identique à `REGIONS` dans
    /// `src/i18n/regions.ts`.
    pub fn zone_euro(&self) -> bool {
        matches!(
            self.region(),
            "AT" | "BE" | "BG" | "CH" | "CY" | "CZ" | "DE" | "DK" | "EE" | "ES"
                | "FI" | "FR" | "GR" | "HR" | "HU" | "IE" | "IT" | "LT" | "LU" | "LV"
                | "MT" | "NL" | "NO" | "PT" | "RO" | "SE" | "SI" | "SK"
        )
    }
}

// `String::new()` est `const` : le verrou s'initialise sans `OnceLock` ni `lazy_static`.
static LOCALE: RwLock<Locale> = RwLock::new(Locale::defaut());

/// Applique ce que l'interface a choisi.
pub fn definir(langue: &str, region: &str) {
    if let Ok(mut verrou) = LOCALE.write() {
        *verrou = Locale::analyser(langue, region);
    }
}

/* ── Persistance ────────────────────────────────────────────────────────────── */

/// Ce qui est écrit sur le disque : exactement ce que l'interface a envoyé.
#[derive(serde::Serialize, serde::Deserialize)]
struct SurDisque {
    langue: String,
    region: String,
    /// Le réglage tel que l'utilisateur l'a laissé (`None` = « suivre Windows »), absent des
    /// fichiers écrits par la 0.21.0. Voir `choix_retenu`.
    #[serde(default)]
    choix: Option<Choix>,
}

/// Le réglage de langue et de région tel qu'il apparaît dans les Paramètres — pas la valeur
/// résolue : `"system"` et `None` y restent tels quels.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Choix {
    pub language: Option<String>,
    pub region: Option<String>,
}

fn fichier(config_dir: &std::path::Path) -> std::path::PathBuf {
    config_dir.join("locale.json")
}

/// Applique et retient la langue et la région.
///
/// 🔑 POURQUOI RUST GARDE UNE COPIE SUR LE DISQUE, alors que la source de vérité est dans
/// l'interface. Certaines choses démarrent AVANT que la fenêtre ait chargé son script et
/// pu appeler `set_locale` : le menu de la zone de notification est construit dans
/// `setup`, la présence part en tâche de fond, et avec « Démarrer minimisé » la fenêtre
/// peut ne jamais s'afficher de la session. Sans cette copie, un utilisateur anglophone
/// verrait « Quitter » dans son menu jusqu'au premier affichage de la fenêtre.
///
/// ⚠️ Une écriture ratée n'est pas une erreur : la langue est appliquée en mémoire, et
/// l'interface la renverra au prochain démarrage de toute façon.
pub fn definir_et_retenir(config_dir: &std::path::Path, langue: &str, region: &str, choix: Option<Choix>) {
    definir(langue, region);
    let contenu = SurDisque { langue: langue.to_string(), region: region.to_string(), choix };
    if let Ok(json) = serde_json::to_string(&contenu) {
        let _ = std::fs::create_dir_all(config_dir);
        let _ = std::fs::write(fichier(config_dir), json);
    }
}

/// Le réglage retenu par la dernière session, s'il a été transmis.
///
/// 🔑 POURQUOI L'INTERFACE LE RELIT ICI plutôt que dans son `localStorage`. Après un
/// redémarrage demandé depuis les Paramètres, WebView2 a relu un `localStorage` antérieur
/// au changement : l'utilisateur passait en français, redémarrait, et retrouvait l'anglais.
/// Ce fichier-ci est écrit de façon synchrone par `set_locale`, avant que le bouton de
/// redémarrage ne soit seulement cliquable.
pub fn choix_retenu(config_dir: &std::path::Path) -> Option<Choix> {
    std::fs::read_to_string(fichier(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<SurDisque>(&t).ok())
        .and_then(|d| d.choix)
}

/// Une installation de Torii antérieure aux langues a-t-elle été trouvée au démarrage ?
/// Calculé une seule fois, par `charger`.
static INSTALLATION_ANTERIEURE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Fichiers qu'une version de Torii ayant tourné au moins une fois laisse forcément.
///
/// ⚠️ PAS le journal : `journal::init` le crée à chaque démarrage, y compris le tout premier.
/// Ce sont des données qui n'apparaissent qu'après un scan ou un enrichissement — une
/// installation neuve ne les a pas encore quand `charger` tourne, en tout début de `setup`.
const TRACES_ANTERIEURES: &[&str] = &[
    "library_cache_v1.json",
    "metadata_cache_v5.json",
    "igdb_meta_cache_v4.json",
    "credentials.dat",
];

/// Relit ce qui a été retenu à la session précédente. À appeler au tout début de `setup`,
/// avant que quoi que ce soit n'écrive dans le dossier de configuration.
///
/// 🔑 `locale.json` ABSENT, C'EST L'UN DE DEUX CAS, qu'on distingue par ce qui entoure :
///   - une installation neuve : rien d'autre dans le dossier → anglais ;
///   - une mise à jour depuis une version d'avant les langues : ses caches sont là →
///     **français**, la seule langue que cette personne ait jamais connue à Torii.
/// Le verdict est retenu (`installation_anterieure`) : l'interface le demande avant de
/// choisir sa langue, pour épingler le français dans ses préférences.
pub fn charger(config_dir: &std::path::Path) {
    let lu = std::fs::read_to_string(fichier(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<SurDisque>(&t).ok());
    match lu {
        Some(d) => {
            let _ = INSTALLATION_ANTERIEURE.set(false);
            definir(&d.langue, &d.region);
        }
        None => {
            let anterieure = TRACES_ANTERIEURES.iter().any(|f| config_dir.join(f).exists());
            let _ = INSTALLATION_ANTERIEURE.set(anterieure);
            definir(if anterieure { "fr" } else { "en" }, "");
        }
    }
}

/// Vrai si Torii était déjà installé avant d'avoir des langues (voir `charger`).
pub fn installation_anterieure() -> bool {
    INSTALLATION_ANTERIEURE.get().copied().unwrap_or(false)
}

/* ── Textes affichés par la couche native ──────────────────────────────────── */

/// L'interface est-elle en anglais ?
///
/// Pour les textes construits avec `format!`, qu'on ne peut pas passer à `tr` :
/// `if locale::en() { format!("{n} games") } else { format!("{n} jeux") }`.
pub fn en() -> bool {
    langue() == Langue::En
}

/// Choisit entre deux versions d'un texte que Rust affiche lui-même — menu de la zone de
/// notification, bandeaux, messages d'erreur remontés tels quels à l'écran.
///
/// 🔑 PAS DE CATALOGUE CÔTÉ RUST, ET C'EST VOULU. Ces textes sont peu nombreux, et chacun
/// n'a de sens qu'à l'endroit exact où il est émis. Les écrire en paire sur place garde la
/// traduction sous les yeux de qui modifie le message — un catalogue séparé, c'est
/// l'assurance que le français évolue et que l'anglais reste à la version d'avant.
pub fn tr(fr: &'static str, en: &'static str) -> &'static str {
    match langue() {
        Langue::Fr => fr,
        Langue::En => en,
    }
}

/// ⚠️ Un verrou empoisonné (panique d'un autre fil pendant l'écriture) ne doit pas
/// propager la panique jusqu'à un scan de bibliothèque : on retombe sur le défaut.
fn courante() -> Locale {
    LOCALE.read().map(|v| v.clone()).unwrap_or(Locale::defaut())
}

pub fn langue() -> Langue {
    courante().langue
}

pub fn region() -> String {
    courante().region().to_string()
}

pub fn steam_langue() -> &'static str {
    courante().steam_langue()
}

pub fn gog_locale() -> &'static str {
    courante().gog_locale()
}

pub fn zone_euro() -> bool {
    courante().zone_euro()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anglais_americain() {
        let l = Locale::analyser("en", "us");
        assert_eq!(l.langue, Langue::En);
        assert_eq!(l.region(), "US"); // normalisé en majuscules
        assert_eq!(l.steam_langue(), "english");
        assert_eq!(l.gog_locale(), "en-US");
        assert!(!l.zone_euro());
    }

    #[test]
    fn francais_de_france() {
        let l = Locale::analyser("fr", "FR");
        assert_eq!(l.steam_langue(), "french");
        assert_eq!(l.gog_locale(), "fr-FR");
        assert!(l.zone_euro());
    }

    /// 🔑 Le cas qui compte vraiment : langue et région ne sont pas liées. Un anglophone
    /// en Belgique doit voir une interface anglaise ET des prix en euros.
    /// Le comparateur tarifie en euros des pays hors zone euro : Instant Gaming y a sa
    /// place. Et un pays tarifé en dollars n'en a pas, même européen.
    #[test]
    fn la_zone_euro_est_celle_du_comparateur() {
        assert!(Locale::analyser("en", "CH").zone_euro(), "la Suisse est tarifée en euros");
        assert!(Locale::analyser("en", "SE").zone_euro(), "la Suède est tarifée en euros");
        assert!(!Locale::analyser("en", "GB").zone_euro(), "le Royaume-Uni est en livres");
        assert!(!Locale::analyser("en", "UA").zone_euro(), "l'Ukraine reçoit des dollars");
        assert!(!Locale::analyser("en", "").zone_euro(), "le repli américain n'est pas en euros");
    }

    #[test]
    fn langue_et_region_sont_independantes() {
        let l = Locale::analyser("en", "BE");
        assert_eq!(l.steam_langue(), "english");
        assert!(l.zone_euro(), "la Belgique reste en euros, quelle que soit la langue");
    }

    #[test]
    fn entrees_aberrantes_retombent_sur_le_defaut() {
        for (langue, region) in [("klingon", "zzzz"), ("", ""), ("FR", "f"), ("en", "12")] {
            let l = Locale::analyser(langue, region);
            assert_eq!(l.langue, Langue::En, "langue « {langue} »");
            assert_eq!(l.region(), "US", "région « {region} »");
        }
    }

    /// 🔑 Le cas qui protège les utilisateurs actuels : une installation d'avant les langues
    /// (caches présents, pas de `locale.json`) reste en français ; un dossier vide passe en
    /// anglais ; et une langue déjà retenue gagne sur tout le reste.
    ///
    /// ⚠️ Pure : elle ne passe pas par `charger`, qui écrit dans le verrou global et le
    /// `OnceLock` — les autres tests, en parallèle, en dépendent.
    #[test]
    fn une_installation_anterieure_se_reconnait_a_ses_caches() {
        let base = std::env::temp_dir().join(format!("torii-locale-{}", std::process::id()));
        let neuve = base.join("neuve");
        let ancienne = base.join("ancienne");
        std::fs::create_dir_all(&neuve).unwrap();
        std::fs::create_dir_all(ancienne.join("logs")).unwrap();
        std::fs::write(neuve.join("torii.log"), "").unwrap(); // le journal ne compte pas
        std::fs::write(ancienne.join("library_cache_v1.json"), "[]").unwrap();

        let trace = |d: &std::path::Path| TRACES_ANTERIEURES.iter().any(|f| d.join(f).exists());
        assert!(!trace(&neuve), "un dossier neuf, journal compris, n'est pas une ancienne installation");
        assert!(trace(&ancienne), "le cache de bibliothèque trahit une installation antérieure");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Le réglage relu au démarrage est celui écrit — `"system"` et « suivre Windows »
    /// compris — et un fichier de la 0.21.0, qui n'en portait pas, n'en invente aucun.
    /// Écrit le fichier à la main : `definir_et_retenir` toucherait au verrou global.
    #[test]
    fn le_choix_retenu_se_relit_tel_quel() {
        let base = std::env::temp_dir().join(format!("torii-choix-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();

        assert_eq!(choix_retenu(&base), None, "pas de fichier, pas de choix");

        std::fs::write(fichier(&base), r#"{"langue":"en","region":"US"}"#).unwrap();
        assert_eq!(choix_retenu(&base), None, "un fichier de la 0.21.0 ne porte pas de choix");

        for choix in [
            Choix { language: Some("fr".into()), region: None },
            Choix { language: Some("system".into()), region: Some("FR".into()) },
            Choix { language: None, region: None },
        ] {
            let d = SurDisque { langue: "fr".into(), region: "FR".into(), choix: Some(choix.clone()) };
            std::fs::write(fichier(&base), serde_json::to_string(&d).unwrap()).unwrap();
            assert_eq!(choix_retenu(&base), Some(choix));
        }

        let _ = std::fs::remove_dir_all(&base);
    }

    /// ⚠️ Ce test-ci touche au verrou global — il ne peut donc pas vérifier grand-chose
    /// sans perturber les autres. Il se contente du strict minimum : que l'écriture soit
    /// relue.
    ///
    /// ⚠️ IL ÉCRIT LA LANGUE PAR DÉFAUT, ET AUCUNE AUTRE. Le cache de métadonnées choisit
    /// son fichier d'après la langue globale : écrire « fr » ici ferait changer de fichier,
    /// entre deux lectures, un test de cache qui tourne en parallèle.
    #[test]
    fn le_verrou_global_se_relit() {
        definir("en", "FR");
        assert_eq!(langue(), Langue::En);
        assert_eq!(region(), "FR");
    }
}
