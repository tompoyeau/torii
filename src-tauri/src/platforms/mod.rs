pub mod epic;
pub mod favorites;
pub mod gog;
pub mod hidden;
pub mod manual;
pub mod playhistory;
pub mod riot;
pub mod steam;
pub mod ubisoft;

use crate::models::GameDto;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Agrège les jeux de toutes les plateformes : d'abord les jeux **installés**
/// (fichiers locaux), puis les jeux **possédés** en ligne (comptes) et manuels.
pub fn scan_all(config_dir: Option<&Path>) -> Vec<GameDto> {
    // 1. Jeux installés, indexés par id.
    let mut map: HashMap<String, GameDto> = steam::scan()
        .into_iter()
        .chain(epic::scan())
        .chain(gog::scan())
        .chain(riot::scan())
        .chain(ubisoft::scan())
        .map(|g| (g.id.clone(), g))
        .collect();

    // Bibliothèque possédée Ubisoft (cache local, sans login ni API) : complète les
    // installés (et leur donne une jaquette, absente du scan registre).
    for owned in ubisoft::owned() {
        merge_owned(&mut map, owned);
    }

    if let Some(dir) = config_dir {
        // 2. Jeux possédés (comptes) : complètent la liste sans écraser l'installé.
        for owned in crate::accounts::owned_games(dir) {
            merge_owned(&mut map, owned);
        }
        // 3. Jeux ajoutés manuellement.
        for game in manual::scan(dir) {
            map.insert(game.id.clone(), game);
        }
    }

    let mut games: Vec<GameDto> = map.into_values().collect();
    games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

    // Marque les jeux masqués (exclusion) et favoris (le front s'appuie dessus pour ses filtres).
    if let Some(dir) = config_dir {
        let excluded = hidden::load(dir);
        let favorites = favorites::load(dir);
        if !excluded.is_empty() || !favorites.is_empty() {
            for game in &mut games {
                game.hidden = excluded.contains(&game.id);
                game.favorite = favorites.contains(&game.id);
            }
        }

        // Dernière session « maison » (clic sur Jouer dans Torii) : comble les jeux sans
        // date du launcher (Riot/EA/Battle.net…) et l'emporte si plus récente que celle du launcher.
        let history = playhistory::load(dir);
        if !history.is_empty() {
            for game in &mut games {
                if let Some(&ours) = history.get(&game.id) {
                    game.last_played = Some(game.last_played.map_or(ours, |cur| cur.max(ours)));
                }
            }
        }
    }
    games
}

/// Fusionne un jeu possédé : s'il est déjà installé, on le marque « possédé »,
/// on récupère le temps de jeu et les visuels (le scan local Epic/GOG ne fournit
/// pas de jaquette — seul le compte en ligne l'a) ; sinon on l'ajoute (non installé).
fn merge_owned(map: &mut HashMap<String, GameDto>, mut owned: GameDto) {
    match map.get_mut(&owned.id) {
        Some(existing) => {
            existing.owned = true;
            if existing.playtime_minutes.is_none() {
                existing.playtime_minutes = owned.playtime_minutes;
            }
            if existing.last_played.is_none() {
                existing.last_played = owned.last_played;
            }
            // Visuels : un jeu installé (scan local) n'a souvent pas de jaquette ;
            // on récupère celle du compte en ligne. On n'écrase jamais l'existant.
            if existing.cover_url.is_none() {
                existing.cover_url = owned.cover_url.take();
            }
            if existing.hero_url.is_none() {
                existing.hero_url = owned.hero_url.take();
            }
        }
        None => {
            map.insert(owned.id.clone(), owned);
        }
    }
}

/// Lance un jeu selon sa plateforme et sa cible.
pub fn launch(platform: &str, target: &str) -> Result<(), String> {
    match platform {
        "steam" => open_uri(&format!("steam://rungameid/{target}")),
        // Epic : toujours via le launcher (deeplink). Le lancement direct de l'exe casse
        // les jeux à SDK Epic Online Services (UNCHARTED…) et rien ne permet de les repérer.
        "epic" => launch_epic(target),
        // GOG : un jeu installé a un chemin d'exécutable ; un jeu possédé non
        // installé n'a que son id produit (numérique) → on ouvre GOG Galaxy dessus.
        "gog" if target.chars().all(|c| c.is_ascii_digit()) => {
            open_uri(&format!("goggalaxy://openGameView/{target}"))
        }
        // Ubisoft Connect : URI de lancement (ouvre le launcher sur le jeu).
        "ubisoft" => open_uri(&format!("uplay://launch/{target}/0")),
        // EA (app EA / ex-Origin) : lance (ou installe) via l'app EA.
        "ea" => open_uri(&format!("origin2://game/launch?offerIds={target}&autoDownload=1")),
        // Battle.net : deeplink protocole (ouvre le client sur le jeu, installé ou non).
        "battlenet" => open_uri(&format!("battlenet://{target}/")),
        // Riot : on lance via RiotClientServices.exe --launch-product=<id>.
        "riot" => {
            let client = riot::client_path().ok_or("Riot Client introuvable.")?;
            Command::new(&client)
                .args(["--launch-product", target, "--launch-patchline", "live"])
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Impossible de lancer le jeu Riot : {e}"))
        }
        // GOG installé et jeux manuels : on lance directement l'exécutable.
        _ => launch_executable(target),
    }
}

/// Déclenche la **désinstallation** d'un jeu installé. On délègue systématiquement à l'UI
/// native du launcher (qui affiche sa propre confirmation et supprime les fichiers) plutôt
/// que de toucher au disque nous-mêmes — c'est l'approche de Playnite, et rien n'est jamais
/// supprimé sans que l'utilisateur valide dans la fenêtre du launcher.
pub fn uninstall(platform: &str, target: &str, install_dir: Option<&str>) -> Result<(), String> {
    match platform {
        // Steam ouvre sa propre boîte de dialogue de désinstallation.
        "steam" => open_uri(&format!("steam://uninstall/{target}")),
        // Epic Games Launcher : action `uninstall` sur l'app (confirmation dans le launcher).
        // ⚠️ Contrairement à `launch`, l'uninstall exige l'identifiant canonique COMPLET
        // (`namespace:catalogItemId:artifactId`) : avec le simple AppName, Epic s'ouvre mais
        // ne cible aucun jeu. On résout le triplet depuis le manifeste (repli AppName seul).
        "epic" => {
            let app_id = epic::full_app_id(target).unwrap_or_else(|| target.to_string());
            open_uri(&format!("com.epicgames.launcher://apps/{app_id}?action=uninstall"))
        }
        // Ubisoft Connect : URI de désinstallation (ouvre le launcher sur la confirmation).
        "ubisoft" => open_uri(&format!("uplay://uninstall/{target}")),
        // App EA (ex-Origin) : désinstallation via l'app EA.
        "ea" => open_uri(&format!("origin2://game/uninstall?offerIds={target}")),
        // GOG Galaxy n'expose pas de deeplink de désinstallation : on lance l'uninstaller
        // Inno Setup (`unins000.exe`) présent dans le dossier d'installation (il confirme).
        "gog" => {
            let dir = install_dir.ok_or("Dossier d'installation GOG inconnu.")?;
            let uninstaller = Path::new(dir).join("unins000.exe");
            if uninstaller.exists() {
                Command::new(&uninstaller)
                    .current_dir(dir)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("Impossible de lancer la désinstallation GOG : {e}"))
            } else {
                // Repli : ouvre le dossier pour une désinstallation manuelle.
                tauri_plugin_opener::open_path(dir, None::<&str>)
                    .map_err(|e| format!("Désinstallateur GOG introuvable ({e}))"))
            }
        }
        // Battle.net / Riot : pas de mécanisme de désinstallation par jeu fiable — on ouvre
        // le dossier d'installation pour laisser l'utilisateur passer par l'outil du launcher.
        _ => match install_dir {
            Some(dir) => tauri_plugin_opener::open_path(dir, None::<&str>)
                .map_err(|e| format!("Impossible d'ouvrir le dossier du jeu : {e}")),
            None => Err(format!("Désinstallation non prise en charge pour {platform}.")),
        },
    }
}

/// Lance un jeu Epic via le deeplink `com.epicgames.launcher://` (jeux en ligne à
/// anti-cheat/EOS qui exigent le launcher, ou jeux possédés non installés).
///
/// Subtilité : en `tauri dev`, l'app tourne dans un **job object** créé par cargo (pour la
/// tuer au Ctrl+C) qui interdit le breakaway. Tout process lancé directement par l'app y est
/// piégé et le bootstrapper de l'Epic Games Launcher meurt aussitôt. On tente donc d'abord un
/// spawn **détaché** (qui marche pour l'app installée, hors job), et en cas d'échec on demande
/// au **service WMI** — extérieur à notre job — de créer le process, qui survit alors.
fn launch_epic(app_name: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;
    let uri = format!("com.epicgames.launcher://apps/{app_name}?action=launch&silent=true");
    let Some(exe) = epic::launcher_exe() else {
        return open_uri(&uri); // repli : handler de protocole (app hors job)
    };
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

    // Chemin rapide (app installée, hors job) : spawn détaché direct.
    let direct = Command::new(&exe)
        .arg(&uri)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if direct.is_ok() {
        return Ok(());
    }

    // Repli en cas d'échec du spawn direct.
    // - En RELEASE (app installée) il n'existe aucun job object → il suffit d'ouvrir le
    //   deeplink via `ShellExecuteW` (comme un clic sur un lien). On évite ainsi tout appel
    //   PowerShell/WMI, dont la seule présence dans le binaire est un motif « malware »
    //   flaggé par Windows Defender (création de process via WMI).
    // - En DEV (`tauri dev`), l'app tourne dans un job object qui interdit le breakaway : le
    //   seul moyen de faire survivre le bootstrapper Epic est de créer le process HORS du job,
    //   via le service WMI. Ce chemin n'est donc compilé qu'en debug.
    #[cfg(not(debug_assertions))]
    {
        return open_uri(&uri);
    }
    #[cfg(debug_assertions)]
    {
        return launch_via_wmi(&exe, &uri);
    }
}

/// Crée un process via `Win32_Process.Create` du **service WMI** : le process obtenu n'est
/// PAS dans notre job object, il survit donc au confinement de `tauri dev`. Utilisé en repli
/// quand le spawn direct est refusé (breakaway interdit).
///
/// ⚠️ Compilé **uniquement en debug** : en release, l'appel PowerShell/WMI est absent du binaire
/// (motif heuristique « malware » pour Windows Defender), et il n'y sert à rien (pas de job object).
#[cfg(debug_assertions)]
fn launch_via_wmi(exe: &Path, uri: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // exe et uri ne contiennent pas d'apostrophe → sûrs dans une chaîne PowerShell simple.
    let command_line = format!("\"{}\" \"{}\"", exe.display(), uri);
    let script = format!(
        "Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{CommandLine='{command_line}'}} | Out-Null"
    );
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Lancement Epic via WMI impossible : {e}"))
}

/// Ouvre une URI de protocole enregistré (steam://, com.epicgames.launcher://…) via
/// `ShellExecuteW` — exactement comme un clic sur un lien.
///
/// Pourquoi pas `cmd /C start` ni `Command::spawn` directement :
/// - `cmd /C start` casse sur le `&` du deeplink Epic (`…launch&silent=true`), traité
///   comme un séparateur de commandes.
/// - un `spawn` direct depuis notre app GUI (sans console) transmet des handles standard
///   invalides au processus enfant ; le bootstrapper de l'Epic Games Launcher meurt alors
///   aussitôt sans rien lancer. `ShellExecuteW` n'hérite pas des handles et détache le
///   processus proprement.
fn open_uri(uri: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // shell32!ShellExecuteW — renvoie une valeur > 32 en cas de succès.
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut core::ffi::c_void,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> isize;
    }

    let to_wide = |s: &str| -> Vec<u16> { OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect() };
    let verb = to_wide("open");
    let file = to_wide(uri);

    // SW_SHOWNORMAL = 1.
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if code > 32 {
        Ok(())
    } else {
        Err(format!("Impossible d'ouvrir {uri} (ShellExecuteW code {code})"))
    }
}

/// Lance une cible locale depuis son propre dossier. Le **répertoire de travail** est
/// toujours celui du fichier : beaucoup de jeux (et de scripts de lancement) référencent
/// leurs binaires en chemin relatif — ex. le `U4Launch.bat` d'UNCHARTED fait `start u4.exe`
/// et `.\Prerequisites\…`, donc sans le bon CWD Windows renvoie « u4.exe introuvable ».
///
/// - `.exe` : spawn direct.
/// - `.bat`/`.cmd` : via `cmd /c` (CreateProcess ne sait pas exécuter un script batch).
/// - autre (`.lnk`, `.url`…) : ShellExecute (le raccourci porte sa propre cible + dossier).
fn launch_executable(exe: &str) -> Result<(), String> {
    let path = Path::new(exe);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    let dir = path.parent();

    let mut cmd = match ext.as_deref() {
        Some("exe") => Command::new(path),
        Some("bat") | Some("cmd") => {
            // On passe le nom de fichier (pas le chemin complet) et on fixe le CWD au dossier
            // du script → évite les pièges de guillemets de `cmd /c` et donne le bon CWD.
            // CREATE_NO_WINDOW : pas de fenêtre console qui clignote au lancement.
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let name = path.file_name().unwrap_or(path.as_os_str());
            let mut c = Command::new("cmd");
            c.arg("/c").arg(name).creation_flags(CREATE_NO_WINDOW);
            c
        }
        _ => {
            return tauri_plugin_opener::open_path(exe, None::<&str>)
                .map_err(|e| format!("Impossible de lancer {exe} : {e}"));
        }
    };
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Impossible de lancer {exe} : {e}"))
}

/// Taille totale d'un dossier (métadonnées uniquement, récursif).
pub(crate) fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            match entry.metadata() {
                Ok(md) if md.is_dir() => total += dir_size(&entry.path()),
                Ok(md) => total += md.len(),
                Err(_) => {}
            }
        }
    }
    total
}

/// Extrait, ligne par ligne, la valeur associée à une clé dans un texte VDF/ACF.
/// Les lignes ont la forme `"clé"\t\t"valeur"`.
pub(crate) fn vdf_first(text: &str, key: &str) -> Option<String> {
    vdf_all(text, key).into_iter().next()
}

/// Extrait toutes les valeurs associées à une clé (ex: tous les "path").
pub(crate) fn vdf_all(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split('"').collect();
        // tokens: [avant, clé, entre, valeur, …]
        if tokens.len() >= 4 && tokens[1].eq_ignore_ascii_case(key) {
            out.push(tokens[3].to_string());
        }
    }
    out
}
