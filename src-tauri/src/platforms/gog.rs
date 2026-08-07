use crate::models::GameDto;

/// Scanne les jeux GOG installés, listés dans le registre.
pub fn scan() -> Vec<GameDto> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_LOCAL_MACHINE;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let roots = [r"SOFTWARE\WOW6432Node\GOG.com\Games", r"SOFTWARE\GOG.com\Games"];

        let mut games = Vec::new();
        for root in roots {
            let Ok(games_key) = hklm.open_subkey(root) else {
                continue;
            };
            for sub in games_key.enum_keys().flatten() {
                let Ok(key) = games_key.open_subkey(&sub) else {
                    continue;
                };
                let title: String = key.get_value("gameName").unwrap_or_default();
                let exe: String = key.get_value("exe").unwrap_or_default();
                let path: String = key.get_value("path").unwrap_or_default();
                let game_id: String = key.get_value("gameID").unwrap_or_else(|_| sub.clone());
                if title.is_empty() || exe.is_empty() {
                    continue;
                }

                let size_gb = if path.is_empty() {
                    0.0
                } else {
                    GameDto::bytes_to_gb(super::dir_size(std::path::Path::new(&path)))
                };

                games.push(GameDto {
                    id: format!("gog:{game_id}"),
                    title,
                    platform: "gog".into(),
                    installed: true,
                    size_gb,
                    install_dir: (!path.is_empty()).then_some(path),
                    launch_target: exe,
                    ..Default::default()
                });
            }
        }
        return games;
    }

    #[cfg(not(windows))]
    Vec::new()
}
