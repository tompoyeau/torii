//! Fiche produit de la Boutique de bout en bout : comparateur + Steam + bas de fiche.
//! `cargo run --example fiche [id ITAD] [FR|US…] [fr|en]` (depuis src-tauri).
//! Sans id : Cyberpunk 2077. Dossier temporaire : les caches de l'application installée
//! ne sont pas touchés.

use ludo_lib::metadata::{store, vitrine};

fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "018d937f-2997-7131-b8b9-7c8af4825fa8".into());
    let region = args.next().unwrap_or_else(|| "FR".into());
    let langue = args.next().unwrap_or_else(|| "fr".into());
    ludo_lib::locale::definir(&langue, &region);
    let dir = std::env::temp_dir().join("ludo-exemple-fiche");

    let t = std::time::Instant::now();
    let Some(g) = store::game(&id, &dir) else {
        println!("Fiche introuvable.");
        return;
    };
    println!("{} — fiche en {:.1} s", g.title, t.elapsed().as_secs_f32());
    println!("  appid {:?} · sortie {:?} · dev {:?} · éditeur {:?}", g.app_id, g.release_date, g.developers, g.publishers);
    println!("  notes : {:?}", g.reviews.iter().map(|r| format!("{} {}", r.source, r.score)).collect::<Vec<_>>());
    println!("  joueurs 24 h {:?} · record {:?} · suivi par {:?}", g.players_day, g.players_peak, g.waitlisted);
    println!("  plus bas : 3 mois {:?} · 1 an {:?} · tous temps {:?} {}", g.low_3m, g.low_1y, g.cheapest_ever, g.currency);
    for p in &g.prices {
        println!("  {:<22} {:>7.2} {} -{}% drm {:?} fin {:?} code {:?}", p.store_name, p.price, p.currency, p.savings, p.drm, p.expiry, p.voucher);
    }
    let Some(s) = &g.steam else {
        println!("  (pas de fiche Steam)");
        return;
    };
    println!("  Steam : avis {} / {:?} % ({}) · tags {:?}", s.review_score, s.review_percent, s.review_count, s.tags);
    println!("  captures {} · aperçu {:?}", s.captures.len(), s.apercu.as_ref().map(|a| &a.nom));
    println!("  langues {} : {:?}", s.langues.len(), s.langues.iter().take(4).map(|l| format!("{}{}{}", l.code, if l.voix { "+voix" } else { "" }, if l.sous_titres { "+st" } else { "" })).collect::<Vec<_>>());
    println!("  classification {:?}", s.classification);
    println!("  catégories {:?}", s.categories.iter().map(|c| format!("{}:{}", c.id, c.libelle)).collect::<Vec<_>>());
    println!("  succès {:?} · dlc {:?} · site {:?}", s.succes, s.dlc, s.site);
    if let Some(c) = &s.configuration {
        println!("  config min {:?}", c.minimale.iter().map(|l| format!("{} = {}", l.libelle, l.valeur)).collect::<Vec<_>>());
        println!("  notes {:?}", c.notes);
    }
    println!("  description : {} blocs, {} images", s.description.len(), s.description.iter().filter(|b| b.genre == "img").count());
    for b in s.description.iter().take(5) {
        println!("    [{}] {}", b.genre, b.texte.chars().take(90).collect::<String>());
    }

    let t = std::time::Instant::now();
    let x = vitrine::extras(g.app_id.unwrap_or(0), &s.dlc, &s.tag_ids, &dir, &Default::default());
    println!("\nBas de fiche en {:.1} s (tags {:?})", t.elapsed().as_secs_f32(), x.similar_tags);
    for j in &x.dlc {
        println!("  DLC {:<44} {:?} -{}% {} gratuit={}", j.title, j.price, j.savings, j.store_name, j.is_free);
    }
    for j in &x.similar {
        println!("  ~ {:<44} {:?} -{}% {}", j.title, j.price, j.savings, j.store_name);
    }
}
