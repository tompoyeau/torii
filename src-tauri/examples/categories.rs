//! Mesure les catégories (tags Steam) d'une vraie bibliothèque.
//!
//! `cargo run --example categories -- <dossier>`
//!
//! ⚠️ `<dossier>` doit être une COPIE du dossier de config de Torii : la passe IGDB réécrit
//! ses caches. Seuls `library_cache_v1.json` (la liste des jeux du dernier scan) et les
//! caches de métadonnées servent ; aucun compte n'est rescanné, donc aucun jeton (GOG et
//! Epic les font tourner) n'est consommé dans la copie à la place de l'original.

use ludo_lib::metadata::categories::{self, Categories};
use ludo_lib::metadata::igdb::{self, IgdbMeta};
use ludo_lib::models::GameDto;
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("usage : categories <copie du dossier de config>"));
    let texte = std::fs::read_to_string(dir.join("library_cache_v1.json")).expect("library_cache_v1.json introuvable");
    let games: Vec<GameDto> = serde_json::from_str::<Vec<GameDto>>(&texte)
        .expect("liste de jeux illisible")
        .into_iter()
        .filter(|g| !g.hidden)
        .collect();
    println!("{} jeux visibles", games.len());

    let debut = Instant::now();
    let ids: Vec<&str> = games.iter().map(|g| g.id.as_str()).collect();
    let mut cats = Categories::charger(&ids, &dir);
    println!("tags des jeux Steam : {:.1} s", debut.elapsed().as_secs_f32());

    let lots = std::cell::Cell::new(0usize);
    let metas: HashMap<String, IgdbMeta> = igdb::fill_metadata(&games, &dir, |lot| {
        lots.set(lots.get() + lot.len());
        if lots.get() % 120 < lot.len() {
            println!("  IGDB … {} fiches ({:.0} s)", lots.get(), debut.elapsed().as_secs_f32());
        }
    })
    .into_iter()
    .collect();
    println!("passe IGDB : {:.1} s", debut.elapsed().as_secs_f32());
    cats.completer(games.iter().filter_map(|g| categories::appid(&g.id, metas.get(&g.id))));
    println!("tout compris : {:.1} s\n", debut.elapsed().as_secs_f32());

    // D'où viennent les catégories de chaque jeu, par plateforme.
    let mut par_plateforme: BTreeMap<&str, [usize; 4]> = BTreeMap::new(); // total, steam, igdb, rien
    let mut compte: HashMap<String, usize> = HashMap::new();
    let mut sans: Vec<String> = Vec::new();
    for g in &games {
        let meta = metas.get(&g.id);
        let tags = cats.du_jeu(&g.id, meta);
        let ligne = par_plateforme.entry(g.platform.as_str()).or_default();
        ligne[0] += 1;
        let par_steam = categories::appid(&g.id, meta).is_some_and(|_| {
            // Même résultat que `du_jeu` sans le repli IGDB : vrai si Steam a des tags.
            !cats.du_jeu(&g.id, meta.map(|m| IgdbMeta { classement: None, ..m.clone() }).as_ref()).is_empty()
        });
        match (tags.is_empty(), par_steam) {
            (true, _) => {
                ligne[3] += 1;
                sans.push(format!("{} ({})", g.title, g.platform));
            }
            (false, true) => ligne[1] += 1,
            (false, false) => ligne[2] += 1,
        }
        for t in tags {
            *compte.entry(t).or_default() += 1;
        }
    }
    println!("{:<10} {:>6} {:>7} {:>6} {:>6}", "", "jeux", "Steam", "IGDB", "rien");
    for (p, [n, s, i, r]) in &par_plateforme {
        println!("{p:<10} {n:>6} {s:>7} {i:>6} {r:>6}");
    }

    let mut liste: Vec<_> = compte.into_iter().collect();
    liste.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let au_moins_3 = liste.iter().filter(|(_, n)| *n >= 3).count();
    println!("\n{} catégories, dont {} avec au moins 3 jeux", liste.len(), au_moins_3);
    println!("{}", liste.iter().take(50).map(|(t, n)| format!("{n} {t}")).collect::<Vec<_>>().join(", "));

    println!("\nSans catégorie ({}) : {}", sans.len(), sans.join(", "));

    println!("\nContrôles :");
    for titre in ["VALORANT", "League of Legends", "Fortnite", "World of Warcraft", "Hades", "The Witcher 3", "Cyberpunk 2077"] {
        for g in games.iter().filter(|g| g.title.to_lowercase().starts_with(&titre.to_lowercase())) {
            println!("  [{}] {} : {}", g.platform, g.title, cats.du_jeu(&g.id, metas.get(&g.id)).join(", "));
        }
    }
}
