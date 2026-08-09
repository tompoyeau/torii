//! Valide le fix « suivi du Location » : un compte à URL personnalisée
//! (/profiles/<id> → /id/<vanity>) doit désormais renvoyer sa liste d'amis.
//! Page publique → pas besoin de cookie. `cargo run --example vanity_friends`.

fn main() {
    // lenyben (« Sterben ») : profil public avec URL personnalisée.
    let steam_id = "76561198206344635";
    let friends = ludo_lib::accounts::steam::friends(steam_id, "");
    println!("Amis récupérés pour {steam_id} : {}", friends.len());
    for f in friends.iter().take(8) {
        println!("  [{:<7}] {}", f.state, f.name);
    }
    if friends.is_empty() {
        println!("❌ VIDE — le fix ne marche pas (ou liste privée).");
    } else {
        println!("✅ OK — la redirection vanity est bien suivie.");
    }
}
