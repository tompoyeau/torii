//! Journal de bord de l'application.
//!
//! Torii tourne chez des gens dont on ne voit pas l'écran. Sans trace écrite, un
//! « ça a planté » ne laisse **rien** à lire : c'est exactement ce qui s'est produit
//! avec la fermeture intempestive après connexion Steam, trouvée par relecture de code
//! faute de mieux.
//!
//! Ce module écrit trois choses dans `logs/torii.log` :
//!   * une ligne au démarrage (version, date) — deux démarrages sans arrêt entre les
//!     deux signalent une fin brutale, même quand rien n'a pu être écrit ;
//!   * les **paniques Rust**, avec leur emplacement et la pile d'appels ;
//!   * les erreurs remontées par l'interface (page blanche, promesse rejetée).
//!
//! ⚠️ Ce qu'il ne capte PAS : un arrêt violent du process (violation d'accès, plantage
//! de WebView2, `TerminateProcess`). Aucun code Rust ne s'exécute alors. Le signe, dans
//! ce cas, est l'absence de ligne d'arrêt avant le démarrage suivant.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Au-delà, le journal est archivé en `.1` et un nouveau commence. Assez grand pour
/// contenir des semaines d'usage, assez petit pour être envoyé par message.
const MAX_BYTES: u64 = 512 * 1024;

pub fn dir(config_dir: &Path) -> PathBuf {
    config_dir.join("logs")
}

/// Chemin du journal courant.
pub fn path(config_dir: &Path) -> PathBuf {
    dir(config_dir).join("torii.log")
}

/// Écrit une ligne horodatée. Best-effort : journaliser ne doit jamais faire échouer
/// quoi que ce soit, ni paniquer (on serait alors appelé depuis le gestionnaire de
/// panique, et une panique dans une panique abrège le process).
pub fn write(config_dir: &Path, niveau: &str, message: &str) {
    let _ = std::fs::create_dir_all(dir(config_dir));
    let fichier = path(config_dir);

    // Rotation : un seul archivage, le précédent est remplacé.
    if std::fs::metadata(&fichier).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let _ = std::fs::rename(&fichier, dir(config_dir).join("torii.log.1"));
    }

    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&fichier) {
        let _ = writeln!(f, "{} [{}] {}", horodatage(), niveau, message);
    }
}

/// Installe le gestionnaire de paniques et note le démarrage.
///
/// À appeler **le plus tôt possible** : une panique survenue avant ne laisserait rien.
pub fn init(config_dir: PathBuf) {
    write(
        &config_dir,
        "INFO",
        &format!("démarrage de Torii {}", env!("CARGO_PKG_VERSION")),
    );

    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let fil = std::thread::current();
        let nom = fil.name().unwrap_or("(sans nom)").to_string();
        let ou = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "emplacement inconnu".into());
        // Le message d'une panique est soit un `&str`, soit une `String`.
        let quoi = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(message illisible)".into());

        write(
            &config_dir,
            "PANIQUE",
            &format!(
                "fil « {nom} » à {ou} : {quoi}\n{}",
                std::backtrace::Backtrace::force_capture()
            ),
        );
        // On laisse le gestionnaire d'origine faire son travail (affichage console en dev).
        precedent(info);
    }));
}

/// Horodatage local lisible : `2026-08-21 14:07:33`.
///
/// Écrit à la main plutôt qu'avec une bibliothèque de dates : c'est la seule chose dont
/// on a besoin, et une dépendance de plus pour formater une date serait mal placée dans
/// un module dont le rôle est de fonctionner quand tout le reste va mal.
fn horodatage() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + decalage_local_secs();

    let (annee, mois, jour) = civil_depuis_jours(secs.div_euclid(86_400));
    let reste = secs.rem_euclid(86_400);
    format!(
        "{annee:04}-{mois:02}-{jour:02} {:02}:{:02}:{:02}",
        reste / 3600,
        (reste % 3600) / 60,
        reste % 60
    )
}

/// Décalage du fuseau local, en secondes. Zéro si Windows ne répond pas : une heure
/// décalée reste plus utile qu'une absence de date.
#[cfg(windows)]
fn decalage_local_secs() -> i64 {
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemTime(t: *mut SystemTime);
        fn GetLocalTime(t: *mut SystemTime);
    }

    unsafe {
        let (mut utc, mut local) = (SystemTime::default(), SystemTime::default());
        GetSystemTime(&mut utc);
        GetLocalTime(&mut local);
        let en_secs = |t: &SystemTime| {
            i64::from(t.hour) * 3600 + i64::from(t.minute) * 60 + i64::from(t.second)
        };
        let mut diff = en_secs(&local) - en_secs(&utc);
        // Passage de minuit entre les deux lectures : on ramène dans ±12 h.
        if diff > 43_200 {
            diff -= 86_400;
        } else if diff < -43_200 {
            diff += 86_400;
        }
        diff
    }
}

#[cfg(not(windows))]
fn decalage_local_secs() -> i64 {
    0
}

/// Jours depuis l'époque Unix → (année, mois, jour). Algorithme « days_from_civil »
/// d'Howard Hinnant, pris à l'envers — le même que celui déjà utilisé pour les dates GOG.
fn civil_depuis_jours(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_de_date() {
        // 1er janvier 1970, puis une date connue : 21 août 2026.
        assert_eq!(civil_depuis_jours(0), (1970, 1, 1));
        assert_eq!(civil_depuis_jours(20_686), (2026, 8, 21));
    }

    #[test]
    fn ecrit_et_tourne() {
        let dir = std::env::temp_dir().join(format!("torii-journal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        write(&dir, "INFO", "première ligne");
        let contenu = std::fs::read_to_string(path(&dir)).unwrap();
        assert!(contenu.contains("[INFO] première ligne"));
        assert!(contenu.starts_with("20"), "la ligne doit commencer par la date");

        // Au-delà du seuil, le journal est archivé et un neuf commence.
        std::fs::write(path(&dir), vec![b'x'; (MAX_BYTES + 1) as usize]).unwrap();
        write(&dir, "INFO", "après rotation");
        let neuf = std::fs::read_to_string(path(&dir)).unwrap();
        assert!(neuf.contains("après rotation"));
        assert!(neuf.len() < 500, "le nouveau journal repart de zéro");
        assert!(super::dir(&dir).join("torii.log.1").exists(), "l'ancien est archivé");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
