//! Test du client social contre un serveur local :
//!   1. `cd server && npx wrangler dev --port 8787 --local`
//!   2. `TORII_API=http://127.0.0.1:8787 cargo run --example social`
//! Utilise un dossier de config jetable : les vrais identifiants ne sont pas touchés.
use ludo_lib::social;

fn main() {
    let dir = std::env::temp_dir().join(format!("torii-social-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let email = format!("client{stamp}@example.com");
    // 🔑 Un SteamID par exécution : il est désormais unique parmi les comptes, donc une
    // valeur fixe ferait échouer le second passage sur la même base locale.
    let steam_id = format!("7656119{:010}", stamp % 10_000_000_000);

    println!("── Sans session : me() doit être None → {:?}", social::me(&dir).is_none());

    let code = social::request_code(&email).expect("demande de code").expect("mode dev attendu");
    println!("── Code reçu (mode dev) : {code}");

    println!("── Mauvais code : {:?}", social::verify(&dir, &email, "000000").unwrap_err());

    // Inscription différée : la validation du code ne crée RIEN, elle rend un
    // laissez-passer. C'est la garantie qu'on vend à l'interface — fermer la fenêtre
    // ici ne doit laisser aucun compte derrière soi.
    let signin = social::verify(&dir, &email, &code).expect("validation du code");
    println!(
        "── Inscription à terminer : {} | aucun compte encore : {} | laissez-passer : {}",
        signin.created,
        signin.account.is_none(),
        signin.signup_token.is_some()
    );
    println!(
        "── Aucune session tant que le pseudo manque : {:?}",
        social::me(&dir).is_none()
    );

    let jeton = signin.signup_token.expect("laissez-passer");
    println!(
        "── Pseudo vide refusé : {:?}",
        social::signup(&dir, &jeton, "").unwrap_err()
    );
    println!(
        "── Laissez-passer bidon refusé : {:?}",
        social::signup(&dir, "n.importe.quoi", "Testeur").unwrap_err()
    );

    let account = social::signup(&dir, &jeton, "Testeur").expect("création du compte");
    println!(
        "── Compte créé : {} | code d'ami {}",
        account.display_name, account.friend_code
    );

    // Un même laissez-passer rejoué ne doit pas fabriquer de doublon : il ouvre une
    // session sur le compte existant, sous son pseudo d'origine.
    let rejoue = social::signup(&dir, &jeton, "Usurpateur").expect("rejeu");
    println!("── Rejeu du laissez-passer : pseudo inchangé = {}", rejoue.display_name == "Testeur");

    // Le jeton doit être rangé chiffré, comme les autres secrets.
    let creds = ludo_lib::accounts::secrets::load(&dir);
    println!("── Jeton persisté : {}", creds.torii_token.is_some());
    let raw = std::fs::read(dir.join("credentials.dat")).unwrap();
    let token = creds.torii_token.clone().unwrap();
    let leaked = raw.windows(token.len()).any(|w| w == token.as_bytes());
    println!("── Jeton lisible en clair sur le disque : {leaked}");

    println!("── me() : {:?}", social::me(&dir).map(|a| a.email));

    let renamed = social::set_profile(&dir, Some("Testeur renommé".into()), Some(steam_id.clone()), Some(true)).unwrap();
    println!("── Profil : {} | steam {:?} | découvrable {}", renamed.display_name, renamed.steam_id, renamed.steam_discoverable);

    // Un SteamID ne se relie qu'à UN compte Torii. Deuxième compte, même Steam : refus.
    // Sans ça, la personne apparaît deux fois chez ses amis — dont une ligne inerte — et
    // n'importe qui peut porter l'avatar Steam d'un autre.
    {
        let dir2 = dir.with_extension("second");
        std::fs::create_dir_all(&dir2).unwrap();
        let email2 = format!("second{stamp}@example.com");
        let code2 = social::request_code(&email2).unwrap().unwrap();
        let jeton2 = social::verify(&dir2, &email2, &code2).unwrap().signup_token.unwrap();
        social::signup(&dir2, &jeton2, "Sosie").unwrap();
        let vol = social::set_profile(&dir2, None, Some(steam_id.clone()), Some(true));
        println!("── Même Steam sur un 2ᵉ compte : {:?}", vol.unwrap_err());
        // Et le premier compte garde son lien, intact.
        let moi = social::me(&dir).unwrap();
        println!("── Lien du 1ᵉʳ compte préservé : {}", moi.steam_id.as_deref() == Some(steam_id.as_str()));
        let _ = std::fs::remove_dir_all(&dir2);
    }

    let neuf = social::rotate_code(&dir).unwrap();
    println!("── Code d'ami régénéré : {} → {}", account.friend_code, neuf);

    println!("── Code d'ami inexistant : {:?}", social::invite(&dir, "ZZZZZZZZ").unwrap_err());

    let presence = social::Presence {
        status: "in-game".into(),
        game_key: Some("igdb:1942".into()),
        game_title: Some("The Witcher 3: Wild Hunt".into()),
        since: Some(stamp as i64 - 3600),
    };
    let circle = social::publish(&dir, &presence).unwrap();
    println!("── Présence publiée, cercle reçu : {} amis, {} demandes", circle.friends.len(), circle.incoming.len());

    social::clear_presence(&dir).unwrap();
    println!("── Présence effacée");

    social::logout(&dir).unwrap();
    // Suppression du compte : le jeton doit cesser de valoir quoi que ce soit, et la même
    // adresse doit pouvoir repartir à zéro comme si elle n'avait jamais existé.
    {
        let dir3 = dir.with_extension("jetable");
        std::fs::create_dir_all(&dir3).unwrap();
        let email3 = format!("jetable{stamp}@example.com");
        let code3 = social::request_code(&email3).unwrap().unwrap();
        let jeton3 = social::verify(&dir3, &email3, &code3).unwrap().signup_token.unwrap();
        social::signup(&dir3, &jeton3, "Éphémère").unwrap();
        println!("── Compte jetable créé : {}", social::me(&dir3).is_some());

        social::delete_account(&dir3).expect("suppression");
        println!("── Après suppression, plus de session : {}", social::me(&dir3).is_none());

        // La même adresse repart neuve : c'est bien le compte qui a disparu, pas seulement
        // la session.
        let code4 = social::request_code(&email3).unwrap().unwrap();
        let apres = social::verify(&dir3, &email3, &code4).unwrap();
        println!(
            "── Même adresse réutilisable, comme une inscription neuve : {}",
            apres.signup_token.is_some() && apres.account.is_none()
        );
        let _ = std::fs::remove_dir_all(&dir3);
    }

    println!("── Déconnecté, jeton effacé : {}", ludo_lib::accounts::secrets::load(&dir).torii_token.is_none());
    println!("── me() après déconnexion : {:?}", social::me(&dir).is_none());

    let _ = std::fs::remove_dir_all(&dir);
}
