//! Valide la récupération des succès Steam (scrape page perso + total % global) contre
//! le vrai compte. `cargo run --example steam_ach` (depuis src-tauri).

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");
    let creds = ludo_lib::accounts::secrets::load(&dir);

    let Some(steam_id) = creds.steam_id.clone() else {
        println!("Pas de SteamID stocké."); return;
    };
    let Some(cookie) = creds.steam_community.clone().or(creds.steam_login_secure.clone()) else {
        println!("Pas de cookie de session Steam."); return;
    };
    println!("SteamID: {steam_id}\n");

    for (appid, label) in [
        (1086940u64, "Baldur's Gate 3"),
        (620, "Portal 2"),
        (1091500, "Cyberpunk 2077"),
        (105600, "Terraria"),
    ] {
        let players = ludo_lib::accounts::steam::current_players(appid);
        println!(">>> {label} — joueurs en ce moment : {players:?}");
        match ludo_lib::accounts::steam::achievements(&steam_id, appid, &cookie) {
            None => println!("=== {label} ({appid}) : aucun succès / indisponible ===\n"),
            Some(a) => {
                let pct = if a.total > 0 { a.unlocked * 100 / a.total } else { 0 };
                println!("=== {label} ({appid}) : {}/{} ({pct}%) · {} lignes ===",
                    a.unlocked, a.total, a.items.len());
                for it in a.items.iter().take(4) {
                    let mark = if it.unlocked { "✔" } else { "·" };
                    let icon_ok = it.icon.starts_with("http");
                    println!("  {mark} {}  [icon:{}]  {}",
                        it.name, icon_ok, it.unlocked_at.as_deref().unwrap_or(""));
                    if !it.description.is_empty() {
                        println!("      {}", it.description);
                    }
                }
                // Contrôle : dernier succès (verrouillé attendu).
                if let Some(last) = a.items.last() {
                    println!("  … dernier: {} (unlocked={})", last.name, last.unlocked);
                }
                println!();
            }
        }
    }
}
