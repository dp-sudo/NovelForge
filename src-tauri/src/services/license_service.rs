use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::errors::AppErrorDto;
use crate::infra::fs_utils::{read_text_if_exists, write_file_atomic};
use crate::infra::time::now_iso;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    pub activated: bool,
    pub tier: String,
    pub license_key_masked: Option<String>,
    pub activated_at: Option<String>,
    pub expires_at: Option<String>,
    pub offline_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LicenseStore {
    license_key_hash: String,
    license_key_masked: String,
    tier: String,
    activated_at: String,
    expires_at: Option<String>,
}

#[derive(Default)]
pub struct LicenseService;

impl LicenseService {
    pub fn get_status(&self) -> Result<LicenseStatus, AppErrorDto> {
        let Some(store) = self.load_store()? else {
            return Ok(LicenseStatus {
                activated: false,
                tier: "free".to_string(),
                license_key_masked: None,
                activated_at: None,
                expires_at: None,
                offline_available: true,
            });
        };

        Ok(LicenseStatus {
            activated: true,
            tier: store.tier,
            license_key_masked: Some(store.license_key_masked),
            activated_at: Some(store.activated_at),
            expires_at: store.expires_at,
            offline_available: true,
        })
    }

    pub fn activate(&self, license_key: &str) -> Result<LicenseStatus, AppErrorDto> {
        let normalized = normalize_license_key(license_key);
        validate_license_key(&normalized)?;

        let store = LicenseStore {
            license_key_hash: sha256_hex(&normalized),
            license_key_masked: mask_license_key(&normalized),
            tier: infer_tier(&normalized),
            activated_at: now_iso(),
            expires_at: None,
        };
        self.save_store(&store)?;
        self.get_status()
    }

    fn load_store(&self) -> Result<Option<LicenseStore>, AppErrorDto> {
        let path = license_file_path()?;
        let raw = read_text_if_exists(&path).map_err(|err| {
            AppErrorDto::new(
                "LICENSE_READ_FAILED",
                "Cannot read local license cache",
                true,
            )
            .with_detail(err.to_string())
        })?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let parsed = serde_json::from_str::<LicenseStore>(&raw).map_err(|err| {
            AppErrorDto::new(
                "LICENSE_READ_FAILED",
                "Local license cache is corrupted",
                true,
            )
            .with_detail(err.to_string())
            .with_suggested_action("Please re-activate your license")
        })?;
        Ok(Some(parsed))
    }

    fn save_store(&self, store: &LicenseStore) -> Result<(), AppErrorDto> {
        let path = license_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                AppErrorDto::new(
                    "LICENSE_WRITE_FAILED",
                    "Cannot create license cache directory",
                    true,
                )
                .with_detail(err.to_string())
            })?;
        }

        let payload = serde_json::to_string(store).map_err(|err| {
            AppErrorDto::new(
                "LICENSE_WRITE_FAILED",
                "Cannot serialize license cache",
                true,
            )
            .with_detail(err.to_string())
        })?;
        write_file_atomic(&path, &payload).map_err(|err| {
            AppErrorDto::new(
                "LICENSE_WRITE_FAILED",
                "Cannot write local license cache",
                true,
            )
            .with_detail(err.to_string())
        })
    }
}

fn license_file_path() -> Result<PathBuf, AppErrorDto> {
    if let Ok(path) = std::env::var("NOVELFORGE_LICENSE_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    let home = dirs::home_dir().ok_or_else(|| {
        AppErrorDto::new(
            "LICENSE_PATH_UNAVAILABLE",
            "Cannot resolve user home directory",
            false,
        )
    })?;
    Ok(home.join(".novelforge").join("license.json"))
}

fn normalize_license_key(input: &str) -> String {
    input
        .trim()
        .to_ascii_uppercase()
        .replace(' ', "")
        .replace('_', "-")
}

fn validate_license_key(key: &str) -> Result<(), AppErrorDto> {
    let parts = key.split('-').collect::<Vec<_>>();
    if parts.len() != 5 || parts[0] != "NF" {
        return Err(
            AppErrorDto::new("LICENSE_INVALID", "Invalid license key format", true)
                .with_suggested_action("Expected format: NF-XXXX-XXXX-XXXX-XXXX"),
        );
    }
    let body_is_valid = parts[1..]
        .iter()
        .all(|part| part.len() == 4 && part.chars().all(|ch| ch.is_ascii_alphanumeric()));
    if !body_is_valid {
        return Err(
            AppErrorDto::new("LICENSE_INVALID", "Invalid license key format", true)
                .with_suggested_action("Expected format: NF-XXXX-XXXX-XXXX-XXXX"),
        );
    }
    Ok(())
}

fn mask_license_key(key: &str) -> String {
    let chars = key.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let head = chars.iter().take(5).collect::<String>();
    let tail = chars
        .iter()
        .rev()
        .take(4)
        .rev()
        .copied()
        .collect::<String>();
    format!("{head}***-****-{tail}")
}

fn infer_tier(key: &str) -> String {
    if key.contains("PRO") {
        "pro".to_string()
    } else {
        "beta".to_string()
    }
}

fn sha256_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use super::LicenseService;

    #[test]
    fn activate_and_load_license_status_succeeds() {
        let path = std::env::temp_dir()
            .join(format!("novelforge-license-tests-{}", Uuid::new_v4()))
            .join("license.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create temp dir");
        }
        std::env::set_var(
            "NOVELFORGE_LICENSE_FILE",
            path.to_string_lossy().to_string(),
        );

        let service = LicenseService;
        let activated = service
            .activate("nf-ab12-cd34-ef56-gh78")
            .expect("activate license");
        assert!(activated.activated);
        assert_eq!(activated.tier, "beta");
        assert!(activated.license_key_masked.is_some());

        let loaded = service.get_status().expect("load status");
        assert!(loaded.activated);
        assert!(loaded.offline_available);

        std::env::remove_var("NOVELFORGE_LICENSE_FILE");
        let root = PathBuf::from(path.parent().expect("parent"));
        let _ = fs::remove_dir_all(root);
    }
}
