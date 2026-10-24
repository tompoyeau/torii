//! Boutique : parcourir par catégories, de bout en bout (Steam → identifiants et prix ITAD).
//! `cargo run --example parcourir [tag tag…]` (depuis src-tauri ; défaut : Coop + Survie).
//!
//! Travaille dans un dossier temporaire : les caches de l'application installée ne sont
//! pas touchés.

use ludo_lib::metadata::vitrine::{self, TriParcours};

fn main() {
    ludo_lib::locale::definir("fr", "FR");
    let dir = std::env::temp_dir().join("ludo-exemple-parcourir");
    let mut tags: Vec<u64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    if tags.is_empty() {
        tags = vec![1685, 1662];
    }

    let menu = vitrine::tags_boutique(&dir);
    println!("{} catégories au menu : {} …", menu.len(), menu.iter().take(20).map(|t| t.nom.as_str()).collect::<Vec<_>>().join(", "));
    let noms: Vec<&str> = tags
        .iter()
        .filter_map(|id| menu.iter().find(|t| t.id == *id).map(|t| t.nom.as_str()))
        .collect();
    println!("\nRecherche : {}", noms.join(" + "));

    for (libelle, tri) in [("MEILLEURES VENTES", TriParcours::Ventes), ("MIEUX NOTÉS", TriParcours::Notes), ("NOUVEAUTÉS", TriParcours::Nouveautes)] {
        let t = std::time::Instant::now();
        let Some(p) = vitrine::parcourir(&tags, tri, 0, &dir, &Default::default()) else {
            println!("\n{libelle} : panne");
            continue;
        };
        let tarifes = p.jeux.iter().filter(|j| j.price.is_some() || j.is_free).count();
        println!("\n{libelle} : {} jeux au total, page de {} ({} avec un prix) en {:.1} s", p.total, p.jeux.len(), tarifes, t.elapsed().as_secs_f32());
        for j in p.jeux.iter().take(6) {
            let prix = match (j.is_free, j.price) {
                (true, _) => "gratuit".to_string(),
                (_, Some(prix)) => format!("{prix:.2} {} chez {}", j.currency, j.store_name),
                _ => "—".into(),
            };
            println!("  {:<40} {:<32} {}", j.title.chars().take(40).collect::<String>(), prix, j.review_percent.map(|p| format!("{p} %")).unwrap_or_default());
        }
    }

    let t = std::time::Instant::now();
    if let Some(p) = vitrine::parcourir(&tags, TriParcours::Ventes, 1, &dir, &Default::default()) {
        println!("\nPage 2 : {} jeux en {:.1} s ({})", p.jeux.len(), t.elapsed().as_secs_f32(), p.jeux.iter().take(4).map(|j| j.title.as_str()).collect::<Vec<_>>().join(", "));
    }
}
