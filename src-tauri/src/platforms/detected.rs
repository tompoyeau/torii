//! Jeux **détectés hors launcher** — Genshin Impact, Dofus, un jeu Game Pass, un
//! exécutable posé sur le disque…
//!
//! Le surveillant de process ([`crate::procwatch`]) ne reconnaissait que les jeux issus
//! d'un scan de bibliothèque : tout ce qui vit hors Steam/Epic/GOG/Riot/Ubisoft était
//! invisible, donc jamais daté dans « Récemment joué » ni diffusé aux amis. C'est ici
//! qu'on comble le trou.
//!
//! # Comment sait-on qu'un `.exe` est un jeu ?
//!
//! On ne le devine pas : **Windows le sait déjà**. La Game Bar tient sa propre liste
//! dans `HKCU\System\GameConfigStore\Children`, alimentée au premier lancement de
//! chaque jeu (chemin complet de l'exécutable, dossier de travail, dernier accès).
//! Relevé sur une machine réelle : 278 entrées, dont zéro navigateur, zéro Discord,
//! zéro éditeur de code — uniquement des jeux et leurs lanceurs. La précision de ce
//! classement est ce qui rend la détection publiable telle quelle.
//!
//! S'y ajoute une règle de chemin pour le Game Pass, dont les jeux s'installent sous
//! `C:\XboxGames\` (le reste du Microsoft Store, lui, n'est retenu que si la Game Bar
//! l'a lui aussi classé jeu — `WindowsApps` contient tout et n'importe quoi).
//!
//! # Ce qu'on en fait
//!
//! Le jeu devient une entrée de bibliothèque comme une autre (`detected_games.json`,
//! plateforme `detected`) : présence, « Récemment joué », fiche, favori, masquage et
//! « ne pas diffuser ce jeu » fonctionnent sans une ligne de plus. Le titre est deviné
//! à partir du dossier, puis **corrigé par IGDB** (qui fournit aussi la jaquette).

use crate::models::GameDto;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Dossiers techniques qu'on traverse sans s'arrêter : ils ne nomment pas le jeu.
const STOP_DIRS: &[&str] = &[
    "bin", "bin64", "binaries", "win64", "win32", "x64", "x86", "retail", "shipping", "content",
    "build", "release", "data", "client", "app", "game", "games64", "runtime", "jre", "jdk",
    // Dossier par défaut des projets Unreal : il nomme le moteur, pas le jeu.
    "shootergame",
];

/// Dossiers de rangement : on ne remonte jamais au-dessus, ils nomment une étagère
/// (« E:\Games »), pas un jeu.
const CONTAINER_DIRS: &[&str] = &[
    "games", "jeux", "program files", "program files (x86)", "programdata", "common", "steamapps",
    "steamlibrary", "windowsapps", "xboxgames", "appdata", "local", "locallow", "roaming", "users",
    "documents", "downloads", "desktop", "temp", "tmp", "wpsystem",
];

/// Exécutables à ne jamais prendre pour un jeu, même listés par la Game Bar : les
/// clients de launcher (qui tournent en permanence) et Torii lui-même.
const NOT_A_GAME: &[&str] = &[
    "steam.exe",
    "steamwebhelper.exe",
    "epicgameslauncher.exe",
    "epicwebhelper.exe",
    "galaxyclient.exe",
    "galaxyclienthelper.exe",
    "upc.exe",
    "ubisoftconnect.exe",
    "ubisoftgamelauncher.exe",
    "battle.net.exe",
    "agent.exe",
    "riotclientservices.exe",
    "riotclientux.exe",
    "eadesktop.exe",
    "eabackgroundservice.exe",
    "ealaunchhelper.exe",
    "torii.exe",
    "ludo.exe",
];

/// Fragments de nom qui trahissent un utilitaire embarqué avec le jeu (anti-triche,
/// rapport de plantage, désinstalleur…) plutôt que le jeu.
const NOISE: &[&str] = &[
    "crashhandler",
    "crashreport",
    "crashpad",
    "unins",
    "setup",
    "installer",
    "updater",
    "easyanticheat",
    "battleye",
    "vcredist",
    "dxsetup",
    "dotnet",
    "helper",
    "service",
];

/// Dossiers système : rien de ce qui vit là n'est un jeu.
const SYSTEM_ROOTS: &[&str] = &[
    "c:\\windows\\",
    "c:\\program files\\windowsapps\\microsoft.windows.",
];

// --- Persistance -----------------------------------------------------------------

fn store_path(config_dir: &Path) -> PathBuf {
    config_dir.join("detected_games.json")
}

fn ignore_path(config_dir: &Path) -> PathBuf {
    config_dir.join("detected_ignored.json")
}

/// Les jeux détectés jusqu'ici (relus à chaque scan de bibliothèque).
pub fn scan(config_dir: &Path) -> Vec<GameDto> {
    let Ok(text) = fs::read_to_string(store_path(config_dir)) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<GameDto>>(&text).unwrap_or_default()
}

fn persist(config_dir: &Path, games: &[GameDto]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(games).map_err(|e| e.to_string())?;
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    fs::write(store_path(config_dir), text).map_err(|e| e.to_string())
}

/// Exécutables que l'utilisateur a retirés de sa bibliothèque : on ne les redétecte pas.
fn ignored(config_dir: &Path) -> HashSet<String> {
    fs::read_to_string(ignore_path(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

/// Cet exécutable a-t-il déjà été refusé par l'utilisateur ? (« Retirer de la
/// bibliothèque » sur un jeu détecté.)
pub fn is_ignored(config_dir: &Path, exe: &str) -> bool {
    ignored(config_dir).contains(&normalize(exe))
}

/// Enregistre un jeu fraîchement détecté (ou rafraîchit celui qui porte le même id).
/// Renvoie `false` si l'exécutable est sur la liste des refusés.
pub fn remember(config_dir: &Path, game: &GameDto) -> bool {
    if ignored(config_dir).contains(&normalize(&game.launch_target)) {
        return false;
    }
    let mut games = scan(config_dir);
    match games.iter_mut().find(|g| g.id == game.id) {
        Some(existing) => *existing = game.clone(),
        None => games.push(game.clone()),
    }
    let _ = persist(config_dir, &games);
    true
}

/// Corrige un jeu détecté avec ce qu'IGDB a reconnu (vrai titre, jaquette, genre…).
///
/// 🔑 L'`id` ne bouge PAS, même quand le titre change : il sert de clé aux favoris, à
/// l'historique de session et à la présence en cours. Le recalculer orphelinerait tout.
pub fn refine(config_dir: &Path, id: &str, apply: impl FnOnce(&mut GameDto)) -> Option<GameDto> {
    let mut games = scan(config_dir);
    let game = games.iter_mut().find(|g| g.id == id)?;
    apply(game);
    let updated = game.clone();
    let _ = persist(config_dir, &games);
    Some(updated)
}

/// Corrige à la main un jeu détecté (titre deviné à côté de la plaque, jaquette
/// choisie sur le disque…). Même contrat que [`super::manual::update`] : l'`id` ne
/// bouge pas, la liste à jour est renvoyée.
pub fn update(
    config_dir: &Path,
    id: &str,
    input: super::manual::ManualInput,
) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    let game = games
        .iter_mut()
        .find(|g| g.id == id)
        .ok_or_else(|| format!("Jeu détecté introuvable : {id}"))?;
    game.title = input.title;
    game.launch_target = input.launch_target;
    game.install_dir = input.install_dir;
    game.cover_url = input.cover_url;
    persist(config_dir, &games)?;
    Ok(games)
}

/// Retire un jeu détecté de la bibliothèque **et** de la détection : son exécutable
/// rejoint la liste des refusés, sans quoi la partie suivante le ferait revenir.
pub fn forget(config_dir: &Path, id: &str) -> Result<Vec<GameDto>, String> {
    let mut games = scan(config_dir);
    let Some(pos) = games.iter().position(|g| g.id == id) else {
        return Ok(games);
    };
    let exe = normalize(&games.remove(pos).launch_target);

    let mut refused: Vec<String> = ignored(config_dir).into_iter().collect();
    if !exe.is_empty() && !refused.contains(&exe) {
        refused.push(exe);
        if let Ok(text) = serde_json::to_string_pretty(&refused) {
            let _ = fs::write(ignore_path(config_dir), text);
        }
    }
    persist(config_dir, &games)?;
    Ok(games)
}

// --- Reconnaissance ---------------------------------------------------------------

/// Un exécutable inconnu du scan de bibliothèque : est-ce un jeu, et lequel ?
/// `None` = ce n'est pas un jeu (ou on n'en sait rien, ce qui revient au même).
pub fn identify(exe: &str) -> Option<GameDto> {
    let path = normalize(exe);
    if !plausible(&path) {
        return None;
    }
    let fiche = game_bar_entry(&path);
    // Un jeu Game Pass est reconnu à son chemin même sans fiche de la Game Bar.
    if fiche.is_none() && !path.contains("\\xboxgames\\") {
        return None;
    }
    let fiche = fiche.unwrap_or_default();

    let (title, root) = match runtime_partage(&path) {
        // Moteur partagé (une machine virtuelle Java, un interpréteur…) : le chemin ne
        // nomme que le moteur, jamais le jeu. Windows, lui, retient de quoi trancher —
        // pour Minecraft, `Arguments = minecraft`. Sans cette indication, on s'abstient.
        true => (fiche.nom()?, exe.to_string()),
        false => {
            let (devine, root) = title_and_root(exe);
            // Le titre de la Game Bar, quand il existe, vaut mieux qu'une devinette.
            (fiche.title.clone().filter(|t| !t.is_empty()).unwrap_or(devine), root)
        }
    };
    if title.is_empty() {
        return None;
    }
    Some(GameDto {
        id: format!("detected:{}", slug(&title)),
        title,
        platform: "detected".into(),
        installed: true,
        install_dir: Some(root),
        launch_target: exe.to_string(),
        ..Default::default()
    })
}

/// Exécutables qui ne sont qu'un **moteur d'exécution** : le même fichier fait tourner
/// n'importe quel jeu, donc ni son nom ni son dossier n'apprennent quoi que ce soit.
fn runtime_partage(path: &str) -> bool {
    const MOTEURS: &[&str] = &["java.exe", "javaw.exe", "python.exe", "pythonw.exe", "node.exe"];
    let file = path.rsplit('\\').next().unwrap_or_default();
    MOTEURS.contains(&file)
}

/// Cet exécutable a-t-il seulement le profil d'un jeu ? Sert au surveillant à décider
/// quels process inconnus valent la peine d'être rejugés au passage suivant : sans ce
/// tri, il garderait sous le coude chaque onglet de navigateur qui s'ouvre.
///
/// Répondre `true` ne dit rien de plus que « ce n'est pas exclu d'office » — seule
/// [`identify`] tranche.
pub fn peut_etre_un_jeu(exe: &str) -> bool {
    plausible(&normalize(exe))
}

/// Écarte d'emblée ce qui ne peut pas être un jeu : dossiers système, clients de
/// launcher, utilitaires embarqués.
fn plausible(path: &str) -> bool {
    if !path.ends_with(".exe") || SYSTEM_ROOTS.iter().any(|r| path.starts_with(r)) {
        return false;
    }
    let file = path.rsplit('\\').next().unwrap_or_default();
    if NOT_A_GAME.contains(&file) {
        return false;
    }
    let stem = file.trim_end_matches(".exe");
    !NOISE.iter().any(|n| stem.contains(n))
}

/// Ce que la Game Bar retient d'un jeu. `Arguments` est la pièce décisive pour les
/// moteurs partagés : le même `javaw.exe` sert à tous les jeux Java, et Windows y note
/// `minecraft`.
#[derive(Clone, Default)]
struct FicheGameBar {
    title: Option<String>,
    arguments: Option<String>,
}

impl FicheGameBar {
    /// Nom du jeu tel que Windows le connaît : son titre, sinon son argument — refusé
    /// s'il ressemble à autre chose qu'un nom (chemin, ligne de commande entière).
    fn nom(&self) -> Option<String> {
        let brut = self
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .or_else(|| self.arguments.clone())?;
        let brut = brut.trim();
        if brut.is_empty() || brut.len() > 40 || brut.contains(['\\', '/', '-']) {
            return None;
        }
        Some(capitalise(&pretty(brut)))
    }
}

/// Première lettre de chaque mot en majuscule (`minecraft` → « Minecraft ») : un
/// argument de ligne de commande n'est pas écrit pour être lu.
fn capitalise(s: &str) -> String {
    s.split(' ')
        .map(|mot| {
            let mut c = mot.chars();
            match c.next() {
                Some(p) => p.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fiche de la Game Bar pour cet exécutable, si Windows le tient pour un jeu.
fn game_bar_entry(path: &str) -> Option<FicheGameBar> {
    if let Some(fiche) = index_get(path) {
        return Some(fiche);
    }
    // L'entrée de la Game Bar naît au **premier** lancement du jeu : si notre index
    // date d'avant, il ne peut pas la connaître. On le rafraîchit une fois avant de
    // conclure que ce n'en est pas un.
    refresh_index();
    index_get(path)
}

/// Index des exécutables classés « jeu » par Windows, avec l'instant de sa lecture.
struct Index {
    exes: HashMap<String, FicheGameBar>,
    read_at: Instant,
}

/// Délai minimal entre deux lectures du registre (une lecture = quelques centaines de
/// sous-clés : négligeable, mais inutile à répéter à chaque process qui démarre).
const INDEX_TTL: Duration = Duration::from_secs(30);

fn index() -> &'static Mutex<Option<Index>> {
    static INDEX: Mutex<Option<Index>> = Mutex::new(None);
    &INDEX
}

fn index_get(path: &str) -> Option<FicheGameBar> {
    index()
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().and_then(|idx| idx.exes.get(path).cloned()))
}

fn refresh_index() {
    let Ok(mut slot) = index().lock() else { return };
    if slot.as_ref().is_some_and(|idx| idx.read_at.elapsed() < INDEX_TTL) {
        return;
    }
    *slot = Some(Index {
        exes: game_bar_entries(),
        read_at: Instant::now(),
    });
}

/// Fiches de la Game Bar, indexées par chemin d'exécutable normalisé.
#[cfg(windows)]
fn game_bar_entries() -> HashMap<String, FicheGameBar> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let mut out = HashMap::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(children) = hkcu.open_subkey(r"System\GameConfigStore\Children") else {
        return out;
    };
    for child in children.enum_keys().flatten() {
        let Ok(entry) = children.open_subkey(&child) else {
            continue;
        };
        let path: String = entry.get_value("MatchedExeFullPath").unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        out.insert(
            normalize(&path),
            FicheGameBar {
                title: entry.get_value("Title").ok(),
                arguments: entry.get_value("Arguments").ok(),
            },
        );
    }
    out
}

#[cfg(not(windows))]
fn game_bar_entries() -> HashMap<String, FicheGameBar> {
    HashMap::new()
}

// --- Titre et dossier du jeu ------------------------------------------------------

/// Devine le nom du jeu et son dossier racine à partir du chemin de l'exécutable.
///
/// On remonte les dossiers en sautant les étages techniques (`Binaries\Win64`…) et on
/// s'arrête net sur une étagère (`E:\Games`). Le dossier retenu est celui qui **parle
/// du même jeu que l'exécutable** (`Genshin Impact game` ↔ `GenshinImpact.exe`), sinon
/// le dernier dossier parlant rencontré.
fn title_and_root(exe: &str) -> (String, String) {
    let path = Path::new(exe);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem_key = norm(&pretty(&stem));

    let mut chosen: Option<&Path> = None;
    let mut last_speaking: Option<&Path> = None;
    for dir in path.ancestors().skip(1) {
        // Plus de nom = racine du disque : on ne remonte pas plus haut.
        let Some(name) = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()) else {
            break;
        };
        if CONTAINER_DIRS.contains(&name.as_str()) {
            break;
        }
        if STOP_DIRS.contains(&name.as_str()) {
            continue;
        }
        let key = norm(&name);
        if !stem_key.is_empty() && !key.is_empty() && (key.contains(&stem_key) || stem_key.contains(&key)) {
            chosen = Some(dir);
            break;
        }
        if last_speaking.is_none() {
            last_speaking = Some(dir);
        }
    }

    let dir = chosen.or(last_speaking);
    let par_dossier = dir
        .and_then(|d| d.file_name())
        .map(|n| pretty(&n.to_string_lossy()))
        .filter(|t| !t.is_empty());
    let par_exe = pretty(&stem);

    // Aucun dossier ne parle du jeu : on garde le nom le plus informatif des deux.
    // « …\AppData\Local\Ubisoft\r6s\RainbowSix.exe » doit donner « Rainbow Six », pas
    // « r6s » — un dossier peut être un nom de code, l'exécutable presque jamais.
    let title = match par_dossier {
        Some(d) if chosen.is_some() => d,
        Some(d) if lettres(&d) >= lettres(&par_exe) => d,
        Some(_) | None => par_exe,
    };

    // Racine surveillée : le dossier du jeu, à défaut celui de l'exécutable, à défaut
    // l'exécutable lui-même (jamais une étagère : on y verrait passer les voisins).
    let root = dir
        .or_else(|| path.parent())
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_else(|| exe.to_string());

    (title, root)
}

/// Rend un nom de dossier ou d'exécutable présentable :
/// `GenshinImpact` → « Genshin Impact », `ONCE_HUMAN` → « Once Human »,
/// `Moria-Win64-Shipping` → « Moria », `Genshin Impact game` → « Genshin Impact ».
fn pretty(raw: &str) -> String {
    // Séparateurs → espaces, puis coupe les collages `motMot` (mais pas les sigles).
    let flat: String = raw
        .chars()
        .map(|c| if matches!(c, '_' | '-' | '.') { ' ' } else { c })
        .collect();
    let chars: Vec<char> = flat.chars().collect();
    let mut spaced = String::new();
    for (i, c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1).copied();
        let coupe = c.is_uppercase()
            && match (prev, next) {
                // « ...tI » : une majuscule qui suit une minuscule ouvre un mot.
                (Some(p), _) if p.is_lowercase() || p.is_numeric() => true,
                // « XCOMGame » : dernière majuscule d'un sigle, suivie d'un mot.
                (Some(p), Some(n)) if p.is_uppercase() && n.is_lowercase() => true,
                _ => false,
            };
        if coupe && !spaced.is_empty() && !spaced.ends_with(' ') {
            spaced.push(' ');
        }
        spaced.push(*c);
    }

    // Mots parasites en fin de nom : `Skyrim Win64` → « Skyrim ». Jamais le premier mot,
    // sinon « Game Dev Tycoon » se ferait décapiter.
    const JUNK: &[&str] = &[
        "win64", "win32", "x64", "x86", "shipping", "game", "exe", "launcher", "client",
        // Variantes de rendu : « RainbowSix_Vulkan » et « RainbowSix » sont le même jeu.
        "vulkan", "dx9", "dx11", "dx12", "d3d11", "d3d12", "opengl", "debug", "final",
    ];
    let mut mots: Vec<&str> = spaced.split_whitespace().collect();
    while mots.len() > 1 && JUNK.contains(&mots[mots.len() - 1].to_lowercase().as_str()) {
        mots.pop();
    }
    let titre = mots.join(" ");

    // Plusieurs mots tout en majuscules (`ONCE HUMAN`) : on capitalise — c'est un nom de
    // fichier, pas un cri. Un mot seul est laissé tel quel : c'est souvent un sigle (XCOM, ARK).
    if titre.contains(' ') && titre == titre.to_uppercase() {
        return titre
            .split(' ')
            .map(|m| {
                let mut c = m.chars();
                match c.next() {
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    titre
}

/// Nombre de lettres d'un nom : mesure grossière de ce qu'il apprend (« r6s » = 2,
/// « Rainbow Six » = 10).
fn lettres(s: &str) -> usize {
    s.chars().filter(|c| c.is_alphabetic()).count()
}

/// Identifiant stable dérivé du titre (même règle que les jeux manuels).
fn slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

/// Clé de comparaison : alphanumérique en minuscules.
fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Chemin comparable : minuscules, séparateurs Windows.
fn normalize(path: &str) -> String {
    path.to_lowercase().replace('/', "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devine_titre_et_dossier() {
        // Le dossier parle du même jeu que l'exécutable : c'est lui qui nomme.
        let (titre, dossier) = title_and_root(r"E:\Games\Genshin Impact game\GenshinImpact.exe");
        assert_eq!(titre, "Genshin Impact");
        assert_eq!(dossier, r"E:\Games\Genshin Impact game");

        // Étages techniques traversés, étagère « Games » jamais franchie.
        let (titre, dossier) =
            title_and_root(r"E:\Games\ReturnToMoria\Moria\Binaries\Win64\Moria-Win64-Shipping.exe");
        assert_eq!(titre, "Moria");
        assert_eq!(dossier, r"E:\Games\ReturnToMoria\Moria");

        // Dossier du publieur au-dessus : on garde celui qui porte le nom du jeu.
        let (titre, _) = title_and_root(r"C:\Users\x\AppData\Local\Ankama\Dofus\Dofus.exe");
        assert_eq!(titre, "Dofus");

        // Dossier en nom de code : l'exécutable en apprend davantage. Les deux
        // installations de Rainbow Six retombent ainsi sur un seul et même jeu.
        let (titre, _) = title_and_root(r"C:\Users\x\AppData\Local\Ubisoft\r6s\RainbowSix.exe");
        assert_eq!(titre, "Rainbow Six");
        let (jumeau, _) =
            title_and_root(r"C:\Users\x\AppData\Local\Ubisoft\r6siege\RainbowSix_Vulkan.exe");
        assert_eq!(jumeau, titre);
    }

    #[test]
    fn nettoie_les_noms_de_fichier() {
        assert_eq!(pretty("GenshinImpact"), "Genshin Impact");
        assert_eq!(pretty("ONCE_HUMAN"), "Once Human");
        assert_eq!(pretty("DeadByDaylight"), "Dead By Daylight");
        assert_eq!(pretty("Skull and Bones"), "Skull and Bones");
        // Sigle préservé, et le « Game » final tombe comme tout mot parasite.
        assert_eq!(pretty("XCOMGame"), "XCOM");
        assert_eq!(pretty("Moria-Win64-Shipping"), "Moria");
    }

    /// Minecraft tourne dans une machine virtuelle Java : le chemin ne mène qu'au
    /// moteur (« java-runtime-gamma »), et c'est Windows qui sait quel jeu y tourne.
    #[test]
    fn nomme_les_jeux_des_moteurs_partages() {
        let jvm = r"e:\wpsystem\...\packages\microsoft.4297127d64ec6_8wekyb3d8bbwe\localcache\local\runtime\java-runtime-gamma\windows-x64\java-runtime-gamma\bin\javaw.exe";
        assert!(runtime_partage(jvm));
        assert!(!runtime_partage(r"e:\games\far cry 4\farcry4.exe"));

        let fiche = FicheGameBar { title: None, arguments: Some("minecraft".into()) };
        assert_eq!(fiche.nom().as_deref(), Some("Minecraft"));

        // Le titre de la Game Bar prime sur l'argument quand il existe.
        let fiche = FicheGameBar {
            title: Some("Minecraft Legends".into()),
            arguments: Some("minecraft".into()),
        };
        assert_eq!(fiche.nom().as_deref(), Some("Minecraft Legends"));

        // Une vraie ligne de commande n'est pas un nom de jeu : on préfère s'abstenir.
        let fiche = FicheGameBar {
            title: None,
            arguments: Some(r"-jar C:\jeux\truc.jar --fullscreen".into()),
        };
        assert_eq!(fiche.nom(), None);
        assert_eq!(FicheGameBar::default().nom(), None);
    }

    #[test]
    fn ecarte_ce_qui_nest_pas_un_jeu() {
        assert!(!plausible(r"c:\windows\system32\notepad.exe"));
        assert!(!plausible(r"e:\steam\steam.exe"));
        assert!(!plausible(r"e:\games\portal 2\crashhandler.exe"));
        assert!(!plausible(r"e:\games\portal 2\readme.txt"));
        assert!(plausible(r"e:\games\portal 2\portal2.exe"));
    }
}
