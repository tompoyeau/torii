//! Vérifie la résolution de Battle.net.exe (via le protocole `battlenet://`) et montre la
//! commande de lancement qui sera utilisée. `cargo run --example bnet_launch`.
//! N'exécute RIEN — affiche seulement ce qui serait lancé.

fn main() {
    match ludo_lib::accounts::battlenet::launcher_exe() {
        None => {
            println!("❌ Battle.net.exe introuvable (protocole battlenet:// non enregistré ?).");
            println!("   → repli sur le deeplink `battlenet://<code>/` (ne marche que client fermé).");
        }
        Some(exe) => {
            println!("✔ Battle.net.exe trouvé :\n    {}", exe.display());
            println!("\nExemple de commande utilisée pour « Jouer » Diablo IV (code Fen) :");
            println!("    \"{}\" --exec=\"launch Fen\"", exe.display());
            println!("\n(marche que le client soit ouvert ou fermé)");
        }
    }
}
