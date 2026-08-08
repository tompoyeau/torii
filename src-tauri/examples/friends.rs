//! Valide la liste d'amis Steam + présence contre le vrai compte.
//! `cargo run --example friends` (depuis src-tauri).

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");
    let creds = ludo_lib::accounts::secrets::load(&dir);

    let Some(steam_id) = creds.steam_id.clone() else {
        println!("Pas de SteamID stocké (connecte Steam dans l'app d'abord).");
        return;
    };
    let Some(cookie) = creds.steam_community.clone().or(creds.steam_login_secure.clone()) else {
        println!("Pas de cookie de session Steam stocké.");
        return;
    };
    println!("SteamID: {steam_id}\nRécupération des amis…\n");

    let friends = ludo_lib::accounts::steam::friends(&steam_id, &cookie);
    if friends.is_empty() {
        println!("0 ami (liste privée ? cookie expiré ?).");
        return;
    }

    let count = |s: &str| friends.iter().filter(|f| f.state == s).count();
    let online = friends.iter().filter(|f| f.state != "offline").count();
    println!(
        "=== {} amis · {} en jeu · {} en ligne ({} hors ligne) ===\n",
        friends.len(),
        count("in-game"),
        online - count("in-game"),
        count("offline"),
    );

    for f in friends.iter().filter(|f| f.state != "offline") {
        let game = f.game_name.as_deref().map(|g| format!("  → {g}")).unwrap_or_default();
        println!("  [{:<7}] {}{}", f.state, f.name, game);
    }
}
