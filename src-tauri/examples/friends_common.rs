//! Valide `friends_games::compute` de bout en bout contre le vrai compte.
//! `cargo run --example friends_common` (depuis src-tauri).

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    let Some(data) = ludo_lib::accounts::friends_games::compute(&dir, true) else {
        println!("Steam non connecté (pas de compute).");
        return;
    };

    let readable = data.friends.iter().filter(|f| !f.private).count();
    let private = data.friends.len() - readable;
    println!(
        "=== {} amis ({} lisibles · {} privés) · {} jeux en commun (≥1 ami) ===\n",
        data.friends.len(),
        readable,
        private,
        data.games.len(),
    );

    println!("-- Amis les plus compatibles --");
    for f in data.friends.iter().filter(|f| !f.private).take(8) {
        println!("  {:<24} {} en commun", f.name, f.common_count);
    }

    println!("\n-- Jeux les plus partagés --");
    for g in data.games.iter().take(12) {
        println!("  {:<40} {} amis", g.title, g.owners.len());
    }

    // Exemple d'intersection multi-amis (2 premiers amis lisibles).
    let sel: Vec<String> = data
        .friends
        .iter()
        .filter(|f| !f.private)
        .take(2)
        .map(|f| f.steam_id.clone())
        .collect();
    if sel.len() == 2 {
        let inter = data
            .games
            .iter()
            .filter(|g| sel.iter().all(|s| g.owners.contains(s)))
            .count();
        println!("\n-- Intersection des 2 premiers amis lisibles + moi : {inter} jeux --");
    }
}
