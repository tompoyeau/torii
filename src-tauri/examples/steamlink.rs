//! Vérifie le cycle « visible par mes amis Steam » : lier, délier, relier.
//!   1. `cd server && npx wrangler dev --port 8791 --local`
//!   2. `TORII_API=http://127.0.0.1:8791 cargo run --example steamlink`
use ludo_lib::social;

fn etat(dir: &std::path::Path) -> String {
    match social::me(dir) {
        Some(a) => format!(
            "steamId={:<20} découvrable={}",
            a.steam_id.unwrap_or_else(|| "(aucun)".into()),
            a.steam_discoverable
        ),
        None => "(non connecté)".into(),
    }
}

fn main() {
    let dir = std::env::temp_dir().join(format!("torii-steamlink-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mail = format!("lien{n}@example.com");

    let code = social::request_code(&mail).unwrap().expect("mode dev");
    social::verify(&dir, &mail, &code).unwrap();
    println!("départ        : {}", etat(&dir));

    // Activer : le bouton envoie le SteamID ET la découvrabilité.
    social::set_profile(&dir, None, Some("76561198258753323".into()), Some(true)).unwrap();
    println!("après ON      : {}", etat(&dir));

    // Désactiver : chaîne VIDE = délier. C'est là que `null` échouait silencieusement.
    social::set_profile(&dir, None, Some(String::new()), Some(false)).unwrap();
    println!("après OFF     : {}", etat(&dir));

    // Réactiver, pour vérifier qu'on peut refaire l'aller-retour.
    social::set_profile(&dir, None, Some("76561198258753323".into()), Some(true)).unwrap();
    println!("après ON (2e) : {}", etat(&dir));

    // Modifier le nom seul ne doit RIEN changer au lien Steam.
    social::set_profile(&dir, Some("Nouveau nom".into()), None, None).unwrap();
    println!("après renommage : {}", etat(&dir));

    let _ = std::fs::remove_dir_all(&dir);
}
