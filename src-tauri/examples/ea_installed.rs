//! Valide la détection d'installation EA (IS.json d'EA Desktop) contre la vraie machine.
//! `cargo run --example ea_installed`.

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    let games = ludo_lib::accounts::ea::load_library(&dir);
    if games.is_empty() {
        println!("Bibliothèque EA vide (non connecté ?).");
        return;
    }
    let installed: Vec<_> = games.iter().filter(|g| g.installed).collect();
    println!("=== {} jeux EA · {} installés ===\n", games.len(), installed.len());
    for g in &games {
        let mark = if g.installed { "✔ installé" } else { "· possédé" };
        let dir = g.install_dir.as_deref().map(|d| format!("  → {d}")).unwrap_or_default();
        let size = if g.size_gb > 0.0 { format!(" [{:.1} Go]", g.size_gb) } else { String::new() };
        println!("{mark}  {}{size}{dir}", g.title);
    }
}
