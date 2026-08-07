//! Valide le remplissage de la métadonnée IGDB contre la vraie bibliothèque.
//! `cargo run --example genres` (depuis src-tauri). Réchauffe aussi le cache.

use std::collections::{BTreeMap, HashMap};

fn main() {
    let appdata = std::env::var("APPDATA").expect("APPDATA introuvable");
    let dir = std::path::Path::new(&appdata).join("com.tompo.ludo");

    println!("Scan de la bibliothèque…");
    let games = ludo_lib::platforms::scan_all(Some(&dir));
    println!("{} jeux au total\n", games.len());

    let done = std::cell::Cell::new(0usize);
    let updates = ludo_lib::metadata::igdb::fill_metadata(&games, &dir, |batch| {
        done.set(done.get() + batch.len());
        println!("  … +{} (total {})", batch.len(), done.get());
    });

    let by_id: HashMap<String, ludo_lib::metadata::igdb::IgdbMeta> =
        updates.iter().cloned().collect();

    // Couverture par plateforme (match IGDB) + richesse (jaquette / description).
    let mut per_plat: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let (mut with_cover, mut with_desc, mut with_genre) = (0usize, 0usize, 0usize);
    for g in &games {
        let e = per_plat.entry(g.platform.clone()).or_default();
        e.1 += 1;
        if let Some(m) = by_id.get(&g.id) {
            e.0 += 1;
            if m.cover_url.is_some() {
                with_cover += 1;
            }
            if m.description.is_some() {
                with_desc += 1;
            }
            if m.genre.is_some() {
                with_genre += 1;
            }
        }
    }
    println!("\n=== Couverture IGDB par plateforme ===");
    for (p, (ok, tot)) in &per_plat {
        println!("  {:<10} {:>4}/{:<4}", p, ok, tot);
    }
    println!("  TOTAL      {:>4}/{:<4}", updates.len(), games.len());
    println!("\n=== Richesse (sur {} matchés) ===", updates.len());
    println!("  genre       {}", with_genre);
    println!("  jaquette    {}", with_cover);
    println!("  description {}", with_desc);

    // Distribution des genres.
    let mut dist: BTreeMap<String, usize> = BTreeMap::new();
    for (_, m) in &updates {
        if let Some(g) = &m.genre {
            *dist.entry(g.clone()).or_default() += 1;
        }
    }
    let mut v: Vec<_> = dist.into_iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("\n=== Genres ({} catégories) ===", v.len());
    for (g, c) in &v {
        println!("  {:<30} {}", g, c);
    }

    // Contrôle ciblé de jeux hors-Steam connus.
    let want = [
        "fortnite",
        "valorant",
        "world of warcraft",
        "diablo",
        "hearthstone",
        "rocket league",
    ];
    println!("\n=== Contrôles hors-Steam ===");
    for g in &games {
        let t = g.title.to_lowercase();
        if want.iter().any(|w| t.contains(w)) {
            match by_id.get(&g.id) {
                Some(m) => println!(
                    "  [{:<9}] {:<34} genre={:<10} jaq={} desc={}",
                    g.platform,
                    g.title,
                    m.genre.clone().unwrap_or_else(|| "-".into()),
                    if m.cover_url.is_some() { "✓" } else { "-" },
                    if m.description.is_some() { "✓" } else { "-" },
                ),
                None => println!("  [{:<9}] {:<34} (aucun match)", g.platform, g.title),
            }
        }
    }
}
