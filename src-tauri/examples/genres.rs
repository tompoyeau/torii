//! Valide le remplissage des genres via IGDB contre la vraie bibliothèque.
//! `cargo run --example genres` (depuis src-tauri). Réchauffe aussi le cache.

use std::collections::{BTreeMap, HashMap};

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    println!("Scan de la bibliothèque…");
    let games = ludo_lib::platforms::scan_all(Some(&dir));
    println!("{} jeux au total\n", games.len());

    let done = std::cell::Cell::new(0usize);
    let updates = ludo_lib::metadata::igdb::fill_genres(&games, &dir, |batch| {
        done.set(done.get() + batch.len());
        println!("  … +{} (total {})", batch.len(), done.get());
    });

    let by_id: HashMap<String, String> = updates.iter().cloned().collect();

    // Couverture par plateforme.
    let mut per_plat: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // (avec genre, total)
    for g in &games {
        let e = per_plat.entry(g.platform.clone()).or_default();
        e.1 += 1;
        if by_id.contains_key(&g.id) {
            e.0 += 1;
        }
    }
    println!("\n=== Couverture par plateforme ===");
    for (p, (ok, tot)) in &per_plat {
        println!("  {:<10} {:>4}/{:<4}", p, ok, tot);
    }
    println!(
        "  TOTAL      {:>4}/{:<4}",
        updates.len(),
        games.len()
    );

    // Distribution des genres.
    let mut dist: BTreeMap<String, usize> = BTreeMap::new();
    for (_, genre) in &updates {
        *dist.entry(genre.clone()).or_default() += 1;
    }
    let mut v: Vec<_> = dist.into_iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("\n=== Genres ({} catégories) ===", v.len());
    for (g, c) in &v {
        println!("  {:<30} {}", g, c);
    }

    // Vérif ciblée de jeux hors-Steam connus.
    let want = [
        "fortnite",
        "valorant",
        "world of warcraft",
        "overwatch",
        "diablo",
        "hearthstone",
        "rocket league",
    ];
    println!("\n=== Contrôles hors-Steam ===");
    for g in &games {
        let t = g.title.to_lowercase();
        if want.iter().any(|w| t.contains(w)) {
            println!(
                "  [{:<9}] {:<40} -> {}",
                g.platform,
                g.title,
                by_id.get(&g.id).map(|s| s.as_str()).unwrap_or("(aucun)")
            );
        }
    }
}
