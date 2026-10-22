//! Accueil de la Boutique de bout en bout (rayons Steam → identifiants et prix ITAD).
//! `cargo run --example vitrine [FR|US|JP…] [fr|en]` (depuis src-tauri).
//!
//! Travaille dans un dossier temporaire : les caches de l'application installée ne sont
//! pas touchés.

use ludo_lib::metadata::vitrine::{self, Rayon, ShelfItem};

fn ligne(j: &ShelfItem) -> String {
    let prix = match (j.is_free, j.price) {
        (true, _) => "gratuit".to_string(),
        (_, Some(p)) => format!("{p:.2} {} (-{}%) {}{}", j.currency, j.savings, j.store_name, if j.record { " ★" } else { "" }),
        _ => "—".into(),
    };
    let avis = j.review_percent.map(|p| format!("{p} %")).unwrap_or_else(|| "pas d'avis".into());
    format!(
        "  {:<40} {prix:<40} {avis:<10} [{}] {}",
        j.title.chars().take(40).collect::<String>(),
        j.tags.join(", "),
        if j.game_id.is_empty() { "(inconnu d'ITAD)" } else { "" }
    )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let region = args.next().unwrap_or_else(|| "FR".into());
    let langue = args.next().unwrap_or_else(|| "fr".into());
    ludo_lib::locale::definir(&langue, &region);
    let dir = std::env::temp_dir().join("ludo-exemple-vitrine");

    let t = std::time::Instant::now();
    let accueil = vitrine::home(&dir, &Default::default());
    println!("Accueil ({region}, {langue}) en {:.1} s", t.elapsed().as_secs_f32());
    println!("\nVEDETTE ({})", accueil.featured.len());
    accueil.featured.iter().for_each(|j| println!("{}", ligne(j)));
    println!("\nMEILLEURES VENTES ({})", accueil.top_sellers.len());
    accueil.top_sellers.iter().for_each(|j| println!("{}", ligne(j)));
    println!("\nPROMOS DU MOMENT ({})", accueil.deals.len());
    for d in accueil.deals.iter().take(6) {
        println!("  {:<40} {:.2} (-{}%) {} {}", d.title, d.price, d.savings, d.store_name, d.expiry.as_deref().unwrap_or(""));
    }
    println!("\nOFFERTS ({})", accueil.giveaways.len());
    for d in &accueil.giveaways {
        println!("  {:<40} {} jusqu'au {}", d.title, d.store_name, d.expiry.as_deref().unwrap_or("?"));
    }

    for (nom, rayon) in [
        ("NOUVEAUTÉS", Rayon::Nouveautes),
        ("À VENIR", Rayon::AVenir),
        ("RPG", Rayon::Genre(122)),
        ("FREE-TO-PLAY", Rayon::Gratuits),
    ] {
        let t = std::time::Instant::now();
        let jeux = vitrine::shelf(rayon, &dir, &Default::default());
        println!("\n{nom} ({}) en {:.1} s", jeux.len(), t.elapsed().as_secs_f32());
        jeux.iter().take(8).for_each(|j| println!("{}", ligne(j)));
    }

    // Deuxième ouverture : les identifiants ITAD viennent du cache disque.
    let t = std::time::Instant::now();
    let _ = vitrine::home(&dir, &Default::default());
    println!("\nAccueil, 2e ouverture : {:.1} s", t.elapsed().as_secs_f32());
}
