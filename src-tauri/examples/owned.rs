//! Test des jeux possédés Steam (API en ligne) :
//! `cargo run --example owned <CLE_API_STEAM> [steamid]`
//! La clé gratuite se récupère sur https://steamcommunity.com/dev/apikey

fn main() {
    let key = std::env::args()
        .nth(1)
        .expect("usage: cargo run --example owned <CLE_API_STEAM> [steamid]");
    let id = std::env::args()
        .nth(2)
        .or_else(ludo_lib::accounts::steam::detect_steam_id)
        .expect("SteamID introuvable — passe-le en 2e argument");

    println!("SteamID: {id}\n");
    let games = ludo_lib::accounts::steam::owned_games(&key, &id);
    println!("=== {} jeux possédés ===\n", games.len());

    let mut sorted = games;
    sorted.sort_by(|a, b| b.playtime_minutes.cmp(&a.playtime_minutes));
    for g in sorted.iter().take(40) {
        let h = g.playtime_minutes.unwrap_or(0) as f64 / 60.0;
        println!("{:<48} {:>6.1} h", g.title, h);
    }
}
