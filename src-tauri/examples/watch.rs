//! Diagnostic de la détection de parties : `cargo run --release --example watch`.
//! Lit le cache de bibliothèque (aucun appel réseau).
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let dir = std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.tompo.ludo");
    let games = ludo_lib::platforms::library_cache::load(&dir);
    let avec_dossier = games.iter().filter(|g| g.install_dir.is_some()).count();
    println!("bibliothèque : {} jeux, dont {avec_dossier} détectables (dossier connu)", games.len());

    let t = Instant::now();
    let running = ludo_lib::procwatch::running_now(&games);
    println!("passe complète (tous les PID résolus) : {:.2} ms\n", t.elapsed().as_secs_f64() * 1000.0);

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    if running.is_empty() {
        println!("aucun jeu en cours d'exécution détecté");
    }
    for (id, started) in &running {
        let titre = games.iter().find(|g| &g.id == id).map(|g| g.title.as_str()).unwrap_or("?");
        match started {
            Some(ts) => {
                let mins = (now - ts) / 60;
                println!("  {titre:<24} démarré il y a {}h{:02}  (t={ts})", mins / 60, mins % 60);
            }
            None => println!("  {titre:<24} heure de démarrage indisponible"),
        }
    }
}
