//! Valide le scraper Instant Gaming. `cargo run --example ig` (depuis src-tauri).
//!
//! Titres tels que le comparateur les donne, avec l'appid Steam : quand ils diffèrent du
//! nom chez Instant Gaming, c'est le nom Steam qui doit prendre le relais.

fn main() {
    for (title, appid) in [
        ("Elden Ring", 1245620),
        ("Hades", 1145360),
        ("Cyberpunk 2077", 1091500),
        ("The Witcher 3: Wild Hunt", 292030),
        ("Baldur's Gate 3", 1086940),
        // Le comparateur ajoute « Restored Land » ; Steam et IG disent « Dying Light: The Beast ».
        ("Dying Light: The Beast Restored Land", 3008130),
    ] {
        match ludo_lib::metadata::instant_gaming::price(title, Some(appid)) {
            Some(o) => println!("{title:<38} → {:>6.2}€  (-{}%)  {}", o.price, o.savings, o.url),
            None => println!("{title:<38} → (pas d'offre exacte trouvée)"),
        }
        std::thread::sleep(std::time::Duration::from_millis(600));
    }
}
