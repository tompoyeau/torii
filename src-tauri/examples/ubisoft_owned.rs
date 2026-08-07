//! Test manuel de la bibliothèque possédée Ubisoft (cache local `configurations`) :
//! `cargo run --example ubisoft_owned` (depuis src-tauri).

fn main() {
    let owned = ludo_lib::platforms::ubisoft::owned();
    println!("=== {} jeux Ubisoft possédés (base, hors DLC) ===\n", owned.len());
    for g in &owned {
        let cover = if g.cover_url.is_some() { "🖼" } else { "—" };
        println!("[{:>7}] {:<50} {}", g.launch_target, g.title, cover);
    }
}
