//! Test de l'enrichissement des métadonnées (appels réseau réels) :
//! `cargo run --example enrich` (depuis src-tauri).

fn main() {
    let cache_dir = std::env::temp_dir().join("ludo-meta-test");
    let mut games = ludo_lib::platforms::scan_all(None);
    println!("Enrichissement de {} jeux…\n", games.len());
    ludo_lib::metadata::enrich(&mut games, &cache_dir, |done, total| {
        if done % 10 == 0 || done == total {
            println!("  … {done}/{total}");
        }
    });

    for g in &games {
        println!(
            "{:<45} {:<22} {}  hero={}",
            g.title,
            g.genre.as_deref().unwrap_or("—"),
            g.year.map(|y| y.to_string()).unwrap_or_else(|| "----".into()),
            if g.hero_url.is_some() { "oui" } else { "non" },
        );
    }
    println!("\nCache écrit dans {}", cache_dir.display());
}
