//! Jeux **trouvés sur ce PC** qu'aucun launcher ne fournit, proposés dans « Ajouter un jeu ».
//!
//! Deux sources, choisies après mesure sur une vraie machine :
//!
//! - **les dossiers de jeux** que l'utilisateur désigne (`E:\Games`…) : chacun de leurs
//!   sous-dossiers est un jeu possible ;
//! - **la liste de la Game Bar** (cf. [`detected`]) : les jeux que Windows a vus tourner,
//!   y compris avant l'installation de Torii. Le surveillant de process ne les adopte qu'à
//!   leur prochain lancement ; ici on peut les ajouter tout de suite.
//!
//! ⚠️ Pas les raccourcis du Bureau et du menu Démarrer : mesuré, 115 candidats après
//! filtrage, dont une demi-douzaine de jeux. Office, Discord et OBS y sont des applications
//! comme les autres, et rien de fiable ne les distingue d'un jeu.
//!
//! Rien n'est ajouté tout seul : on propose, l'utilisateur coche. Et rien ne part en
//! ligne : les fiches IGDB arrivent ensuite par la passe habituelle de la bibliothèque,
//! qui cherche douze titres par appel.

use super::{detected, id_set, normalize, sous};
use crate::models::GameDto;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Un jeu proposé.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub title: String,
    pub launch_target: String,
    /// Dossier du jeu, s'il peut être désigné sans risque pour le suivi de session.
    pub install_dir: Option<String>,
    pub source: Source,
}

/// D'où vient une proposition.
#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// Un sous-dossier d'un dossier de jeux.
    Dossier,
    /// La liste de jeux de Windows (Game Bar).
    Windows,
}

/// Ce que l'interface affiche : les propositions, les dossiers retenus, le nombre de
/// propositions écartées (pour pouvoir les réafficher).
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Propositions {
    pub jeux: Vec<Suggestion>,
    pub dossiers: Vec<String>,
    pub ignores: usize,
}

/// Les propositions du moment, sans rien écrire.
///
/// `bibliotheque` : les jeux déjà connus (dernier relevé). Les jeux manuels et détectés
/// sont relus en plus sur le disque : un ajout tout frais doit disparaître des propositions
/// même si le relevé n'a pas encore suivi.
pub fn proposer(config_dir: &Path, bibliotheque: &[GameDto]) -> Propositions {
    proposer_dans(config_dir, bibliotheque, dossiers_retenus(config_dir))
}

/// Comme [`proposer`], pour des dossiers de jeux donnés (diagnostic : `examples/suggestions.rs`).
pub fn proposer_dans(config_dir: &Path, bibliotheque: &[GameDto], dossiers: Vec<String>) -> Propositions {
    let mut connus: Vec<GameDto> = bibliotheque.to_vec();
    connus.extend(super::manual::scan(config_dir));
    connus.extend(detected::scan(config_dir));
    let deja = Connus::de(&connus);

    let ignores = id_set::SUGGESTIONS_IGNORED.load(config_dir);
    let ecarte = |exe: &str| {
        let cle = normalize(exe);
        ignores.contains(&cle) || detected::is_ignored(config_dir, exe)
    };

    let mut jeux: Vec<Suggestion> = Vec::new();
    for racine in &dossiers {
        for s in dans_le_dossier(Path::new(racine), &deja) {
            if !ecarte(&s.launch_target) {
                jeux.push(s);
            }
        }
    }
    for exe in detected::executables_de_la_game_bar() {
        if let Some(s) = de_windows(&exe, &deja) {
            if !ecarte(&s.launch_target) {
                jeux.push(s);
            }
        }
    }

    // Le même jeu par deux chemins (dossier et Game Bar) : on garde la première, celle du
    // dossier, dont le titre et l'exécutable ont été choisis avec plus de soin.
    let mut vus: HashSet<String> = HashSet::new();
    jeux.retain(|s| vus.insert(normalize(&s.launch_target)));
    let mut dirs_vus: HashSet<String> = HashSet::new();
    jeux.retain(|s| s.install_dir.as_deref().map_or(true, |d| dirs_vus.insert(normalize(d))));
    jeux.sort_by_key(|s| s.title.to_lowercase());

    Propositions { jeux, dossiers, ignores: ignores.len() }
}

/// Les dossiers de jeux retenus, dans l'ordre alphabétique.
pub fn dossiers_retenus(config_dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = id_set::GAME_FOLDERS.load(config_dir).into_iter().collect();
    v.sort_by_key(|d| d.to_lowercase());
    v
}

/// Ajoute ou retire un dossier de jeux.
pub fn retenir_dossier(config_dir: &Path, dossier: &str, retenu: bool) -> Result<(), String> {
    let dossier = dossier.trim().trim_matches('"').trim_end_matches(['\\', '/']).to_string();
    if retenu && !Path::new(&dossier).is_dir() {
        return Err(if crate::locale::en() { format!("Folder not found: {dossier}") } else { format!("Dossier introuvable : {dossier}") });
    }
    // Comparé sans la casse : on ne retient pas deux fois `E:\Games`.
    let present = id_set::GAME_FOLDERS
        .load(config_dir)
        .into_iter()
        .find(|d| normalize(d) == normalize(&dossier));
    match (retenu, present) {
        (true, None) => id_set::GAME_FOLDERS.set(config_dir, &dossier, true).map(|_| ()),
        (false, Some(d)) => id_set::GAME_FOLDERS.set(config_dir, &d, false).map(|_| ()),
        _ => Ok(()),
    }
}

/// « Ignorer » : cet exécutable ne sera plus proposé.
pub fn ignorer(config_dir: &Path, exe: &str) -> Result<(), String> {
    id_set::SUGGESTIONS_IGNORED.set(config_dir, &normalize(exe), true).map(|_| ())
}

/// Réaffiche tout ce qui avait été ignoré.
pub fn ne_plus_ignorer(config_dir: &Path) -> Result<(), String> {
    id_set::SUGGESTIONS_IGNORED.clear(config_dir).map(|_| ())
}

// --- Ce que la bibliothèque connaît déjà --------------------------------------------

/// Dossiers, exécutables et titres déjà dans la bibliothèque.
struct Connus {
    /// Dossiers d'installation et chemins de fichiers lancés, normalisés.
    racines: Vec<String>,
    /// Titres réduits à leurs lettres et chiffres.
    titres: Vec<String>,
}

impl Connus {
    fn de(jeux: &[GameDto]) -> Self {
        let mut racines = Vec::new();
        for g in jeux {
            if let Some(d) = g.install_dir.as_deref().filter(|d| !d.is_empty()) {
                racines.push(normalize(d));
            }
            // Un appid Steam ou une adresse n'est pas un chemin.
            if Path::new(&g.launch_target).is_absolute() {
                racines.push(normalize(&g.launch_target));
            }
        }
        let titres = jeux.iter().map(|g| cle(&g.title)).filter(|t| !t.is_empty()).collect();
        Connus { racines, titres }
    }

    /// Le dossier contient un jeu connu, ou vit dans le dossier d'un jeu connu.
    fn couvre_dossier(&self, dossier: &str) -> bool {
        let d = normalize(dossier);
        self.racines.iter().any(|r| sous(&d, r) || sous(r, &d))
    }

    /// L'exécutable vit dans le dossier d'un jeu connu (ou en est un).
    fn couvre_executable(&self, exe: &str) -> bool {
        let e = normalize(exe);
        self.racines.iter().any(|r| sous(&e, r))
    }

    /// Un jeu connu porte exactement ce titre.
    fn porte_le_titre(&self, titre: &str) -> bool {
        let t = cle(titre);
        !t.is_empty() && self.titres.iter().any(|x| *x == t)
    }

    /// Un jeu connu porte un titre qui contient celui-ci, ou l'inverse (au moins six
    /// lettres de part et d'autre). Réservé aux propositions de Windows, dont le titre est
    /// souvent deviné : « Rainbow Six » pour « Tom Clancy's Rainbow Six Siege », installé
    /// ailleurs par Ubisoft Connect.
    fn porte_un_titre_proche(&self, titre: &str) -> bool {
        let t = cle(titre);
        t.len() >= 6 && self.titres.iter().any(|x| x.len() >= 6 && (x.contains(&t) || t.contains(x.as_str())))
    }
}

/// Clé de comparaison d'un titre : lettres et chiffres en minuscules.
fn cle(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

// --- Source 1 : les dossiers de jeux ------------------------------------------------

/// Les jeux d'un dossier de jeux : un par sous-dossier inconnu de la bibliothèque, s'il
/// contient un exécutable de jeu.
///
/// Mesuré sur une vraie machine : sur 22 sous-dossiers, 11 déjà connus, 8 sans exécutable
/// (restes d'un jeu désinstallé, installeur, dossier de ROM) et 3 vrais jeux. Les
/// dossiers sans exécutable sont tus, pas proposés « à compléter ».
fn dans_le_dossier(racine: &Path, deja: &Connus) -> Vec<Suggestion> {
    let Ok(entrees) = std::fs::read_dir(racine) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entree in entrees.flatten() {
        let dossier = entree.path();
        if !dossier.is_dir() || detected::est_une_etagere(&dossier) {
            continue;
        }
        let dossier_txt = dossier.to_string_lossy().to_string();
        if deja.couvre_dossier(&dossier_txt) {
            continue;
        }
        let Some(exe) = executable_principal(&dossier) else { continue };
        let titre = titre_du_dossier(&dossier, &exe);
        if deja.porte_le_titre(&titre) {
            continue;
        }
        out.push(Suggestion {
            title: titre,
            launch_target: exe.to_string_lossy().to_string(),
            install_dir: Some(dossier_txt),
            source: Source::Dossier,
        });
    }
    out
}

/// Profondeur maximale de recherche sous le dossier d'un jeu
/// (`Jeu\Binaries\Win64\jeu.exe` = 3).
const PROFONDEUR_MAX: usize = 4;
/// Entrées lues au plus par dossier de jeu : un dossier de données géant ne doit pas
/// faire attendre la modale.
const ENTREES_MAX: usize = 4_000;

/// Sous-dossiers qui n'abritent jamais le jeu lui-même.
const DOSSIERS_ANNEXES: &[&str] = &[
    "redist", "_redist", "_commonredist", "commonredist", "directx", "vcredist", "dotnet",
    "prerequisites", "prereqs", "physx", "__installer", "installer", "_installer", "support",
    "md5", "crack", "_crack",
];

/// Exécutables d'accompagnement que le filtre de la détection ne connaît pas : outils de
/// contrôle d'une archive, de configuration ou de rapport.
const OUTILS: &[&str] = &[
    "quicksfv", "md5", "checksum", "verify", "benchmark", "settings", "config", "dxwebsetup",
    "prereq", "cleanup", "repair", "activation", "register", "unitycrashhandler", "cefprocess",
];

/// L'exécutable qui lance le jeu, parmi ceux de son dossier.
///
/// Ordre de préférence : un nom qui parle du jeu (comparé au dossier), puis le moins
/// profond — le lanceur à la racine plutôt que le binaire caché dans `bin\`, c'est celui
/// que vise le raccourci de l'éditeur —, puis le plus gros.
fn executable_principal(dossier: &Path) -> Option<PathBuf> {
    let cle_dossier = cle(&dossier.file_name()?.to_string_lossy());
    let mut trouves: Vec<(PathBuf, usize, u64)> = Vec::new();
    let mut lues = 0;
    ramasser(dossier, 0, &mut trouves, &mut lues);
    trouves
        .into_iter()
        .map(|(p, profondeur, taille)| {
            let c = cle(&p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
            let parle = c.len() >= 3 && (cle_dossier.contains(&c) || c.contains(&cle_dossier));
            (p, parle, profondeur, taille)
        })
        .min_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(b.3.cmp(&a.3)))
        .map(|(p, ..)| p)
}

fn ramasser(dossier: &Path, profondeur: usize, out: &mut Vec<(PathBuf, usize, u64)>, lues: &mut usize) {
    if profondeur > PROFONDEUR_MAX {
        return;
    }
    let Ok(entrees) = std::fs::read_dir(dossier) else { return };
    for entree in entrees.flatten() {
        *lues += 1;
        if *lues > ENTREES_MAX {
            return;
        }
        let chemin = entree.path();
        let Ok(meta) = entree.metadata() else { continue };
        if meta.is_dir() {
            let nom = entree.file_name().to_string_lossy().to_lowercase();
            if !DOSSIERS_ANNEXES.contains(&nom.as_str()) {
                ramasser(&chemin, profondeur + 1, out, lues);
            }
        } else if est_un_executable_de_jeu(&chemin) {
            out.push((chemin, profondeur, meta.len()));
        }
    }
}

fn est_un_executable_de_jeu(chemin: &Path) -> bool {
    let texte = chemin.to_string_lossy();
    if !detected::peut_etre_un_jeu(&texte) {
        return false;
    }
    let stem = chemin.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    !OUTILS.iter().any(|o| stem.contains(o))
}

/// Titre d'un jeu trouvé dans un dossier.
///
/// Le nom de l'exécutable l'emporte quand le dossier le contient avec du bruit en plus
/// (`LanternKeeperX7k2\Lantern Keeper.exe` → « Lantern Keeper ») : le nom d'un
/// exécutable est écrit par l'éditeur, celui d'un dossier souvent par une archive. Sinon,
/// la devinette de la détection (titre de la Game Bar, puis dossiers et exécutable).
fn titre_du_dossier(dossier: &Path, exe: &Path) -> String {
    let nom_exe = detected::nom_lisible(&exe.to_string_lossy());
    let cle_exe = cle(&nom_exe);
    let cle_dossier = cle(&dossier.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
    if cle_exe.len() >= 5 && cle_dossier.len() > cle_exe.len() && cle_dossier.contains(&cle_exe) {
        return nom_exe;
    }
    let (devine, _) = detected::decrire(&exe.to_string_lossy());
    if devine.trim().is_empty() { nom_exe } else { devine }
}

// --- Source 2 : la liste de jeux de Windows -------------------------------------------

/// Une entrée de la Game Bar, si c'est un jeu encore installé et inconnu de la bibliothèque.
fn de_windows(exe: &str, deja: &Connus) -> Option<Suggestion> {
    if !Path::new(exe).is_file() || deja.couvre_executable(exe) {
        return None;
    }
    let jeu = detected::identify(exe)?;
    if deja.porte_un_titre_proche(&jeu.title) || deja.porte_le_titre(&jeu.title) {
        return None;
    }
    // Pour un moteur partagé, `identify` donne l'exécutable lui-même comme racine : pas
    // un dossier à surveiller.
    let install_dir = jeu
        .install_dir
        .filter(|d| Path::new(d).is_dir() && !detected::est_une_etagere(Path::new(d)));
    Some(Suggestion { title: jeu.title, launch_target: exe.to_string(), install_dir, source: Source::Windows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Un dossier de jeux comme sur une vraie machine. Renvoie (bac, dossier de jeux).
    fn etagere(nom: &str) -> (PathBuf, PathBuf) {
        let bac = std::env::temp_dir().join(format!("torii-suggestions-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&bac);
        let jeux = bac.join("Games");
        fs::create_dir_all(&jeux).unwrap();
        (bac, jeux)
    }

    fn fichier(chemin: PathBuf, octets: usize) -> PathBuf {
        fs::create_dir_all(chemin.parent().unwrap()).unwrap();
        fs::write(&chemin, vec![0u8; octets]).unwrap();
        chemin
    }

    fn jeu(id: &str, titre: &str, dossier: &Path) -> GameDto {
        GameDto {
            id: id.into(),
            title: titre.into(),
            install_dir: Some(dossier.to_string_lossy().to_string()),
            launch_target: "620".into(),
            ..Default::default()
        }
    }

    #[test]
    fn propose_les_jeux_inconnus_d_un_dossier() {
        let (bac, jeux) = etagere("dossier");
        // Un jeu connu (Steam), un jeu inconnu, un reste de désinstallation, un installeur.
        fichier(jeux.join("Portal 2").join("portal2.exe"), 10);
        let sorry = fichier(jeux.join("LanternKeeperX7k2").join("Lantern Keeper.exe"), 10);
        fichier(jeux.join("LanternKeeperX7k2").join("UnityCrashHandler64.exe"), 10);
        fichier(jeux.join("Cyberpunk 2077").join("archive").join("pc").join("content.archive"), 10);
        fichier(jeux.join("Un Repack").join("setup.exe"), 10);
        fichier(jeux.join("Un Repack").join("MD5").join("QuickSFV.EXE"), 10);

        let biblio = [jeu("steam:620", "Portal 2", &jeux.join("Portal 2"))];
        let trouves = dans_le_dossier(&jeux, &Connus::de(&biblio));
        assert_eq!(trouves.len(), 1, "{trouves:?}");
        assert_eq!(trouves[0].title, "Lantern Keeper");
        assert_eq!(trouves[0].launch_target, sorry.to_string_lossy());
        assert_eq!(trouves[0].install_dir.as_deref(), Some(jeux.join("LanternKeeperX7k2").to_str().unwrap()));
        assert_eq!(trouves[0].source, Source::Dossier);
        let _ = fs::remove_dir_all(&bac);
    }

    /// Le lanceur à la racine plutôt que le binaire caché plus bas ; un nom qui parle du
    /// jeu plutôt qu'un exécutable plus gros mais anonyme.
    #[test]
    fn choisit_l_executable_qui_lance_le_jeu() {
        let (bac, jeux) = etagere("executable");
        let racine = fichier(jeux.join("Portage").join("portage.exe"), 10);
        fichier(jeux.join("Portage").join("bin").join("portage.exe"), 1000);
        assert_eq!(executable_principal(&jeux.join("Portage")), Some(racine));

        let parle = fichier(jeux.join("Moria").join("Binaries").join("Win64").join("Moria-Win64-Shipping.exe"), 10);
        fichier(jeux.join("Moria").join("Engine").join("Tool.exe"), 5000);
        assert_eq!(executable_principal(&jeux.join("Moria")), Some(parle));

        // Rien d'autre que des annexes : pas de jeu à proposer.
        fichier(jeux.join("Vide").join("_CommonRedist").join("vcredist_x64.exe"), 10);
        fichier(jeux.join("Vide").join("Redist").join("game.exe"), 10);
        assert_eq!(executable_principal(&jeux.join("Vide")), None);
        let _ = fs::remove_dir_all(&bac);
    }

    #[test]
    fn reconnait_ce_que_la_bibliotheque_a_deja() {
        let biblio = [
            jeu("ubisoft:635", "Tom Clancy's Rainbow Six Siege", Path::new(r"E:\Games\Tom Clancy's Rainbow Six Siege")),
            GameDto { id: "manual:x".into(), title: "X".into(), launch_target: r"E:\Jeux\Solo\x.exe".into(), ..Default::default() },
            jeu("steam:1", "Doom", Path::new(r"E:\SteamLibrary\steamapps\common\DOOM")),
        ];
        let c = Connus::de(&biblio);
        // Un dossier qui contient un jeu connu, ou vit dans le sien.
        assert!(c.couvre_dossier(r"E:\Games\Tom Clancy's Rainbow Six Siege"));
        assert!(c.couvre_dossier(r"E:\Jeux\Solo"));
        assert!(c.couvre_dossier(r"E:\SteamLibrary"));
        assert!(!c.couvre_dossier(r"E:\Games\Tom Clancy's Rainbow Six Siege X"));
        // Le sosie de Windows : titre deviné contenu dans celui de la bibliothèque.
        assert!(c.porte_un_titre_proche("Rainbow Six"));
        // Mais pas sous six lettres : « Doom » ne cache pas « Doom 3 ».
        assert!(!c.porte_un_titre_proche("Doom 3"));
        assert!(c.porte_le_titre("DOOM"));
    }

    #[test]
    fn retient_les_dossiers_et_les_refus() {
        let (bac, jeux) = etagere("reglages");
        let texte = jeux.to_string_lossy().to_string();
        retenir_dossier(&bac, &format!("\"{texte}\\\""), true).unwrap();
        retenir_dossier(&bac, &texte.to_uppercase(), true).unwrap();
        assert_eq!(dossiers_retenus(&bac), [texte.clone()]);
        assert!(retenir_dossier(&bac, r"Z:\nulle\part", true).is_err());

        let exe = fichier(jeux.join("Mon Jeu").join("jeu.exe"), 10);
        let p = proposer(&bac, &[]);
        assert!(p.jeux.iter().any(|s| s.launch_target == exe.to_string_lossy()));
        ignorer(&bac, &exe.to_string_lossy().to_uppercase()).unwrap();
        let p = proposer(&bac, &[]);
        assert!(!p.jeux.iter().any(|s| s.source == Source::Dossier), "{:?}", p.jeux);
        assert_eq!(p.ignores, 1);
        ne_plus_ignorer(&bac).unwrap();
        assert!(proposer(&bac, &[]).jeux.iter().any(|s| s.source == Source::Dossier));

        // Un jeu ajouté à la main disparaît des propositions, même avant le relevé suivant.
        super::super::manual::add(
            &bac,
            serde_json::from_value(serde_json::json!({ "title": "Mon Jeu", "launchTarget": exe, "installDir": jeux.join("Mon Jeu") })).unwrap(),
        )
        .unwrap();
        assert!(!proposer(&bac, &[]).jeux.iter().any(|s| s.source == Source::Dossier));

        retenir_dossier(&bac, &texte, false).unwrap();
        assert!(dossiers_retenus(&bac).is_empty());
        let _ = fs::remove_dir_all(&bac);
    }
}
