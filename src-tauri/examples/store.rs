//! Valide la boutique ITAD : vitrine, recherche, fiche produit.
//! `cargo run --example store` (depuis src-tauri). Nécessite ITAD_API_KEY sur le Worker.

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    println!("=== Vitrine (mises en avant, page 0) ===");
    let items = ludo_lib::metadata::store::deals(0, "featured", &Default::default());
    println!("{} jeux\n", items.len());
    for it in items.iter().take(6) {
        println!(
            "  {:<34} {:>6.2}€ (-{:>2}%) {:<18} {}",
            trunc(&it.title, 34), it.price, it.savings, it.store_name, if it.cover_url.is_some() { "[jaquette]" } else { "" }
        );
    }

    println!("\n=== Recherche « witcher » ===");
    let res = ludo_lib::metadata::store::search("witcher", &Default::default());
    println!("{} résultats\n", res.len());
    for it in res.iter().take(6) {
        println!("  {:<40} {:>6.2}€  id={}", trunc(&it.title, 40), it.price, it.game_id);
    }

    if let Some(first) = res.first() {
        println!("\n=== Fiche produit : {} (id {}) ===", first.title, first.game_id);
        match ludo_lib::metadata::store::game(&first.game_id, &dir) {
            Some(g) => {
                println!("Titre       : {}", g.title);
                println!("Plus bas    : {:?}€", g.cheapest_ever);
                println!("Genre       : {:?}", g.genre);
                println!("Studio      : {:?}", g.developer);
                println!("Desc        : {}", g.description.as_deref().map(|d| trunc(d, 90)).unwrap_or_else(|| "—".into()));
                println!("Hero        : {}", g.hero_url.as_deref().unwrap_or("—"));
                println!("Captures    : {}", g.screenshots.len());
                println!("Comparatif ({} boutiques) :", g.prices.len());
                for p in &g.prices {
                    println!("  {:<18} {:>6.2}€ (au lieu de {:>6.2}€, -{}%)", p.store_name, p.price, p.retail_price, p.savings);
                }
            }
            None => println!("!! fiche introuvable"),
        }
    }
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}
