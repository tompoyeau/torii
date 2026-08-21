//! Test du client social contre un serveur local :
//!   1. `cd server && npx wrangler dev --port 8787 --local`
//!   2. `TORII_API=http://127.0.0.1:8787 cargo run --example social`
//! Utilise un dossier de config jetable : les vrais identifiants ne sont pas touchés.
use ludo_lib::social;

fn main() {
    let dir = std::env::temp_dir().join(format!("torii-social-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let email = format!("client{stamp}@example.com");

    println!("── Sans session : me() doit être None → {:?}", social::me(&dir).is_none());

    let code = social::request_code(&email).expect("demande de code").expect("mode dev attendu");
    println!("── Code reçu (mode dev) : {code}");

    println!("── Mauvais code : {:?}", social::verify(&dir, &email, "000000").unwrap_err());

    let signin = social::verify(&dir, &email, &code).expect("connexion");
    let account = signin.account;
    println!(
        "── Connecté : {} | code d'ami {} | compte créé : {}",
        account.display_name, account.friend_code, signin.created
    );

    // Le jeton doit être rangé chiffré, comme les autres secrets.
    let creds = ludo_lib::accounts::secrets::load(&dir);
    println!("── Jeton persisté : {}", creds.torii_token.is_some());
    let raw = std::fs::read(dir.join("credentials.dat")).unwrap();
    let token = creds.torii_token.clone().unwrap();
    let leaked = raw.windows(token.len()).any(|w| w == token.as_bytes());
    println!("── Jeton lisible en clair sur le disque : {leaked}");

    println!("── me() : {:?}", social::me(&dir).map(|a| a.email));

    let renamed = social::set_profile(&dir, Some("Testeur".into()), Some("76561198000000042".into()), Some(true)).unwrap();
    println!("── Profil : {} | steam {:?} | découvrable {}", renamed.display_name, renamed.steam_id, renamed.steam_discoverable);

    let neuf = social::rotate_code(&dir).unwrap();
    println!("── Code d'ami régénéré : {} → {}", account.friend_code, neuf);

    println!("── Code d'ami inexistant : {:?}", social::invite(&dir, "ZZZZZZZZ").unwrap_err());

    let presence = social::Presence {
        status: "in-game".into(),
        game_key: Some("igdb:1942".into()),
        game_title: Some("The Witcher 3: Wild Hunt".into()),
        since: Some(stamp as i64 - 3600),
    };
    let circle = social::publish(&dir, &presence).unwrap();
    println!("── Présence publiée, cercle reçu : {} amis, {} demandes", circle.friends.len(), circle.incoming.len());

    social::clear_presence(&dir).unwrap();
    println!("── Présence effacée");

    social::logout(&dir).unwrap();
    println!("── Déconnecté, jeton effacé : {}", ludo_lib::accounts::secrets::load(&dir).torii_token.is_none());
    println!("── me() après déconnexion : {:?}", social::me(&dir).is_none());

    let _ = std::fs::remove_dir_all(&dir);
}
