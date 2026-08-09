//! Vérifie que `family_owners` (nb de copies dans la famille Steam) est bien peuplé.
//! `cargo run --example family_copies` (depuis src-tauri).

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    let games = ludo_lib::accounts::owned_games(&dir);
    let mut multi: Vec<_> = games.iter().filter(|g| g.family_owners.len() >= 2).collect();
    multi.sort_by(|a, b| b.family_owners.len().cmp(&a.family_owners.len()));

    println!(
        "{} jeux Steam · {} avec ≥2 copies famille\n",
        games.iter().filter(|g| g.platform == "steam").count(),
        multi.len(),
    );
    for g in multi.iter().take(15) {
        println!("  {:<44} {} copies (moi: {})", g.title, g.family_owners.len(), g.owned);
    }
}
