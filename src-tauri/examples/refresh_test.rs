//! Valide l'auto-refresh Steam : régénère un cookie web à partir du refresh token
//! stocké, puis récupère la bibliothèque possédée avec. `cargo run --example refresh_test`

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");
    let creds: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("credentials.json")).unwrap())
            .unwrap();

    let rt = creds["steamRefreshToken"]
        .as_str()
        .expect("aucun steamRefreshToken — reconnecte-toi (login frais)");
    let steam_id = creds["steamId"].as_str().expect("aucun steamId");

    println!("Refresh token : {} car | steamId : {steam_id}", rt.len());
    println!("\n1) GenerateAccessTokenForApp → cookie frais…");
    match ludo_lib::accounts::steam::refresh_web_cookie(rt, steam_id) {
        None => println!("   ❌ ÉCHEC (AccessDenied ou token invalide) — repli à prévoir."),
        Some(cookie) => {
            println!("   ✅ Cookie régénéré : {}…", &cookie[..cookie.len().min(45)]);
            println!("\n2) Bibliothèque via ce cookie frais…");
            let games = ludo_lib::accounts::steam::owned_from_community(steam_id, &cookie);
            println!("   => {} jeux possédés récupérés", games.len());
            for g in games.iter().take(10) {
                println!("      {}", g.title);
            }
        }
    }
}
