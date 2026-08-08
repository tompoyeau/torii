//! Valide le scraper Instant Gaming. `cargo run --example ig` (depuis src-tauri).

fn main() {
    for title in ["Elden Ring", "Hades", "Cyberpunk 2077", "The Witcher 3: Wild Hunt", "Baldur's Gate 3"] {
        match ludo_lib::metadata::instant_gaming::price(title) {
            Some(o) => println!("{title:<28} → {:>6.2}€  (-{}%)  {}", o.price, o.savings, o.url),
            None => println!("{title:<28} → (pas d'offre exacte trouvée)"),
        }
        std::thread::sleep(std::time::Duration::from_millis(600));
    }
}
