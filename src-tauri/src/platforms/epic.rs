use crate::models::GameDto;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EpicItem {
    display_name: String,
    #[serde(default)]
    install_location: String,
    #[serde(default)]
    install_size: u64,
    app_name: String,
    #[serde(default)]
    main_game_app_name: String,
    #[serde(default)]
    catalog_namespace: String,
    #[serde(default)]
    catalog_item_id: String,
    #[serde(default, rename = "bIsIncompleteInstall")]
    incomplete: bool,
}

/// Une ligne du journal `installations.jsonl` du nouveau launcher Epic.
///
/// Seul l'identifiant est obligatoire : une ligne de suppression (« tombstone ») ne porte
/// que `installationId`, `writtenAt` et `deleted`, et le format, encore en préversion,
/// peut s'enrichir.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Installation {
    installation_id: String,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    dir: String,
    #[serde(default)]
    namespace: String,
    /// Le `CatalogItemId` des anciens manifestes.
    #[serde(default)]
    item_id: String,
    /// L'`AppName` des anciens manifestes et de l'API du compte (« Calluna » = Control).
    #[serde(default)]
    artifact_id: String,
    #[serde(default)]
    version: String,
    /// Renseigné sur les DLC et les composants d'un jeu (Fortnite en a trois).
    #[serde(default)]
    parent_installation_id: Option<String>,
    /// Contenu Fab (assets Unreal) : pas un jeu.
    #[serde(default)]
    is_fab: bool,
    /// Marque une ligne de suppression. Type non documenté : toute valeur autre que
    /// `false` compte.
    #[serde(default)]
    deleted: Option<Value>,
}

impl Installation {
    fn supprimee(&self) -> bool {
        self.deleted.as_ref().is_some_and(|v| *v != Value::Bool(false))
    }

    /// Jeu de base entièrement installé : ni DLC, ni composant, ni contenu Fab.
    fn est_un_jeu(&self) -> bool {
        self.complete
            && self.parent_installation_id.as_deref().unwrap_or_default().is_empty()
            && !self.is_fab
            && !self.dir.is_empty()
            && !self.artifact_id.is_empty()
    }

    /// Identifiant canonique des deeplinks `install` / `uninstall` (cf. [`full_app_id`]).
    fn triplet(&self) -> Option<String> {
        (!self.namespace.is_empty() && !self.item_id.is_empty())
            .then(|| format!("{}%3A{}%3A{}", self.namespace, self.item_id, self.artifact_id))
    }
}

/// Scanne les jeux Epic installés.
///
/// 🔑 Deux launchers Epic coexistent depuis octobre 2026. L'ancien décrit chaque jeu dans
/// un manifeste `*.item` ; le nouveau, en préversion, tient un journal et, quand on
/// l'installe, reprend les jeux puis désinstalle l'ancien. L'ancien reste téléchargeable :
/// on lit les deux.
pub fn scan() -> Vec<GameDto> {
    let mut games = scan_journal();
    for game in scan_manifests() {
        if !games.iter().any(|g| g.id == game.id) {
            games.push(game);
        }
    }
    games
}

/// Jeux de l'ancien launcher, à partir de ses manifestes JSON `*.item`.
fn scan_manifests() -> Vec<GameDto> {
    let Some(dir) = manifests_dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut games = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(item) = serde_json::from_str::<EpicItem>(&text) else {
            continue;
        };

        // On ne garde que les jeux de base installés (pas les DLC / composants).
        let is_base = item.main_game_app_name.is_empty() || item.main_game_app_name == item.app_name;
        if item.incomplete || !is_base || item.install_location.is_empty() {
            continue;
        }
        // Manifeste potentiellement périmé : Epic ne supprime pas toujours le `.item` quand
        // le jeu est désinstallé ou son dossier déplacé/effacé (ex. Palia laissait un manifeste
        // pointant vers un dossier disparu → jeu marqué installé à tort). On n'accepte l'entrée
        // que si le dossier d'installation existe réellement ; sinon le jeu reste visible en tant
        // que possédé/non installé via le compte Epic.
        if !std::path::Path::new(&item.install_location).is_dir() {
            continue;
        }

        // Cible de lancement = AppName Epic → deeplink `com.epicgames.launcher://`.
        // On lance TOUJOURS via le launcher (et non l'exe en direct) : certains jeux
        // exigent le SDK Epic Online Services actif (ex. UNCHARTED échoue en direct),
        // et rien dans le manifeste ne permet de les distinguer de façon fiable de ceux
        // qui tourneraient en standalone. Passer par Epic garantit un lancement propre.
        let launch_target = item.app_name.clone();

        games.push(GameDto {
            id: format!("epic:{}", item.app_name),
            title: item.display_name,
            platform: "epic".into(),
            installed: true,
            size_gb: GameDto::bytes_to_gb(item.install_size),
            install_dir: Some(item.install_location),
            launch_target,
            ..Default::default()
        });
    }
    games
}

/// Jeux du nouveau launcher, d'après son journal `installations.jsonl`.
///
/// Le journal ne note ni le nom ni la taille des jeux. Le nom vient de la copie de
/// l'ancien manifeste que le launcher garde pour les jeux qu'il a repris, sinon du dossier
/// d'installation : un titre provisoire, que celui du compte Epic remplace (`merge_owned`).
fn scan_journal() -> Vec<GameDto> {
    let Some(dir) = eglv2_dir() else {
        return Vec::new();
    };
    journal(&dir)
        .into_iter()
        // Même filet que pour les manifestes : un dossier disparu n'est plus un jeu installé.
        .filter(|inst| Path::new(&inst.dir).is_dir())
        .map(|inst| {
            let titre = titre_repris(&dir, &inst.installation_id);
            GameDto {
                id: format!("epic:{}", inst.artifact_id),
                provisional_title: titre.is_none(),
                title: titre.unwrap_or_else(|| nom_du_dossier(&inst)),
                platform: "epic".into(),
                installed: true,
                size_gb: GameDto::bytes_to_gb(taille(&inst)),
                install_dir: Some(inst.dir.clone()),
                launch_target: inst.artifact_id.clone(),
                ..Default::default()
            }
        })
        .collect()
}

/// Installations vivantes du nouveau launcher (vide sans journal lisible).
fn journal(dir: &Path) -> Vec<Installation> {
    fs::read_to_string(dir.join("installations.jsonl"))
        .map(|text| rejouer(&text))
        .unwrap_or_default()
}

/// Rejoue le journal, comme le launcher le fait lui-même.
///
/// Chaque écriture ajoute une ligne : la dernière d'une installation l'emporte, une ligne
/// de suppression la retire. La première ligne est un en-tête (`version`, `entries`) sans
/// identifiant, écartée comme toute ligne illisible. On ne garde que les jeux de base, et
/// une seule installation par jeu : la dernière écrite.
fn rejouer(text: &str) -> Vec<Installation> {
    let mut vivantes: HashMap<String, (usize, Installation)> = HashMap::new();
    for (rang, ligne) in text.lines().enumerate() {
        let Ok(inst) = serde_json::from_str::<Installation>(ligne) else {
            continue;
        };
        if inst.supprimee() {
            vivantes.remove(&inst.installation_id);
        } else {
            vivantes.insert(inst.installation_id.clone(), (rang, inst));
        }
    }

    let mut par_jeu: HashMap<String, (usize, Installation)> = HashMap::new();
    for (rang, inst) in vivantes.into_values().filter(|(_, inst)| inst.est_un_jeu()) {
        if par_jeu.get(&inst.artifact_id).is_none_or(|(autre, _)| *autre < rang) {
            par_jeu.insert(inst.artifact_id.clone(), (rang, inst));
        }
    }
    let mut jeux: Vec<(usize, Installation)> = par_jeu.into_values().collect();
    jeux.sort_by_key(|(rang, _)| *rang);
    jeux.into_iter().map(|(_, inst)| inst).collect()
}

/// Nom d'un jeu repris de l'ancien launcher : le nouveau garde une copie de son manifeste,
/// nommée d'après l'installation (`import-scan\v1-items\<installationId>.item`).
fn titre_repris(dir: &Path, installation_id: &str) -> Option<String> {
    let path = dir
        .join("import-scan")
        .join("v1-items")
        .join(format!("{installation_id}.item"));
    let item: EpicItem = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let titre = item.display_name.trim();
    (!titre.is_empty()).then(|| titre.to_string())
}

fn nom_du_dossier(inst: &Installation) -> String {
    Path::new(&inst.dir)
        .file_name()
        .map(|nom| nom.to_string_lossy().into_owned())
        .unwrap_or_else(|| inst.artifact_id.clone())
}

/// Taille sur disque d'une installation du nouveau launcher, qui ne la note nulle part : on
/// additionne le dossier. Une fois par installation et par version, puis gardée en mémoire,
/// car ce relevé local est rejoué à chaque exécutable inconnu qui démarre (`procwatch`).
fn taille(inst: &Installation) -> u64 {
    static TAILLES: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);
    let cle = format!("{}|{}|{}", inst.installation_id, inst.version, inst.dir);
    if let Some(octets) = TAILLES.lock().ok().and_then(|t| t.as_ref()?.get(&cle).copied()) {
        return octets;
    }
    let octets = super::dir_size(Path::new(&inst.dir));
    if let Ok(mut t) = TAILLES.lock() {
        t.get_or_insert_with(HashMap::new).insert(cle, octets);
    }
    octets
}

/// Chemin de l'`EpicGamesLauncher.exe`, lu depuis le handler de protocole enregistré
/// (`HKCR\com.epicgames.launcher\shell\open\command` = `"…\EpicGamesLauncher.exe" "%1"`).
/// Sert à démarrer le launcher avec un deeplink en argument (fiable même launcher fermé).
/// Le nouveau launcher reprend ce protocole et le nom de l'exécutable.
pub fn launcher_exe() -> Option<PathBuf> {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(r"com.epicgames.launcher\shell\open\command")
        .ok()?;
    let cmd: String = key.get_value("").ok()?;
    // Premier token entre guillemets = chemin de l'exécutable.
    let exe = PathBuf::from(cmd.split('"').nth(1)?);
    exe.is_file().then_some(exe)
}

/// Identifiant **canonique complet** d'un jeu Epic pour les deeplinks autres que `launch`.
///
/// Le launcher tolère le simple `AppName` (artifactId) pour `action=launch`, mais pour
/// `action=uninstall` (comme pour `install`) il attend l'identifiant triple
/// `namespace:catalogItemId:artifactId` — sinon le deeplink est reçu (Epic s'ouvre) mais
/// aucun jeu n'est ciblé. On le retrouve dans le manifeste `*.item` (ancien launcher) ou le
/// journal (nouveau) dont l'`AppName` correspond, et on le renvoie avec les deux-points
/// **encodés `%3A`** (format des liens « Install » officiels d'Epic).
pub fn full_app_id(app_name: &str) -> Option<String> {
    triplet_manifestes(app_name).or_else(|| {
        journal(&eglv2_dir()?)
            .iter()
            .find(|inst| inst.artifact_id == app_name)?
            .triplet()
    })
}

fn triplet_manifestes(app_name: &str) -> Option<String> {
    let dir = manifests_dir()?;
    for entry in fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("item") {
            continue;
        }
        let Ok(item) = fs::read_to_string(&path).and_then(|t| {
            serde_json::from_str::<EpicItem>(&t).map_err(std::io::Error::other)
        }) else {
            continue;
        };
        if item.app_name == app_name && !item.catalog_namespace.is_empty() && !item.catalog_item_id.is_empty()
        {
            return Some(format!(
                "{}%3A{}%3A{}",
                item.catalog_namespace, item.catalog_item_id, item.app_name
            ));
        }
    }
    None
}

/// Identifiant pour le deeplink `action=install` : le triplet
/// `namespace%3AcatalogItemId%3AappName`. Résolu depuis le manifeste `.item` ou le journal
/// (jeu déjà partiellement présent) sinon depuis le cache des jeux possédés écrit au scan du
/// compte (`epic_install_ids.json`) ; repli sur l'AppName seul si rien n'est trouvé (Epic
/// s'ouvrira au moins). `config_dir` = dossier de config de l'app.
pub fn install_id(app_name: &str, config_dir: &std::path::Path) -> String {
    full_app_id(app_name)
        .or_else(|| owned_install_id(app_name, config_dir))
        .unwrap_or_else(|| app_name.to_string())
}

/// Triplet d'installation d'un jeu **possédé** (non installé), mis en cache au dernier scan
/// du compte Epic (accounts/epic.rs) sous forme `{ appName: "ns%3Acid%3Aapp" }`.
fn owned_install_id(app_name: &str, config_dir: &std::path::Path) -> Option<String> {
    let txt = fs::read_to_string(config_dir.join("epic_install_ids.json")).ok()?;
    let map: std::collections::HashMap<String, String> = serde_json::from_str(&txt).ok()?;
    map.get(app_name).cloned()
}

fn manifests_dir() -> Option<PathBuf> {
    let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dir = PathBuf::from(program_data)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    dir.is_dir().then_some(dir)
}

/// Dossier de données du nouveau launcher (`%LOCALAPPDATA%\com.epicgames.eglv2`).
fn eglv2_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("com.epicgames.eglv2");
    dir.is_dir().then_some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN_TETE: &str = r#"{"version":1,"writtenAt":"2026-10-21T18:15:35Z","entries":3}"#;

    /// Une installation complète, telle que le launcher l'écrit ; `extra` ajoute des champs.
    fn ligne(id: &str, artifact: &str, extra: &str) -> String {
        format!(
            r#"{{"installationId":"{id}","complete":true,"dir":"E:\\Jeux\\{artifact}","platform":"Windows","namespace":"ns{artifact}","itemId":"item{artifact}","artifactId":"{artifact}","tags":[""],"sidecar":{{"revision":0}},"launchExecutable":"{artifact}.exe","writtenAt":"2026-10-21T16:03:11Z"{extra}}}"#
        )
    }

    fn suppression(id: &str) -> String {
        format!(r#"{{"installationId":"{id}","deleted":true,"writtenAt":"2026-10-21T18:00:00Z"}}"#)
    }

    fn rejoue(lignes: &[String]) -> Vec<Installation> {
        let mut text = EN_TETE.to_string();
        for l in lignes {
            text.push('\n');
            text.push_str(l);
        }
        rejouer(&text)
    }

    fn jeux(insts: &[Installation]) -> Vec<&str> {
        insts.iter().map(|i| i.artifact_id.as_str()).collect()
    }

    #[test]
    fn l_en_tete_et_les_lignes_illisibles_sont_ecartees() {
        let insts = rejoue(&[ligne("A", "Calluna", ""), "pas du json".into(), "[1, 2]".into(), String::new()]);
        assert_eq!(jeux(&insts), ["Calluna"]);
        assert_eq!(insts[0].dir, r"E:\Jeux\Calluna");
    }

    /// Une mise à jour réécrit l'installation : seule la dernière ligne compte.
    #[test]
    fn la_derniere_ligne_d_une_installation_l_emporte() {
        let insts = rejoue(&[
            ligne("A", "Calluna", r#","version":"1.32""#),
            ligne("A", "Calluna", r#","version":"1.33""#),
        ]);
        assert_eq!(insts.len(), 1);
        assert_eq!(insts[0].version, "1.33");
    }

    /// Désinstaller ajoute une ligne de suppression ; réinstaller ensuite fait revenir le jeu.
    #[test]
    fn une_suppression_retire_l_installation() {
        assert!(rejoue(&[ligne("A", "Calluna", ""), suppression("A")]).is_empty());
        let reinstalle = rejoue(&[ligne("A", "Calluna", ""), suppression("A"), ligne("A", "Calluna", "")]);
        assert_eq!(jeux(&reinstalle), ["Calluna"]);
        // `deleted: false` n'est pas une suppression.
        let faux = rejoue(&[ligne("A", "Calluna", r#","deleted":false"#)]);
        assert_eq!(jeux(&faux), ["Calluna"]);
    }

    /// Fortnite s'installe en quatre morceaux : le jeu et trois composants rattachés.
    #[test]
    fn seuls_les_jeux_de_base_complets_comptent() {
        let insts = rejoue(&[
            ligne("FN", "Fortnite", ""),
            ligne("STW", "aa31f9e9", r#","parentInstallationId":"FN""#),
            ligne("FAB", "PackDeRochers", r#","isFab":true"#),
            ligne("EN_COURS", "Jackal", r#","complete":false"#).replace(r#""complete":true,"#, ""),
        ]);
        assert_eq!(jeux(&insts), ["Fortnite"]);
    }

    /// Deux installations vivantes du même jeu : le launcher garde la dernière écrite.
    #[test]
    fn un_jeu_installe_deux_fois_garde_la_derniere_installation() {
        let ancienne = ligne("A", "Calluna", "");
        let nouvelle = ligne("B", "Calluna", "").replace(r"E:\\Jeux", r"F:\\Jeux");
        let insts = rejoue(&[ancienne, nouvelle]);
        assert_eq!(insts.len(), 1);
        assert_eq!(insts[0].installation_id, "B");
        assert_eq!(insts[0].dir, r"F:\Jeux\Calluna");
    }

    #[test]
    fn le_triplet_reprend_namespace_item_et_artifact() {
        let insts = rejoue(&[ligne("A", "Calluna", "")]);
        assert_eq!(insts[0].triplet().as_deref(), Some("nsCalluna%3AitemCalluna%3ACalluna"));
        let sans_namespace = rejoue(&[ligne("A", "Calluna", "").replace(r#""namespace":"nsCalluna","#, "")]);
        assert_eq!(sans_namespace[0].triplet(), None);
    }
}
