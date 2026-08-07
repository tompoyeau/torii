//! Vérifie owned_from_community (page → jeton WebAPI → GetOwnedGames).
//! `cargo run --example community`

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");
    let creds: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("credentials.json")).unwrap())
            .unwrap();
    let cookie = creds["steamCommunity"]
        .as_str()
        .expect("aucune session communautaire — reconnecte-toi");
    let steam_id = creds["steamId"].as_str().expect("steamId manquant");

    println!("Récupération via page communautaire + GetOwnedGames…");
    let games = ludo_lib::accounts::steam::owned_from_community(steam_id, cookie);
    println!("=> {} jeux (jeux-only)\n", games.len());
    let mut sorted = games;
    sorted.sort_by_key(|g| std::cmp::Reverse(g.playtime_minutes));
    for g in sorted.iter().take(12) {
        let h = g.playtime_minutes.unwrap_or(0) as f64 / 60.0;
        println!("  {:<45} {:>6.1} h", g.title, h);
    }
}
