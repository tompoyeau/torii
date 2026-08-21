//! Identifiants des comptes de launchers, stockés dans le dossier de config de l'app.
//!
//! 🔒 **Chiffrés au repos** via DPAPI (`CryptProtectData`, portée utilisateur Windows) :
//! le fichier contient des refresh tokens de très longue durée — Steam ~200 j, Epic 365 j,
//! GOG jusqu'à révocation — plus les cookies de session. En clair, n'importe quel process
//! lisant `%APPDATA%` (un infostealer, typiquement) repartait avec un accès complet aux
//! comptes. DPAPI lie le chiffrement au compte Windows courant : pas de clé à gérer, et le
//! blob est inutilisable depuis un autre compte ou une autre machine.
//!
//! Migration automatique : au premier chargement, un ancien `credentials.json` en clair est
//! relu, réécrit chiffré dans `credentials.dat`, puis **supprimé**.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Identifiants stockés localement (chiffrés sur disque, cf. en-tête du module).
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    /// Cookie de session Steam côté store : "steamLoginSecure=…; sessionid=…".
    pub steam_login_secure: Option<String>,
    /// Cookie de session côté communauté (pour la page des jeux XML).
    pub steam_community: Option<String>,
    /// Clé API Steam (chemin avancé/optionnel, non exposé par défaut).
    pub steam_api_key: Option<String>,
    pub steam_id: Option<String>,
    /// Refresh token Steam (~200 j) capté au login. Permet de régénérer un cookie
    /// de session frais sans reconnexion (le cookie web expire en ~24 h).
    pub steam_refresh_token: Option<String>,
    /// Jeton de rafraîchissement GOG (les access tokens expirent en ~1 h ; on
    /// stocke le refresh token et on redérive l'access token à chaque sync).
    pub gog_refresh_token: Option<String>,
    /// Jeton de rafraîchissement Epic (même principe : access token ~8 h).
    pub epic_refresh_token: Option<String>,
    /// Session du service social Torii (comptes, amis, présence). Secret de longue
    /// durée : il a sa place ici, chiffré, et pas dans le stockage de la WebView.
    pub torii_token: Option<String>,
}

/// Fichier chiffré (format courant).
fn file(config_dir: &Path) -> PathBuf {
    config_dir.join("credentials.dat")
}

/// Ancien fichier en clair, migré puis supprimé au premier chargement.
fn legacy_file(config_dir: &Path) -> PathBuf {
    config_dir.join("credentials.json")
}

pub fn load(config_dir: &Path) -> Credentials {
    // 1. Cas normal : blob chiffré.
    if let Ok(blob) = std::fs::read(file(config_dir)) {
        if let Some(json) = unprotect(&blob) {
            if let Ok(creds) = serde_json::from_slice(&json) {
                return creds;
            }
        }
        // Blob illisible (copié depuis un autre compte Windows, fichier tronqué…) :
        // on repart d'identifiants vides plutôt que de planter — l'utilisateur se
        // reconnecte. On ne supprime rien, au cas où le contexte redeviendrait valide.
        return Credentials::default();
    }

    // 2. Migration depuis l'ancien fichier en clair.
    let Ok(text) = std::fs::read_to_string(legacy_file(config_dir)) else {
        return Credentials::default();
    };
    let creds: Credentials = serde_json::from_str(&text).unwrap_or_default();
    // `save` réécrit chiffré ET supprime le fichier en clair.
    let _ = save(config_dir, &creds);
    creds
}

pub fn save(config_dir: &Path, creds: &Credentials) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec(creds).map_err(|e| e.to_string())?;
    let blob = protect(&json).ok_or("Chiffrement des identifiants impossible (DPAPI).")?;

    // Écriture atomique : une coupure en plein écrasement laisserait sinon un fichier
    // tronqué, donc une déconnexion de TOUS les comptes au prochain démarrage.
    let path = file(config_dir);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &blob).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;

    // Le fichier en clair n'a plus lieu d'être (migration terminée).
    let _ = std::fs::remove_file(legacy_file(config_dir));
    Ok(())
}

// --- DPAPI (Windows) -----------------------------------------------------------

/// Entropie propre à Torii : le blob ne peut pas être déchiffré par un appel DPAPI
/// générique, il faut connaître cette valeur en plus du contexte utilisateur.
#[cfg(windows)]
const ENTROPY: &[u8] = b"torii-credentials-v1";

#[cfg(windows)]
#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[cfg(windows)]
#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        data_in: *const DataBlob,
        data_descr: *const u16,
        optional_entropy: *const DataBlob,
        reserved: *mut core::ffi::c_void,
        prompt_struct: *mut core::ffi::c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;
    fn CryptUnprotectData(
        data_in: *const DataBlob,
        data_descr: *mut *mut u16,
        optional_entropy: *const DataBlob,
        reserved: *mut core::ffi::c_void,
        prompt_struct: *mut core::ffi::c_void,
        flags: u32,
        data_out: *mut DataBlob,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

/// `CRYPTPROTECT_UI_FORBIDDEN` : jamais d'invite à l'écran (on est dans un service de fond).
#[cfg(windows)]
const UI_FORBIDDEN: u32 = 0x1;

#[cfg(windows)]
fn blob_of(bytes: &[u8]) -> DataBlob {
    DataBlob {
        cb_data: bytes.len() as u32,
        pb_data: bytes.as_ptr() as *mut u8,
    }
}

/// Copie le contenu d'un blob renvoyé par DPAPI puis libère la mémoire Windows.
#[cfg(windows)]
fn take(out: DataBlob) -> Vec<u8> {
    let slice = unsafe { std::slice::from_raw_parts(out.pb_data, out.cb_data as usize) };
    let owned = slice.to_vec();
    unsafe { LocalFree(out.pb_data as *mut core::ffi::c_void) };
    owned
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Option<Vec<u8>> {
    let input = blob_of(data);
    let entropy = blob_of(ENTROPY);
    let mut out = DataBlob { cb_data: 0, pb_data: std::ptr::null_mut() };
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            &entropy,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            UI_FORBIDDEN,
            &mut out,
        )
    };
    (ok != 0).then(|| take(out))
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    let input = blob_of(data);
    let entropy = blob_of(ENTROPY);
    let mut out = DataBlob { cb_data: 0, pb_data: std::ptr::null_mut() };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            &entropy,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            UI_FORBIDDEN,
            &mut out,
        )
    };
    (ok != 0).then(|| take(out))
}

// Hors Windows (compilation croisée / CI) : pas de DPAPI, on garde le JSON tel quel.
#[cfg(not(windows))]
fn protect(data: &[u8]) -> Option<Vec<u8>> {
    Some(data.to_vec())
}
#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    Some(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_is_encrypted_on_disk() {
        let dir = std::env::temp_dir().join(format!("torii-secrets-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let creds = Credentials {
            steam_refresh_token: Some("SECRET-REFRESH-TOKEN".into()),
            steam_id: Some("76561198000000000".into()),
            ..Default::default()
        };
        save(&dir, &creds).unwrap();

        let back = load(&dir);
        assert_eq!(back.steam_refresh_token.as_deref(), Some("SECRET-REFRESH-TOKEN"));
        assert_eq!(back.steam_id.as_deref(), Some("76561198000000000"));

        // Le jeton ne doit apparaître nulle part en clair dans le fichier.
        #[cfg(windows)]
        {
            let raw = std::fs::read(file(&dir)).unwrap();
            let needle = b"SECRET-REFRESH-TOKEN";
            assert!(
                !raw.windows(needle.len()).any(|w| w == needle),
                "le jeton est lisible en clair sur le disque !"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_and_deletes_plaintext_file() {
        let dir = std::env::temp_dir().join(format!("torii-migr-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Ancien format : JSON en clair.
        std::fs::write(
            legacy_file(&dir),
            r#"{"gogRefreshToken":"OLD-GOG","steamId":"76561198000000001"}"#,
        )
        .unwrap();

        let creds = load(&dir);
        assert_eq!(creds.gog_refresh_token.as_deref(), Some("OLD-GOG"));
        assert!(file(&dir).exists(), "le fichier chiffré doit avoir été créé");
        assert!(
            !legacy_file(&dir).exists(),
            "le fichier en clair doit avoir été supprimé après migration"
        );
        // Et il se relit bien depuis le nouveau format.
        assert_eq!(load(&dir).gog_refresh_token.as_deref(), Some("OLD-GOG"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
