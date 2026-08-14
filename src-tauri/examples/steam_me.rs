//! Valide la récupération du profil Steam de l'utilisateur (pseudo + avatar).
//! `cargo run --example steam_me` (depuis src-tauri).

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    match ludo_lib::accounts::steam_me(&dir) {
        Some(p) => {
            println!("=== Profil Steam ===");
            println!("  Pseudo : {}", p.name);
            println!("  SteamID: {}", p.steam_id);
            println!("  Avatar : {}", p.avatar_url);
            println!("  Profil : {}", p.profile_url);
        }
        None => println!("Aucun profil (Steam non connecté ou profil inaccessible)."),
    }
}
