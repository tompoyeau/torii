//! Diagnostic de « Trouvés sur ce PC » : `cargo run --example suggestions [-- <dossier>…]`.
//!
//! Montre ce que Torii proposerait d'ajouter : les sous-dossiers des dossiers de jeux
//! donnés (à défaut, ceux retenus dans Torii) et les jeux de la liste de Windows, moins ce
//! que la bibliothèque connaît déjà (cache du dernier relevé).
//!
//! Aucun appel réseau, aucune écriture.

use ludo_lib::platforms::{library_cache, suggestions};
use std::time::Instant;

fn main() {
    let dir = std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("com.tompo.ludo");
    let bibliotheque = library_cache::load(&dir);
    let donnes: Vec<String> = std::env::args().skip(1).collect();
    let dossiers = if donnes.is_empty() { suggestions::dossiers_retenus(&dir) } else { donnes };

    let debut = Instant::now();
    let p = suggestions::proposer_dans(&dir, &bibliotheque, dossiers.clone());
    let duree = debut.elapsed();

    println!("Dossiers : {dossiers:?} — {} jeux en bibliothèque\n", bibliotheque.len());
    for s in &p.jeux {
        println!("  [{:?}] {:<32} {}", s.source, s.title, s.launch_target);
        println!("  {:>10} dossier : {}", "", s.install_dir.as_deref().unwrap_or("—"));
    }
    println!("\n{} propositions, {} ignorées, en {:.0} ms", p.jeux.len(), p.ignores, duree.as_secs_f64() * 1000.0);
}
