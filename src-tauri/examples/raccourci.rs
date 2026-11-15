//! Diagnostic de l'ajout manuel : `cargo run --example raccourci [-- <fichier>…]`.
//!
//! Montre ce que Torii déduit d'un fichier choisi dans « Ajouter un jeu » : titre, vraie
//! cible d'un raccourci, arguments, dossier du jeu. Sans argument, passe en revue les
//! raccourcis du Bureau et du menu Démarrer, au format `raccourci <TAB> cible <TAB>
//! arguments` — de quoi comparer avec ce qu'en dit Windows.
//!
//! Aucun appel réseau, aucune écriture.

use ludo_lib::platforms::{manual, raccourci};
use std::path::{Path, PathBuf};

fn main() {
    let fichiers: Vec<String> = std::env::args().skip(1).collect();
    if !fichiers.is_empty() {
        for f in &fichiers {
            match manual::inspecter(f) {
                Ok(vu) => println!("{f}\n  {vu:?}"),
                Err(e) => println!("{f}\n  refusé : {e}"),
            }
        }
        return;
    }

    let mut raccourcis = Vec::new();
    for racine in dossiers() {
        ramasser(&racine, &mut raccourcis);
    }
    raccourcis.sort();
    let mut illisibles = 0;
    for chemin in &raccourcis {
        match raccourci::lire(chemin) {
            Some(r) => println!("{}\t{}\t{}", chemin.display(), r.cible, r.arguments.unwrap_or_default()),
            None => {
                illisibles += 1;
                println!("{}\t\t", chemin.display());
            }
        }
    }
    eprintln!("{} raccourcis, {illisibles} sans cible locale", raccourcis.len());
}

/// Bureau et menu Démarrer, de l'utilisateur et communs.
fn dossiers() -> Vec<PathBuf> {
    let var = |nom: &str| std::env::var(nom).ok().map(PathBuf::from);
    [
        var("USERPROFILE").map(|p| p.join("Desktop")),
        var("PUBLIC").map(|p| p.join("Desktop")),
        var("APPDATA").map(|p| p.join(r"Microsoft\Windows\Start Menu\Programs")),
        var("ProgramData").map(|p| p.join(r"Microsoft\Windows\Start Menu\Programs")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn ramasser(dossier: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entrees) = std::fs::read_dir(dossier) else { return };
    for entree in entrees.flatten() {
        let chemin = entree.path();
        if chemin.is_dir() {
            ramasser(&chemin, out);
        } else if chemin.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")) {
            out.push(chemin);
        }
    }
}
