//! Vérifie la récupération des jeux possédés via la session de login stockée.
//! `cargo run --example session`

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");
    let creds: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("credentials.json")).unwrap())
            .unwrap();
    let cookie = creds["steamLoginSecure"]
        .as_str()
        .expect("aucune session Steam stockée — connecte-toi d'abord dans l'app");

    println!("Récupération de la bibliothèque possédée…");
    let games = ludo_lib::accounts::steam::owned_from_session(&dir, cookie);
    println!("=> {} jeux possédés récupérés\n", games.len());
    for g in games.iter().take(15) {
        println!("  {}", g.title);
    }
}
