//! Valide la wishlist enrichie de prix de bout en bout (Steam appids → ITAD).
//! `cargo run --example wishlist_prices` (depuis src-tauri).

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    let appids = ludo_lib::accounts::steam_wishlist_appids(&dir);
    println!("Wishlist : {} appids\n", appids.len());
    if appids.is_empty() {
        println!("(vide — Steam non connecté ou wishlist inaccessible)");
        return;
    }

    let items = ludo_lib::metadata::store::wishlist(&appids, &Default::default());
    let priced = items.iter().filter(|i| i.price.is_some()).count();
    let on_sale = items.iter().filter(|i| i.savings > 0).count();
    println!("{} items · {priced} avec prix · {on_sale} en promo\n", items.len());

    for it in items.iter().take(20) {
        let price = it
            .price
            .map(|p| format!("{p:.2}€ (-{}%) {}", it.savings, it.store_name))
            .unwrap_or_else(|| "—".into());
        let low = it.history_low.map(|l| format!(" | bas: {l:.2}€")).unwrap_or_default();
        let title = if it.title.is_empty() { format!("app {}", it.app_id) } else { it.title.clone() };
        println!("  {:<38} {price}{low}", title.chars().take(38).collect::<String>());
    }
}
