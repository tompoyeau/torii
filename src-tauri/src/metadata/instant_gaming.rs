//! Prix **Instant Gaming** (revendeur de clés, prix natifs en EUR).
//!
//! Instant Gaming n'a **pas d'API publique** et n'est **pas** couvert par CheapShark.
//! Sa page de recherche rend la liste côté client, MAIS l'endpoint `?ajax=true`
//! renvoie un **fragment HTML server-rendu** avec les cartes produits (titre, prix €,
//! remise, lien). On le parse en best-effort (⚠️ fragile : casse si IG change son HTML
//! → dans ce cas on renvoie simplement `None`, la fiche produit reste fonctionnelle).
//!
//! Usage volontairement **à la demande, un seul appel par fiche produit** (jamais en
//! masse sur la vitrine). Match par titre normalisé exact + édition PC (évite les
//! DLC/éditions/consoles que la recherche remonte).

use serde::Serialize;

const BROWSER_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// Une offre Instant Gaming résolue pour un jeu.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IgOffer {
    /// Prix en EUR (natif, pas de conversion).
    pub price: f64,
    /// Remise en % entier (0 si aucune).
    pub savings: u32,
    /// Lien d'achat (page produit Instant Gaming).
    pub url: String,
    /// `false` si le jeu est en rupture de stock sur IG (page produit sans « add to cart »).
    pub available: bool,
}

/// Clé de rapprochement d'un titre (minuscules alphanumériques).
fn key(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Extrait le premier attribut `attr="..."` après la position `from` dans `block`.
fn attr<'a>(block: &'a str, attr: &str) -> Option<&'a str> {
    let pat = format!("{attr}=\"");
    let i = block.find(&pat)? + pat.len();
    let rest = &block[i..];
    let j = rest.find('"')?;
    Some(&rest[..j])
}

/// Décode le minimum d'entités HTML rencontrées dans les prix/titres IG.
fn decode(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}

/// Parse un prix EUR depuis un texte type `58.99&nbsp;€` ou `58,99 €`.
fn parse_price(raw: &str) -> Option<f64> {
    let cleaned: String = decode(raw)
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let cleaned = cleaned.replace(',', ".");
    cleaned.parse().ok()
}

/// Récupère la meilleure offre Instant Gaming pour un jeu (édition PC, titre exact).
/// `None` si rien de fiable (jamais d'erreur remontée à l'appelant).
pub fn price(title: &str) -> Option<IgOffer> {
    let want = key(title);
    if want.len() < 3 {
        return None;
    }
    let url = format!(
        "https://www.instant-gaming.com/en/search/?q={}&ajax=true",
        urlencode(title)
    );
    let body = ureq::get(&url)
        .set("User-Agent", BROWSER_UA)
        .set("X-Requested-With", "XMLHttpRequest")
        .timeout(std::time::Duration::from_secs(12))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    let mut offer = parse(&body, &want)?;
    // Le fragment de recherche n'indique jamais le stock : on va lire la page produit.
    // Rupture = page sans bouton « add to cart » (IG affiche alors `nostock` / « Out of stock »).
    offer.available = is_available(&offer.url);
    Some(offer)
}

/// Vérifie la disponibilité d'un jeu sur sa page produit IG. En rupture, IG retire le
/// bouton d'ajout au panier (`addtocart`). En cas d'échec réseau on suppose disponible
/// (on ne masque pas une offre par excès de prudence).
fn is_available(product_url: &str) -> bool {
    match ureq::get(product_url)
        .set("User-Agent", BROWSER_UA)
        .timeout(std::time::Duration::from_secs(12))
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
    {
        Some(body) => body.contains("addtocart"),
        None => true,
    }
}

/// Extrait la meilleure offre PC d'un fragment HTML de recherche IG, pour un titre
/// normalisé `want`. Séparé de `price` pour être testable sans réseau.
fn parse(body: &str, want: &str) -> Option<IgOffer> {
    // Chaque produit = un bloc <article class="item ..."> … </article>.
    let mut best: Option<IgOffer> = None;
    for block in body.split("<article").skip(1) {
        // Titre : <span class="title" title="...">.
        let title_attr = block
            .split("class=\"title\"")
            .nth(1)
            .and_then(|s| attr(s, "title"))
            .map(decode);
        let Some(prod_title) = title_attr else { continue };

        // Lien produit (href du <a class="cover">).
        let Some(href) = attr(block, "href").filter(|h| h.contains("instant-gaming.com")) else {
            continue;
        };

        // Les titres IG sont suffixés « <Nom> - <Plateforme> (<Boutique>) »
        // (ex. « Elden Ring - PC (Steam) », « … - Switch 2 », « … - PS4 & PS5 »).
        // On isole le nom (avant le dernier « - ») et on ne garde que les éditions PC.
        let (name, platform) = match prod_title.rfind(" - ") {
            Some(i) => (prod_title[..i].trim(), prod_title[i + 3..].to_lowercase()),
            None => (prod_title.as_str(), String::new()),
        };
        if !platform.is_empty() && !platform.contains("pc") {
            continue; // édition console → ignorée
        }

        // Match titre exact (normalisé, hors suffixe plateforme) → écarte les DLC et
        // éditions (« Elden Ring Shadow of the Erdtree », « … Nightreign », « … Deluxe »).
        if key(name) != want {
            continue;
        }

        // Prix : <div class="price">…€</div>.
        let price_txt = block
            .split("class=\"price\"")
            .nth(1)
            .and_then(|s| s.split('>').nth(1))
            .and_then(|s| s.split('<').next());
        let Some(price) = price_txt.and_then(parse_price) else { continue };

        let savings = block
            .split("class=\"discount\"")
            .nth(1)
            .and_then(|s| s.split('>').nth(1))
            .and_then(|s| s.split('<').next())
            .map(|s| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        // On garde la moins chère parmi les correspondances exactes.
        let offer = IgOffer {
            price,
            savings,
            url: href.to_string(),
            available: true, // rempli par `price()` après lecture de la page produit
        };
        best = match best {
            Some(b) if b.price <= offer.price => Some(b),
            _ => Some(offer),
        };
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fragment calqué sur la vraie structure IG (recherche « elden ring ») : un DLC PC,
    // une version console, et le jeu de base PC (celui qu'on doit sélectionner).
    const FRAG: &str = r#"
    <article class="item">
      <a class="cover" href="https://www.instant-gaming.com/en/16007-buy-elden-ring-shadow-of-the-erdtree-edition-pc-steam/" title="buy ...">
      <div class="name"><span class="title" title="Elden Ring Shadow of the Erdtree Edition - PC (Steam)">x</span></div>
      <div class="discount">-21%</div><div class="price">31.49&nbsp;€</div>
    </article>
    <article class="item">
      <a class="cover" href="https://www.instant-gaming.com/en/19051-buy-elden-ring-tarnished-edition-switch-2/" title="buy ...">
      <div class="name"><span class="title" title="Elden Ring Tarnished Edition - Switch 2">x</span></div>
      <div class="discount">-26%</div><div class="price">58.99&nbsp;€</div>
    </article>
    <article class="item">
      <a class="cover" href="https://www.instant-gaming.com/en/4824-buy-elden-ring-pc-steam/" title="buy ...">
      <div class="name"><span class="title" title="Elden Ring - PC (Steam)">x</span></div>
      <div class="discount">-14%</div><div class="price">51.39&nbsp;€</div>
    </article>"#;

    #[test]
    fn picks_base_pc_edition() {
        let o = parse(FRAG, "eldenring").expect("doit trouver le jeu de base");
        assert_eq!(o.price, 51.39); // pas le DLC (31.49) ni la Switch (58.99)
        assert_eq!(o.savings, 14);
        assert!(o.url.ends_with("4824-buy-elden-ring-pc-steam/"));
    }

    #[test]
    fn no_match_returns_none() {
        assert!(parse(FRAG, "haloinfinite").is_none());
    }

    #[test]
    fn parses_price_formats() {
        assert_eq!(parse_price("51.39&nbsp;€"), Some(51.39));
        assert_eq!(parse_price("9,59 €"), Some(9.59));
    }
}
