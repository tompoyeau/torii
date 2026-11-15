//! Lecture d'un raccourci Windows (`.lnk`) : sa cible, ses arguments, son dossier de travail.
//!
//! # Pourquoi lire le raccourci au lieu de le lancer tel quel
//!
//! Un jeu ajouté à la main l'est souvent par son raccourci du Bureau ou du menu Démarrer.
//! Le garder comme cible de lancement pose deux problèmes :
//!
//! - le suivi de session rattache un process à un jeu par **préfixe de chemin** sur son
//!   dossier (cf. `procwatch`). Le dossier d'un raccourci, c'est le Bureau : tout ce qui
//!   en démarre passerait pour ce jeu ;
//! - le jour où le raccourci est supprimé, le jeu ne se lance plus, alors que rien n'a
//!   bougé sur le disque.
//!
//! On en tire donc le vrai exécutable et ses arguments, et le raccourci ne sert plus à rien.
//!
//! # Pourquoi demander à Windows
//!
//! Le format `.lnk` est publié, et le lire à la main paraît simple : un en-tête, puis le
//! chemin en clair. Essayé sur une vraie machine, ce lecteur-là manquait un raccourci sur
//! dix — ceux que posent les installeurs de Battle.net, Riot ou Rockstar, dont la cible
//! n'existe que sous forme de liste d'identifiants du shell, un format que seul
//! l'Explorateur documente par sa propre lecture. `IShellLinkW` **est** cette lecture : la
//! réponse est par construction celle de Windows.
//!
//! ⚠️ On lit, on ne répare pas : pas d'appel à `Resolve`, qui partirait à la recherche
//! d'une cible déplacée (et peut ouvrir une fenêtre). Une cible disparue est rendue telle
//! qu'elle est écrite ; c'est à l'appelant de vérifier qu'elle existe.

use std::path::Path;
use windows::core::{Interface, HSTRING};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

/// Ce qu'un raccourci désigne.
#[derive(Debug, PartialEq)]
pub struct Raccourci {
    /// Chemin complet du fichier visé.
    pub cible: String,
    /// Arguments passés à la cible, s'il y en a.
    pub arguments: Option<String>,
    /// Dossier de travail (« Démarrer dans »), s'il est renseigné.
    pub dossier: Option<String>,
}

/// Longueur maximale d'un chemin Windows, et bien au-delà de celle d'une ligne d'arguments.
const TAMPON: usize = 32_768;

/// Lit un fichier `.lnk`. `None` s'il est illisible ou s'il ne désigne pas un fichier :
/// application du Microsoft Store, raccourci « annoncé » d'un installeur MSI…
pub fn lire(chemin: &Path) -> Option<Raccourci> {
    // SAFETY : COM est initialisé sur ce fil avant tout appel et libéré après, uniquement
    // si c'est nous qui l'avons initialisé. Un fil déjà initialisé dans l'autre mode
    // renvoie une erreur ici, mais COM y reste utilisable : on continue sans libérer.
    unsafe {
        let initialise = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let lu = interroger(chemin);
        if initialise.is_ok() {
            CoUninitialize();
        }
        lu
    }
}

/// SAFETY : COM doit être initialisé sur le fil appelant.
unsafe fn interroger(chemin: &Path) -> Option<Raccourci> {
    let lien: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
    let fichier: IPersistFile = lien.cast().ok()?;
    fichier.Load(&HSTRING::from(chemin.as_os_str()), STGM_READ).ok()?;

    let mut tampon = vec![0u16; TAMPON];
    let mut lire_champ = |remplir: &dyn Fn(&mut [u16]) -> windows::core::Result<()>| -> Option<String> {
        tampon.fill(0);
        remplir(&mut tampon).ok()?;
        let fin = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
        let texte = String::from_utf16_lossy(&tampon[..fin]).trim().to_string();
        (!texte.is_empty()).then_some(texte)
    };

    let cible = lire_champ(&|t| lien.GetPath(t, std::ptr::null_mut(), 0))?;
    let arguments = lire_champ(&|t| lien.GetArguments(t));
    let dossier = lire_champ(&|t| lien.GetWorkingDirectory(t));
    Some(Raccourci { cible, arguments, dossier })
}

/// Pour les tests (ici et dans `manual`) : fait écrire un vrai raccourci par Windows,
/// puisque c'est lui qui en fixe le format.
#[cfg(test)]
pub(super) fn creer(lnk: &Path, cible: &str, arguments: Option<&str>, dossier: Option<&str>) {
    unsafe {
        let initialise = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let lien: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
        lien.SetPath(&HSTRING::from(cible)).unwrap();
        if let Some(a) = arguments {
            lien.SetArguments(&HSTRING::from(a)).unwrap();
        }
        if let Some(d) = dossier {
            lien.SetWorkingDirectory(&HSTRING::from(d)).unwrap();
        }
        let fichier: IPersistFile = lien.cast().unwrap();
        fichier.Save(&HSTRING::from(lnk.as_os_str()), true).unwrap();
        drop((fichier, lien));
        if initialise.is_ok() {
            CoUninitialize();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn bac(nom: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("torii-raccourci-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lit_la_cible_le_dossier_et_les_arguments() {
        let dir = bac("complet");
        let exe = dir.join("Dolphin.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let lnk = dir.join("Zelda.lnk");
        let arguments = r#"--exec="E:\Roms\zelda.iso" --batch"#;
        creer(&lnk, &exe.to_string_lossy(), Some(arguments), Some(&dir.to_string_lossy()));

        let lu = lire(&lnk).unwrap();
        assert!(lu.cible.eq_ignore_ascii_case(&exe.to_string_lossy()), "{}", lu.cible);
        assert_eq!(lu.arguments.as_deref(), Some(arguments));
        assert!(lu.dossier.unwrap().eq_ignore_ascii_case(&dir.to_string_lossy()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Ni arguments ni dossier : `None`, pas des chaînes vides. Et un nom hors ASCII passe.
    #[test]
    fn un_raccourci_nu_n_a_que_sa_cible() {
        let dir = bac("nu");
        let exe = dir.join("Hadès Édition 原神.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let lnk = dir.join("Hadès.lnk");
        creer(&lnk, &exe.to_string_lossy(), None, None);

        let lu = lire(&lnk).unwrap();
        assert!(lu.cible.ends_with("Hadès Édition 原神.exe"), "{}", lu.cible);
        assert_eq!(lu.arguments, None);
        assert_eq!(lu.dossier, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Une cible disparue est rendue telle qu'elle est écrite : on lit, on ne répare pas.
    #[test]
    fn une_cible_disparue_reste_lisible() {
        let dir = bac("disparue");
        let exe = dir.join("parti.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let lnk = dir.join("Parti.lnk");
        creer(&lnk, &exe.to_string_lossy(), None, None);
        std::fs::remove_file(&exe).unwrap();

        let lu = lire(&lnk).unwrap();
        assert!(lu.cible.to_lowercase().ends_with("parti.exe"), "{}", lu.cible);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_ce_qui_n_est_pas_un_raccourci() {
        let dir = bac("faux");
        let faux = dir.join("faux.lnk");
        std::fs::write(&faux, b"[InternetShortcut]\r\nURL=steam://rungameid/620\r\n").unwrap();
        assert_eq!(lire(&faux), None);
        assert_eq!(lire(&dir.join("absent.lnk")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
