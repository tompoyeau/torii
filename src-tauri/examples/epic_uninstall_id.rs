// Example jetable : vérifie la résolution de l'identifiant canonique Epic (triplet)
// utilisé par le deeplink d'uninstall, à partir de l'AppName d'un jeu installé.
// `cargo run --example epic_uninstall_id`
fn main() {
    let games = ludo_lib::platforms::epic::scan();
    if games.is_empty() {
        println!("Aucun jeu Epic installé détecté.");
        return;
    }
    for g in &games {
        let app_name = &g.launch_target;
        match ludo_lib::platforms::epic::full_app_id(app_name) {
            Some(id) => println!(
                "OK  {:<40} -> com.epicgames.launcher://apps/{id}?action=uninstall",
                g.title
            ),
            None => println!("MISS {:<40} (repli AppName seul: {app_name})", g.title),
        }
    }
}
