use super::{detected, normalize, raccourci};
use crate::models::GameDto;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Données fournies par l'utilisateur pour ajouter un jeu à la main.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualInput {
    pub title: String,
    /// Chemin du fichier à lancer, ou adresse (`steam://…`).
    pub launch_target: String,
    /// Arguments passés à la cible (`-windowed`, la ROM d'un émulateur…).
    #[serde(default)]
    pub launch_args: Option<String>,
    #[serde(default)]
    pub install_dir: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    /// Fiche choisie dans la recherche. Absente = on ne touche pas à ce que le jeu a déjà.
    #[serde(default)]
    pub fiche: Option<Fiche>,
}

/// Ce qu'une fiche trouvée en ligne apporte à un jeu ajouté à la main, en plus de son
/// titre et de sa jaquette (qui sont des champs du formulaire).
///
/// 🔑 Présente mais vide, elle **efface** : c'est ainsi qu'on détache un jeu d'une fiche
/// choisie par erreur.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Fiche {
    #[serde(default)]
    pub hero_url: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub developer: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub screenshots: Vec<String>,
}

fn store_path(config_dir: &Path) -> PathBuf {
    config_dir.join("manual_games.json")
}

/// Lit les jeux ajoutés manuellement.
pub fn scan(config_dir: &Path) -> Vec<GameDto> {
    let path = store_path(config_dir);
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<GameDto>>(&text).unwrap_or_default()
}

// --- Inspection d'une cible --------------------------------------------------------

/// Ce que Torii déduit d'un fichier choisi (ou déposé) pour préremplir le formulaire.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    /// Titre proposé. Vide quand rien ne permet de le deviner (une adresse).
    pub title: String,
    /// Cible à lancer : le fichier choisi, ou la cible d'un raccourci.
    pub launch_target: String,
    pub launch_args: Option<String>,
    /// Dossier du jeu, s'il peut être désigné sans risque (cf. [`detected::decrire`]).
    pub install_dir: Option<String>,
}

/// Regarde ce que l'utilisateur vient de désigner et en tire de quoi remplir le formulaire.
///
/// - **exécutable** : titre et dossier devinés comme pour un jeu détecté ;
/// - **raccourci `.lnk`** vers un exécutable : on prend sa cible et ses arguments, et le
///   raccourci ne sert plus à rien (cf. [`raccourci`]). Son nom, écrit pour être lu, fait
///   le titre ;
/// - **le reste** (`.bat`, `.url`, raccourci vers autre chose qu'un exécutable, adresse) :
///   gardé tel quel, sans dossier — rien n'y garantit que le fichier vit avec le jeu.
pub fn inspecter(chemin: &str) -> Result<Inspection, String> {
    let chemin = sans_guillemets(chemin);
    verifier_cible(&chemin)?;
    if est_une_adresse(&chemin) {
        return Ok(Inspection { title: String::new(), launch_target: chemin, launch_args: None, install_dir: None });
    }

    let extension = Path::new(&chemin)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "exe" => {
            let (title, install_dir) = detected::decrire(&chemin);
            Ok(Inspection { title, launch_target: chemin, launch_args: None, install_dir })
        }
        "lnk" => {
            let title = nom_du_raccourci(&chemin);
            Ok(match executable_vise(Path::new(&chemin)) {
                Some(r) => {
                    let install_dir = dossier_du_jeu(&r.cible, &title);
                    if demarre_chez_lui(&r) {
                        Inspection { title, launch_target: r.cible, launch_args: r.arguments, install_dir }
                    } else {
                        // « Démarrer dans » un autre dossier que celui de l'exécutable :
                        // seul le raccourci sait le reproduire, on le garde pour lancer.
                        // Le dossier du jeu, lui, reste celui de la vraie cible.
                        Inspection { title, launch_target: chemin, launch_args: None, install_dir }
                    }
                }
                None => Inspection { title, launch_target: chemin, launch_args: None, install_dir: None },
            })
        }
        _ => Ok(Inspection {
            title: detected::nom_lisible(&chemin),
            launch_target: chemin,
            launch_args: None,
            install_dir: None,
        }),
    }
}

/// L'exécutable de jeu qu'un raccourci désigne, s'il en désigne un.
///
/// Un raccourci vers `steam.exe -applaunch …` ou vers le client Riot n'en désigne pas :
/// prendre sa cible ferait surveiller le launcher à la place du jeu.
fn executable_vise(lnk: &Path) -> Option<raccourci::Raccourci> {
    raccourci::lire(lnk).filter(|r| {
        r.cible.to_lowercase().ends_with(".exe")
            && Path::new(&r.cible).is_file()
            && detected::peut_etre_un_jeu(&r.cible)
    })
}

/// Le raccourci démarre-t-il sa cible depuis le dossier de celle-ci ? C'est ce que fait
/// Torii en lançant un exécutable (cf. `launch_executable`) : dans ce cas la cible et ses
/// arguments suffisent à reproduire le raccourci.
fn demarre_chez_lui(r: &raccourci::Raccourci) -> bool {
    let Some(dossier) = r.dossier.as_deref() else {
        return true;
    };
    Path::new(&r.cible)
        .parent()
        .is_some_and(|parent| normalize(&parent.to_string_lossy()) == normalize(dossier))
}

/// Dossier d'un jeu dont le titre vient d'un humain : celui qui porte ce titre, sinon
/// celui que désigne l'exécutable (cf. [`detected::dossier_nomme`], [`detected::decrire`]).
fn dossier_du_jeu(exe: &str, titre: &str) -> Option<String> {
    detected::dossier_nomme(exe, titre).or_else(|| detected::decrire(exe).1)
}

/// Nom d'un raccourci, sans le « - Raccourci » que Windows ajoute en le créant.
fn nom_du_raccourci(chemin: &str) -> String {
    let nom = Path::new(chemin)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let nom = nom.trim();
    for suffixe in [" - Raccourci", " - Shortcut"] {
        if let Some(court) = nom.strip_suffix(suffixe) {
            // « jeu.exe - Raccourci » : le nom d'origine porte encore son extension.
            return court.trim_end_matches(".exe").trim().to_string();
        }
    }
    nom.to_string()
}

/// Un chemin collé depuis l'Explorateur (« Copier en tant que chemin ») arrive entre
/// guillemets.
fn sans_guillemets(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

/// Une adresse (`steam://rungameid/620`, `shell:AppsFolder\…`) plutôt qu'un fichier :
/// un schéma d'au moins deux lettres, pour ne pas prendre `C:\…` pour une adresse.
pub(crate) fn est_une_adresse(cible: &str) -> bool {
    let Some((schema, _)) = cible.split_once(':') else {
        return false;
    };
    schema.len() >= 2
        && schema.starts_with(|c: char| c.is_ascii_alphabetic())
        && schema.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

/// Refuse une cible qui ne mènera nulle part : mieux vaut le dire à l'ajout qu'au clic sur
/// « Jouer ». Une adresse n'est pas vérifiable, on la laisse passer.
fn verifier_cible(cible: &str) -> Result<(), String> {
    if cible.is_empty() {
        return Err(crate::locale::tr("Indique le fichier à lancer.", "Choose the file to launch.").into());
    }
    if est_une_adresse(cible) || Path::new(cible).is_file() {
        return Ok(());
    }
    Err(if crate::locale::en() { format!("File not found: {cible}") } else { format!("Fichier introuvable : {cible}") })
}

// --- Ajout, correction, retrait ----------------------------------------------------

impl ManualInput {
    /// Espaces et guillemets de trop retirés, champs vides ramenés à « absent ».
    fn nettoye(mut self) -> Self {
        let vide_vers_absent = |s: Option<String>| s.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        self.title = self.title.trim().to_string();
        self.launch_target = sans_guillemets(&self.launch_target);
        self.launch_args = vide_vers_absent(self.launch_args);
        self.install_dir = vide_vers_absent(self.install_dir.map(|d| sans_guillemets(&d)));
        self.cover_url = vide_vers_absent(self.cover_url.map(|c| sans_guillemets(&c)));
        self
    }
}

/// Reporte une saisie sur un jeu (manuel ou détecté, cf. [`detected::update`]).
///
/// 🔑 L'`id` ne change PAS, même si le titre change : il sert de clé aux favoris, aux jeux
/// masqués, à l'historique de session et à la fiche ouverte côté front. Le recalculer
/// depuis le nouveau titre orphelinerait tout ça d'un coup.
///
/// La cible n'est revérifiée que si elle change : on doit pouvoir renommer un jeu dont le
/// disque externe est débranché.
pub(super) fn appliquer(game: &mut GameDto, input: ManualInput) -> Result<(), String> {
    let input = input.nettoye();
    if input.title.is_empty() {
        return Err(crate::locale::tr("Donne un titre au jeu.", "Give the game a title.").into());
    }
    if normalize(&input.launch_target) != normalize(&game.launch_target) {
        verifier_cible(&input.launch_target)?;
    }
    game.title = input.title;
    game.launch_target = input.launch_target;
    game.launch_args = input.launch_args;
    game.install_dir = input.install_dir;
    game.cover_url = input.cover_url;
    if let Some(fiche) = input.fiche {
        game.hero_url = fiche.hero_url;
        game.genre = fiche.genre;
        game.description = fiche.description;
        game.developer = fiche.developer;
        game.year = fiche.year;
        game.screenshots = fiche.screenshots;
    }
    Ok(())
}

/// Identifiant d'un nouveau jeu manuel : tiré du titre, et **libre**.
///
/// Deux jeux peuvent porter le même titre (deux versions de Minecraft). Sans le suffixe,
/// le second prenait l'id du premier : écrit dans le fichier, mais invisible à l'écran et
/// écrasé au scan suivant.
fn id_libre(titre: &str, pris: &[GameDto]) -> String {
    let slug: String = titre
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let base = format!("manual:{slug}");
    if !pris.iter().any(|g| g.id == base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !pris.iter().any(|g| &g.id == id))
        .unwrap_or(base)
}

/// Le jeu hors launcher qui lance déjà exactement ça (même cible, mêmes arguments).
///
/// Les arguments comptent : un émulateur sert autant de jeux qu'il a de ROM.
///
/// 🔑 On compare ce qui est **réellement lancé** : un jeu ajouté autrefois par son
/// raccourci et le même jeu ajouté aujourd'hui par l'exécutable que vise ce raccourci
/// sont un seul jeu. Vu à l'essai : sans ça, les deux entrées cohabitaient, avec le même
/// dossier surveillé.
fn deja_present<'a>(jeux: &'a [GameDto], input: &ManualInput) -> Option<&'a GameDto> {
    let voulu = lancement_reel(&input.launch_target, input.launch_args.as_deref());
    jeux.iter()
        .find(|g| lancement_reel(&g.launch_target, g.launch_args.as_deref()) == voulu)
}

/// Ce que lance une cible : celle d'un raccourci lisible et ses arguments, sinon la cible
/// telle quelle. Chemin normalisé, pour comparer.
fn lancement_reel(cible: &str, arguments: Option<&str>) -> (String, Option<String>) {
    let est_un_raccourci = Path::new(cible).extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
    if est_un_raccourci {
        if let Some(r) = raccourci::lire(Path::new(cible)) {
            return (normalize(&r.cible), r.arguments);
        }
    }
    (normalize(cible), arguments.map(str::trim).filter(|a| !a.is_empty()).map(String::from))
}

/// Ajoute un jeu manuel et renvoie la liste à jour.
pub fn add(config_dir: &Path, input: ManualInput) -> Result<Vec<GameDto>, String> {
    let input = input.nettoye();
    verifier_cible(&input.launch_target)?;

    let mut games = scan(config_dir);
    // Un jeu déjà repéré par le surveillant ferait un doublon dans la même catégorie.
    let detectes = detected::scan(config_dir);
    if let Some(existant) = deja_present(&games, &input).or_else(|| deja_present(&detectes, &input)) {
        return Err(if crate::locale::en() {
            format!("This game is already in your library: {}", existant.title)
        } else {
            format!("Ce jeu est déjà dans ta bibliothèque : {}", existant.title)
        });
    }

    let mut game = GameDto {
        id: id_libre(input.title.trim(), &games),
        platform: "manual".into(),
        installed: true,
        // Cible posée d'emblée : `appliquer` ne la revérifie pas une seconde fois.
        launch_target: input.launch_target.clone(),
        ..Default::default()
    };
    appliquer(&mut game, input)?;
    games.push(game);
    persist(config_dir, &games)?;
    Ok(games)
}

/// Ajoute plusieurs jeux d'un coup (« Trouvés sur ce PC »). Chacun passe les mêmes
/// contrôles qu'un ajout seul ; un refus n'empêche pas les suivants. Renvoie la liste à
/// jour et un message par jeu refusé.
pub fn add_plusieurs(config_dir: &Path, inputs: Vec<ManualInput>) -> (Vec<GameDto>, Vec<String>) {
    let mut refus = Vec::new();
    for input in inputs {
        let titre = input.title.trim().to_string();
        if let Err(e) = add(config_dir, input) {
            refus.push(format!("{titre} : {e}"));
        }
    }
    (scan(config_dir), refus)
}

/// Met à jour un jeu manuel existant (édition de sa fiche) et renvoie la liste à jour.
pub fn update(config_dir: &Path, id: &str, input: ManualInput) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    let game = games
        .iter_mut()
        .find(|g| g.id == id)
        .ok_or_else(|| if crate::locale::en() { format!("Manually added game not found: {id}") } else { format!("Jeu manuel introuvable : {id}") })?;
    appliquer(game, input)?;
    persist(config_dir, &games)?;
    Ok(games)
}

/// Retire un jeu manuel par son id et renvoie la liste à jour.
pub fn remove(config_dir: &Path, id: &str) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    games.retain(|g| g.id != id);
    persist(config_dir, &games)?;
    Ok(games)
}

/// Répare les jeux ajoutés avant que Torii sache lire un raccourci. Renvoie leur nombre.
///
/// L'ancien formulaire donnait pour dossier d'installation celui du fichier choisi, quel
/// qu'il soit. Pour un raccourci, ou un fichier posé à même une étagère, c'est le Bureau
/// ou `E:\Games` — et le suivi de session prenait alors pour ce jeu tout ce qui en
/// démarrait.
///
/// 🔑 On ne corrige que cette signature précise (dossier = celui du fichier lancé, qui est
/// un raccourci ou vit sur une étagère), et que le dossier : la cible de lancement
/// fonctionne, on n'y touche pas. Le bon dossier est celui de l'exécutable que vise le
/// raccourci ; faute de pouvoir le désigner, le jeu n'a plus de dossier du tout — mieux
/// vaut ne pas suivre ses parties que lui attribuer celles des autres.
///
/// Idempotent : une fois réparé, un jeu ne porte plus la signature.
pub fn reparer(config_dir: &Path) -> usize {
    let mut games = scan(config_dir);
    let mut repares = 0;
    for game in &mut games {
        let Some(dossier) = game.install_dir.as_deref().filter(|d| !d.is_empty()) else {
            continue;
        };
        let cible = Path::new(&game.launch_target);
        let chez_le_fichier = cible
            .parent()
            .is_some_and(|parent| normalize(&parent.to_string_lossy()) == normalize(dossier));
        let est_un_raccourci = cible.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
        if !chez_le_fichier || !(est_un_raccourci || detected::est_une_etagere(Path::new(dossier))) {
            continue;
        }
        game.install_dir = if est_un_raccourci {
            executable_vise(cible).and_then(|r| dossier_du_jeu(&r.cible, &game.title))
        } else {
            None
        };
        repares += 1;
    }
    if repares > 0 && persist(config_dir, &games).is_err() {
        return 0;
    }
    repares
}

fn persist(config_dir: &Path, games: &[GameDto]) -> Result<(), String> {
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(games).map_err(|e| e.to_string())?;
    fs::write(store_path(config_dir), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str, exe: &str) -> ManualInput {
        ManualInput {
            title: title.into(),
            launch_target: exe.into(),
            launch_args: None,
            install_dir: None,
            cover_url: None,
            fiche: None,
        }
    }

    /// Dossier de test vide, avec un faux exécutable dedans. Renvoie (dossier, exécutable).
    fn bac(nom: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("torii-manual-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("jeu.exe");
        fs::write(&exe, b"MZ").unwrap();
        (dir, exe.to_string_lossy().to_string())
    }

    #[test]
    fn add_update_remove_roundtrip() {
        let (dir, exe) = bac("aller-retour");

        add(&dir, input("Mon Jeu", &exe)).unwrap();
        let games = scan(&dir);
        assert_eq!(games.len(), 1);
        let id = games[0].id.clone();
        assert!(games[0].installed);

        // Édition : le titre change, l'id NON (favoris/masqués/sessions y sont accrochés).
        let autre = dir.join("deluxe.exe");
        fs::write(&autre, b"MZ").unwrap();
        let autre = autre.to_string_lossy().to_string();
        let mut edit = input("Mon Jeu — Édition Deluxe", &autre);
        edit.cover_url = Some("https://exemple/cover.jpg".into());
        edit.launch_args = Some("  -windowed ".into());
        update(&dir, &id, edit).unwrap();
        let after = scan(&dir);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, id, "l'id d'un jeu manuel doit rester stable");
        assert_eq!(after[0].title, "Mon Jeu — Édition Deluxe");
        assert_eq!(after[0].launch_target, autre);
        assert_eq!(after[0].launch_args.as_deref(), Some("-windowed"));
        assert_eq!(after[0].cover_url.as_deref(), Some("https://exemple/cover.jpg"));

        // Éditer un id inconnu échoue proprement.
        assert!(update(&dir, "manual:inexistant", input("X", &exe)).is_err());

        remove(&dir, &id).unwrap();
        assert!(scan(&dir).is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    /// Deux jeux du même titre : le second avait l'id du premier et disparaissait.
    #[test]
    fn deux_jeux_du_meme_titre_ont_chacun_leur_id() {
        let (dir, exe) = bac("homonymes");
        let autre = dir.join("autre.exe");
        fs::write(&autre, b"MZ").unwrap();

        add(&dir, input("Minecraft", &exe)).unwrap();
        add(&dir, input("Minecraft", &autre.to_string_lossy())).unwrap();
        let mut troisieme = input("Minecraft", &exe);
        troisieme.launch_args = Some("--demo".into());
        add(&dir, troisieme).unwrap();

        let ids: Vec<String> = scan(&dir).into_iter().map(|g| g.id).collect();
        assert_eq!(ids, ["manual:minecraft", "manual:minecraft-2", "manual:minecraft-3"]);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Même cible, mêmes arguments : c'est le même jeu, quel que soit le titre saisi.
    #[test]
    fn refuse_un_jeu_deja_present() {
        let (dir, exe) = bac("doublon");
        add(&dir, input("Mon Jeu", &exe)).unwrap();

        let erreur = add(&dir, input("Autre nom", &exe.to_uppercase())).unwrap_err();
        assert!(erreur.contains("Mon Jeu"), "{erreur}");
        assert_eq!(scan(&dir).len(), 1);

        // Un émulateur sert plusieurs jeux : les arguments les distinguent.
        let mut rom = input("Zelda", &exe);
        rom.launch_args = Some(r#"--exec="E:\Roms\zelda.iso""#.into());
        add(&dir, rom).unwrap();
        assert_eq!(scan(&dir).len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_une_cible_introuvable() {
        let (dir, exe) = bac("introuvable");
        assert!(add(&dir, input("Fantôme", r"Z:\nulle\part.exe")).is_err());
        assert!(add(&dir, input("Sans cible", "  ")).is_err());
        assert!(add(&dir, input("   ", &exe)).is_err());
        assert!(scan(&dir).is_empty());

        // Une adresse ne se vérifie pas : elle passe.
        add(&dir, input("Portal 2", "steam://rungameid/620")).unwrap();
        // Chemin collé entre guillemets : accepté, et rangé sans eux.
        add(&dir, input("Mon Jeu", &format!("\"{exe}\""))).unwrap();
        assert_eq!(scan(&dir)[1].launch_target, exe);

        // Corriger le titre d'un jeu dont le fichier a disparu reste possible…
        let id = scan(&dir)[1].id.clone();
        fs::remove_file(&exe).unwrap();
        update(&dir, &id, input("Mon Jeu (disque débranché)", &exe)).unwrap();
        // …mais pas lui donner une nouvelle cible qui n'existe pas.
        assert!(update(&dir, &id, input("Mon Jeu", r"Z:\nulle\part.exe")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    /// La fiche absente ne touche à rien ; présente, elle remplace — vide comprise.
    #[test]
    fn la_fiche_choisie_remplace_les_infos_du_jeu() {
        let (dir, exe) = bac("fiche");
        let mut avec = input("Celeste", &exe);
        avec.fiche = Some(Fiche {
            genre: Some("Plateforme".into()),
            year: Some(2018),
            screenshots: vec!["https://exemple/1.jpg".into()],
            ..Default::default()
        });
        add(&dir, avec).unwrap();
        let id = scan(&dir)[0].id.clone();

        update(&dir, &id, input("Celeste", &exe)).unwrap();
        let garde = &scan(&dir)[0];
        assert_eq!(garde.genre.as_deref(), Some("Plateforme"));
        assert_eq!(garde.year, Some(2018));

        let mut detache = input("Celeste", &exe);
        detache.fiche = Some(Fiche::default());
        update(&dir, &id, detache).unwrap();
        let vide = &scan(&dir)[0];
        assert_eq!(vide.genre, None);
        assert!(vide.screenshots.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn distingue_une_adresse_d_un_chemin() {
        assert!(est_une_adresse("steam://rungameid/620"));
        assert!(est_une_adresse("com.epicgames.launcher://apps/Fortnite?action=launch"));
        assert!(est_une_adresse(r"shell:AppsFolder\Microsoft.MinecraftUWP_8wekyb3d8bbwe!App"));
        assert!(!est_une_adresse(r"C:\Jeux\jeu.exe"));
        assert!(!est_une_adresse(r"\\serveur\partage\jeu.exe"));
        assert!(!est_une_adresse("jeu.exe"));
    }

    #[test]
    fn inspecte_un_executable_et_un_script() {
        let (dir, exe) = bac("inspection");
        let vu = inspecter(&format!("  \"{exe}\"  ")).unwrap();
        assert_eq!(vu.launch_target, exe);
        assert_eq!(vu.launch_args, None);
        assert!(!vu.title.is_empty());

        // Un script garde son chemin, sans dossier : rien ne dit qu'il vit avec le jeu.
        let script = dir.join("Lancer_MonJeu.bat");
        fs::write(&script, b"@echo off").unwrap();
        let vu = inspecter(&script.to_string_lossy()).unwrap();
        assert_eq!(vu.title, "Lancer Mon Jeu");
        assert_eq!(vu.install_dir, None);

        // Un `.lnk` illisible reste la cible, sous le nom du raccourci.
        let lnk = dir.join("Mon Jeu - Raccourci.lnk");
        fs::write(&lnk, b"pas un raccourci").unwrap();
        let vu = inspecter(&lnk.to_string_lossy()).unwrap();
        assert_eq!(vu.title, "Mon Jeu");
        assert_eq!(vu.launch_target, lnk.to_string_lossy());
        assert_eq!(vu.install_dir, None);

        assert!(inspecter(r"Z:\nulle\part.exe").is_err());
        assert_eq!(inspecter("steam://rungameid/620").unwrap().title, "");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Un jeu rangé comme sur un vrai disque : `<bac>\Games\Mon Jeu\jeu.exe`, et un
    /// « Bureau » à côté pour y poser des raccourcis. Renvoie (bac, bureau, exécutable).
    fn disque(nom: &str) -> (PathBuf, PathBuf, String) {
        let (bac, _) = bac(nom);
        let jeu = bac.join("Games").join("Mon Jeu");
        let bureau = bac.join("Bureau");
        fs::create_dir_all(&jeu).unwrap();
        fs::create_dir_all(&bureau).unwrap();
        let exe = jeu.join("jeu.exe");
        fs::write(&exe, b"MZ").unwrap();
        (bac, bureau, exe.to_string_lossy().to_string())
    }

    /// Le cœur du lot : un raccourci est remplacé par sa cible, ses arguments, et le
    /// dossier du JEU — plus jamais celui du raccourci.
    #[test]
    fn un_raccourci_donne_sa_cible_et_le_dossier_du_jeu() {
        let (bac, bureau, exe) = disque("raccourci");
        let lnk = bureau.join("Mon Jeu.lnk");
        raccourci::creer(&lnk, &exe, Some("-windowed"), None);

        let vu = inspecter(&lnk.to_string_lossy()).unwrap();
        assert_eq!(vu.title, "Mon Jeu");
        assert!(vu.launch_target.eq_ignore_ascii_case(&exe), "{}", vu.launch_target);
        assert_eq!(vu.launch_args.as_deref(), Some("-windowed"));
        let dossier = vu.install_dir.unwrap();
        assert!(dossier.to_lowercase().ends_with(r"\games\mon jeu"), "{dossier}");

        // « Démarrer dans » ailleurs que chez l'exécutable : le raccourci reste la cible,
        // mais le dossier surveillé est toujours celui du jeu.
        let ailleurs = bureau.join("Mon Jeu (ailleurs).lnk");
        raccourci::creer(&ailleurs, &exe, None, Some(&bac.to_string_lossy()));
        let vu = inspecter(&ailleurs.to_string_lossy()).unwrap();
        assert_eq!(vu.launch_target, ailleurs.to_string_lossy());
        assert!(vu.install_dir.unwrap().to_lowercase().ends_with(r"\games\mon jeu"));
        let _ = fs::remove_dir_all(&bac);
    }

    /// Un jeu ajouté par son raccourci, puis par l'exécutable que vise ce raccourci : c'est
    /// le même jeu, dans un sens comme dans l'autre.
    #[test]
    fn un_raccourci_et_sa_cible_sont_le_meme_jeu() {
        let (bac, bureau, exe) = disque("raccourci-doublon");
        let lnk = bureau.join("Mon Jeu.lnk");
        raccourci::creer(&lnk, &exe, None, None);
        let lnk = lnk.to_string_lossy().to_string();

        // L'ancien formulaire gardait le raccourci comme cible.
        persist(
            &bac,
            &[GameDto { id: "manual:mon-jeu".into(), title: "Mon Jeu".into(), launch_target: lnk.clone(), ..Default::default() }],
        )
        .unwrap();
        let erreur = add(&bac, input("Mon Jeu (exe)", &exe)).unwrap_err();
        assert!(erreur.contains("Mon Jeu"), "{erreur}");

        // Mais un raccourci qui ajoute des options lance autre chose.
        let avec_options = bureau.join("Mon Jeu fenêtré.lnk");
        raccourci::creer(&avec_options, &exe, Some("-windowed"), None);
        let mut fenetre = input("Mon Jeu fenêtré", &exe);
        fenetre.launch_args = Some("-windowed".into());
        add(&bac, fenetre).unwrap();
        assert!(add(&bac, input("Encore", &avec_options.to_string_lossy())).is_err());
        assert_eq!(scan(&bac).len(), 2);
        let _ = fs::remove_dir_all(&bac);
    }

    /// Les jeux ajoutés avec l'ancien formulaire : dossier = celui du fichier choisi.
    #[test]
    fn repare_les_dossiers_fausses_par_l_ancien_formulaire() {
        let (bac, bureau, exe) = disque("reparation");
        let lnk = bureau.join("Mon Jeu.lnk");
        raccourci::creer(&lnk, &exe, None, None);
        let casse = bureau.join("Disparu.lnk");
        fs::write(&casse, b"pas un raccourci").unwrap();
        let sur_etagere = bac.join("Games").join("solo.exe");
        fs::write(&sur_etagere, b"MZ").unwrap();

        let ancien = |id: &str, titre: &str, cible: &Path, dossier: &Path| GameDto {
            id: id.into(),
            title: titre.into(),
            platform: "manual".into(),
            launch_target: cible.to_string_lossy().to_string(),
            install_dir: Some(dossier.to_string_lossy().to_string()),
            ..Default::default()
        };
        let jeu = Path::new(&exe);
        persist(
            &bac,
            &[
                ancien("manual:raccourci", "Mon Jeu", &lnk, &bureau),
                ancien("manual:casse", "Disparu", &casse, &bureau),
                ancien("manual:etagere", "Solo", &sur_etagere, &bac.join("Games")),
                // Un exécutable dans son propre dossier : rien à réparer.
                ancien("manual:sain", "Mon Jeu", jeu, jeu.parent().unwrap()),
            ],
        )
        .unwrap();

        assert_eq!(reparer(&bac), 3);
        let apres = scan(&bac);
        // Le raccourci reste la cible ; seul le dossier est corrigé.
        assert_eq!(apres[0].launch_target, lnk.to_string_lossy());
        assert!(apres[0].install_dir.as_deref().unwrap().to_lowercase().ends_with(r"\games\mon jeu"));
        assert_eq!(apres[1].install_dir, None);
        assert_eq!(apres[2].install_dir, None);
        assert_eq!(apres[3].install_dir.as_deref(), jeu.parent().map(|p| p.to_str().unwrap()));

        // Une seconde passe ne trouve plus rien.
        assert_eq!(reparer(&bac), 0);
        let _ = fs::remove_dir_all(&bac);
    }

    #[test]
    fn retire_le_suffixe_des_raccourcis() {
        assert_eq!(nom_du_raccourci(r"C:\Bureau\Genshin Impact.lnk"), "Genshin Impact");
        assert_eq!(nom_du_raccourci(r"C:\Bureau\jeu.exe - Raccourci.lnk"), "jeu");
        assert_eq!(nom_du_raccourci(r"C:\Bureau\Half-Life 2 - Shortcut.lnk"), "Half-Life 2");
    }
}
