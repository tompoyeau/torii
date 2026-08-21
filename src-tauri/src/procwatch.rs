//! Détection des jeux qui tournent — **y compris lancés hors de Torii**.
//!
//! Sert à dater « Récemment joué » avec précision : jusqu'ici seule une partie lancée
//! depuis Torii était datée (au clic sur « Jouer »), donc une session démarrée depuis
//! Steam, le bureau ou un raccourci n'apparaissait jamais.
//!
//! # Pourquoi pas `sysinfo`
//!
//! Mesuré sur une machine réelle (370 process) : un rafraîchissement complet `sysinfo`
//! avec les chemins coûte **~11,9 ms**, et l'ancien suivi de session le faisait toutes
//! les 3 secondes *pendant la partie* — exactement au moment où il ne faut rien coûter.
//!
//! Ici un tick, c'est : un `EnumProcesses` (un appel système qui rend un simple tableau
//! de PID) puis la résolution du chemin des seuls PID **apparus depuis le tick précédent**
//! — en régime de croisière, zéro ou un. Mesuré à **0,42 ms par tick**, soit 28× moins
//! que l'approche précédente, et moins de 0,01 % d'un cœur au rythme retenu.

use crate::models::GameDto;
use crate::platforms::playhistory;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

/// Rythme au repos : un jeu qui démarre doit apparaître vite dans « Récemment joué ».
const POLL_IDLE: Duration = Duration::from_secs(5);
/// Rythme pendant une partie : on n'attend plus qu'une fermeture, autant se faire oublier.
const POLL_IN_GAME: Duration = Duration::from_secs(15);
/// Au-delà, un « Jouer » cliqué dans Torii dont aucun process n'est apparu est abandonné
/// (jeu qui ne démarre pas, exécutable hors du dossier connu…).
const ARM_TIMEOUT: Duration = Duration::from_secs(120);

/// Un jeu détectable : les chemins dont l'apparition d'un process signe une partie.
struct Target {
    id: String,
    /// Titre affichable, repris tel quel dans la présence publiée aux amis.
    title: String,
    /// Dossier d'installation et/ou exécutable, normalisés (comparaison façon Windows).
    roots: Vec<String>,
}

/// Une partie en cours : les process qui la composent et l'instant où elle a commencé.
#[derive(Default)]
struct Running {
    pids: HashSet<u32>,
    /// Démarrage réel du premier process (`GetProcessTimes`), pas l'instant de détection.
    since: i64,
}

#[derive(Default)]
pub struct WatchState {
    targets: Vec<Target>,
    /// Jeux en cours, par id.
    running: HashMap<String, Running>,
    /// Jeu lancé DEPUIS Torii avec l'option « revenir à la fermeture » : à sa fermeture,
    /// on restaure la fenêtre et le front rouvre sa fiche.
    armed: Option<(String, Instant)>,
}

/// État partagé du surveillant (géré par Tauri).
#[derive(Default)]
pub struct Watch(pub Mutex<WatchState>);

/// Émis quand un jeu est détecté en cours d'exécution (lancé d'où que ce soit).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameLaunched {
    id: String,
    at: i64,
}

/// Émis quand un jeu suivi se ferme (→ le front rouvre sa fiche).
#[derive(Clone, Serialize)]
struct GameExited {
    id: String,
}

/// Reconstruit la liste des jeux à surveiller à partir d'un scan de bibliothèque.
/// Appelé après chaque `scan_library` : un jeu installé entre-temps devient détectable.
pub fn set_targets(app: &tauri::AppHandle, games: &[GameDto]) {
    let targets = targets_from(games);
    if let Ok(mut state) = app.state::<Watch>().0.lock() {
        state.targets = targets;
    }
}

/// Traduit une bibliothèque en cibles détectables.
fn targets_from(games: &[GameDto]) -> Vec<Target> {
    games
        .iter()
        .filter_map(|g| {
            let mut roots: Vec<String> = Vec::new();
            if let Some(dir) = g.install_dir.as_deref().filter(|d| !d.is_empty()) {
                roots.push(normalize(dir));
            }
            // Jeu manuel sans dossier renseigné : l'exécutable lui-même fait la cible.
            if roots.is_empty() && g.launch_target.to_lowercase().ends_with(".exe") {
                roots.push(normalize(&g.launch_target));
            }
            (!roots.is_empty()).then(|| Target {
                id: g.id.clone(),
                title: g.title.clone(),
                roots,
            })
        })
        .collect()
}

/// Id du jeu auquel appartient un exécutable, s'il y en a un.
fn match_id(targets: &[Target], exe: &str) -> Option<String> {
    let path = normalize(exe);
    targets
        .iter()
        .find(|t| t.roots.iter().any(|r| under(&path, r)))
        .map(|t| t.id.clone())
}

/// Partie en cours : `(id, titre, depuis)`. Utilisé par le battement de cœur de la
/// présence. `None` si aucun jeu ne tourne — ou si plusieurs tournent, auquel cas on
/// prend celui commencé en dernier (c'est celui devant lequel la personne est).
pub fn current_game(app: &tauri::AppHandle) -> Option<(String, String, i64)> {
    let state = app.state::<Watch>();
    let state = state.0.lock().ok()?;
    let (id, running) = state.running.iter().max_by_key(|(_, r)| r.since)?;
    let title = state
        .targets
        .iter()
        .find(|t| &t.id == id)
        .map(|t| t.title.clone())
        .unwrap_or_default();
    Some((id.clone(), title, running.since))
}

/// Secondes écoulées depuis la dernière action clavier/souris de l'utilisateur.
/// Sert à passer en « absent » plutôt que d'afficher « en ligne » devant un PC désert.
pub fn idle_seconds() -> u64 {
    sys::idle_seconds()
}

/// Diagnostic : quels jeux de `games` tournent en ce moment. Même rapprochement que la
/// surveillance continue, mais en une passe — sert à vérifier la détection sur une
/// machine réelle (`cargo run --example watch`).
pub fn running_now(games: &[GameDto]) -> Vec<(String, Option<i64>)> {
    let targets = targets_from(games);
    let mut found: Vec<(String, Option<i64>)> = pids()
        .into_iter()
        .filter_map(|p| Some((match_id(&targets, &exe_path(p)?)?, started_at(p))))
        .collect();
    found.sort();
    found.dedup_by(|a, b| a.0 == b.0);
    found
}

/// Arme le retour automatique : le jeu vient d'être lancé depuis Torii, on rouvrira sa
/// fiche à sa fermeture. La détection du process, elle, est commune à tous les jeux.
pub fn arm(app: &tauri::AppHandle, game_id: String) {
    if let Ok(mut state) = app.state::<Watch>().0.lock() {
        state.armed = Some((game_id, Instant::now()));
    }
}

/// Démarre le fil de surveillance (un seul pour toute l'application).
pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut seen: HashSet<u32> = HashSet::new();
        loop {
            let delay = tick(&app, &mut seen);
            std::thread::sleep(delay);
        }
    });
}

/// Un passage : repère les process apparus et disparus, en déduit les parties.
/// Renvoie le délai avant le prochain passage.
fn tick(app: &tauri::AppHandle, seen: &mut HashSet<u32>) -> Duration {
    let state_handle = app.state::<Watch>();
    let Ok(mut state) = state_handle.0.lock() else {
        return POLL_IDLE;
    };

    // Rien à surveiller (bibliothèque pas encore scannée) : aucun appel système.
    if state.targets.is_empty() {
        return POLL_IDLE;
    }

    let alive: HashSet<u32> = pids().into_iter().collect();

    // 1) Process apparus depuis le dernier passage → est-ce un jeu connu ?
    let mut launched: Vec<(String, Option<i64>)> = Vec::new();
    for pid in alive.difference(seen) {
        let Some(exe) = exe_path(*pid) else { continue };
        let Some(id) = match_id(&state.targets, &exe) else { continue };
        let first = !state.running.contains_key(&id);
        let started = started_at(*pid).unwrap_or_else(|| now_unix());
        let entry = state.running.entry(id.clone()).or_default();
        entry.pids.insert(*pid);
        if first {
            entry.since = started;
            // Date = démarrage RÉEL du process, pas l'instant où on le remarque. Ça
            // couvre les 5 s de latence du sondage, et surtout le cas d'un jeu (ou d'une
            // application permanente type Wallpaper Engine) déjà lancé quand Torii
            // s'ouvre : il est daté de son vrai démarrage, pas de « maintenant ».
            launched.push((id, Some(started)));
        }
    }

    // 2) Process disparus → un jeu dont plus aucun process ne tourne s'est fermé.
    let mut exited: Vec<String> = Vec::new();
    state.running.retain(|id, running| {
        running.pids.retain(|p| alive.contains(p));
        if running.pids.is_empty() {
            exited.push(id.clone());
            false
        } else {
            true
        }
    });

    *seen = alive;

    // Le « Jouer » cliqué dans Torii dont rien n'est jamais apparu : on désarme.
    if let Some((id, since)) = &state.armed {
        if since.elapsed() > ARM_TIMEOUT && !state.running.contains_key(id) {
            state.armed = None;
        }
    }
    let armed_id = state.armed.as_ref().map(|(id, _)| id.clone());
    let in_game = !state.running.is_empty();
    drop(state); // aucun verrou tenu pendant les écritures disque et les émissions

    for (id, started) in launched {
        let at = match app.path().app_config_dir() {
            Ok(dir) => match started {
                Some(ts) => playhistory::record_at(&dir, &id, ts).unwrap_or_default(),
                None => playhistory::record(&dir, &id).unwrap_or_default(),
            },
            Err(_) => 0,
        };
        let _ = app.emit("game-launched", GameLaunched { id, at });
    }

    for id in exited {
        // Seul un jeu lancé depuis Torii (option « revenir à la fermeture ») ramène la
        // fenêtre au premier plan : sinon Torii surgirait à la fin de n'importe quelle
        // partie lancée ailleurs.
        if armed_id.as_deref() != Some(id.as_str()) {
            continue;
        }
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.unminimize();
            let _ = win.show();
            let _ = win.set_focus();
            let _ = win.maximize();
        }
        let _ = app.emit("game-exited", GameExited { id });
        if let Ok(mut state) = state_handle.0.lock() {
            state.armed = None;
        }
    }

    if in_game {
        POLL_IN_GAME
    } else {
        POLL_IDLE
    }
}

/// Instant Unix courant, repli quand `GetProcessTimes` ne répond pas.
fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Chemin comparable : minuscules, séparateurs Windows, sans `\` final.
fn normalize(path: &str) -> String {
    let p = path.to_lowercase().replace('/', "\\");
    p.trim_end_matches('\\').to_string()
}

/// `path` est-il la racine elle-même ou un fichier dessous ? Le `\` évite qu'un dossier
/// voisin au nom plus long (« Portal 2 Demo ») ne passe pour le jeu (« Portal 2 »).
fn under(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}\\"))
}

// --- Énumération des process (Win32) --------------------------------------------

#[cfg(windows)]
mod sys {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    /// Droit minimal permettant de lire le chemin d'un process — n'exige aucune élévation.
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    /// `FILETIME` : compteur 64 bits scindé en deux mots de 32 bits.
    #[repr(C)]
    #[derive(Default)]
    pub struct FileTime {
        low: u32,
        high: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn K32EnumProcesses(pids: *mut u32, cb: u32, needed: *mut u32) -> i32;
        fn GetProcessTimes(
            h: Handle,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
        fn QueryFullProcessImageNameW(h: Handle, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
        fn CloseHandle(h: Handle) -> i32;
    }

    /// PID actifs. UN appel système, aucune donnée annexe : c'est ce qui rend le tick
    /// quasi gratuit comparé à un inventaire complet des process.
    pub fn pids() -> Vec<u32> {
        let mut buf = vec![0u32; 4096];
        let mut needed = 0u32;
        let ok = unsafe { K32EnumProcesses(buf.as_mut_ptr(), (buf.len() * 4) as u32, &mut needed) };
        if ok == 0 {
            return Vec::new();
        }
        buf.truncate(needed as usize / std::mem::size_of::<u32>());
        buf
    }

    /// `LASTINPUTINFO` : taille de la structure + tick du dernier événement d'entrée.
    #[repr(C)]
    struct LastInputInfo {
        cb_size: u32,
        time: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetLastInputInfo(info: *mut LastInputInfo) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount() -> u32;
    }

    /// Secondes depuis la dernière frappe ou le dernier mouvement de souris.
    /// Un seul appel système, aucun hook clavier : rien qui ressemble à un enregistreur
    /// de frappe, et rien qui coûte quoi que ce soit.
    pub fn idle_seconds() -> u64 {
        unsafe {
            let mut info = LastInputInfo {
                cb_size: std::mem::size_of::<LastInputInfo>() as u32,
                time: 0,
            };
            if GetLastInputInfo(&mut info) == 0 {
                return 0;
            }
            // Les deux compteurs bouclent tous les 49 jours : la soustraction en `u32`
            // reste juste au passage du tour, contrairement à un calcul en `u64`.
            u64::from(GetTickCount().wrapping_sub(info.time)) / 1000
        }
    }

    /// Instant Unix (secondes) auquel le process a démarré. Rend « Récemment joué »
    /// exact à la seconde plutôt qu'au rythme du sondage.
    pub fn started_at(pid: u32) -> Option<i64> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut creation = FileTime::default();
            let (mut exit, mut kernel, mut user) =
                (FileTime::default(), FileTime::default(), FileTime::default());
            let ok = GetProcessTimes(h, &mut creation, &mut exit, &mut kernel, &mut user);
            CloseHandle(h);
            if ok == 0 {
                return None;
            }
            // FILETIME = intervalles de 100 ns depuis le 1er janvier 1601 ; l'écart avec
            // l'époque Unix est de 11 644 473 600 secondes.
            let ticks = ((creation.high as u64) << 32) | creation.low as u64;
            Some((ticks / 10_000_000) as i64 - 11_644_473_600)
        }
    }

    /// Chemin de l'exécutable d'un PID. `None` si le process a déjà disparu ou s'il est
    /// protégé (process système) — deux cas normaux, sans intérêt ici.
    pub fn exe_path(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 512];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(h);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..size as usize]))
        }
    }
}

#[cfg(not(windows))]
mod sys {
    pub fn pids() -> Vec<u32> {
        Vec::new()
    }
    pub fn exe_path(_pid: u32) -> Option<String> {
        None
    }
    pub fn started_at(_pid: u32) -> Option<i64> {
        None
    }
    pub fn idle_seconds() -> u64 {
        0
    }
}

use sys::{exe_path, pids, started_at};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_is_windows_friendly() {
        assert_eq!(normalize(r"C:\Jeux\Mon Jeu\"), r"c:\jeux\mon jeu");
        assert_eq!(normalize("C:/Jeux/Mon Jeu"), r"c:\jeux\mon jeu");
    }

    /// Le rapprochement accepte un exe en sous-dossier du jeu, mais doit refuser un
    /// dossier voisin dont le nom commence pareil.
    #[test]
    fn root_prefix_matching() {
        let root = normalize(r"C:\Steam\steamapps\common\Portal 2");
        let m = |p: &str| under(&normalize(p), &root);
        assert!(m(r"C:\Steam\steamapps\common\Portal 2\portal2.exe"));
        assert!(m(r"C:\Steam\steamapps\common\Portal 2\bin\win64\game.exe"));
        assert!(!m(r"C:\Steam\steamapps\common\Portal 2 Demo\portal2.exe"));
        assert!(!m(r"C:\Steam\steamapps\common\Autre Jeu\jeu.exe"));
    }
}
