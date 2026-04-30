//! API key storage with Windows Credential Manager as primary backend,
//! plus encrypted local-file fallback for environments where keyring access fails.

use std::fs;
use std::path::PathBuf;

use crate::errors::AppErrorDto;
use crate::infra::app_paths::app_data_dir;
use crate::infra::crypto;

const KEYRING_SERVICE: &str = "novelforge";

pub fn save_api_key(provider_id: &str, api_key: &str) -> Result<(), AppErrorDto> {
    match save_to_keyring(provider_id, api_key) {
        Ok(()) => {
            let _ = delete_fallback_secret(provider_id);
            Ok(())
        }
        Err(keyring_err) => {
            log::warn!(
                "[SECRET] keyring save failed for provider={} fallback=file err={}",
                provider_id,
                keyring_err
            );
            save_fallback_secret(provider_id, api_key)
        }
    }
}

pub fn load_api_key(provider_id: &str) -> Result<Option<String>, AppErrorDto> {
    let keyring_result = load_from_keyring(provider_id);
    match keyring_result {
        Ok(Some(value)) => Ok(Some(value)),
        Ok(None) => load_fallback_secret(provider_id),
        Err(err) => {
            log::warn!(
                "[SECRET] keyring load failed for provider={} fallback=file err={}",
                provider_id,
                err
            );
            load_fallback_secret(provider_id)
        }
    }
}

pub fn delete_api_key(provider_id: &str) -> Result<(), AppErrorDto> {
    let mut keyring_error: Option<AppErrorDto> = None;
    if let Err(err) = delete_from_keyring(provider_id) {
        keyring_error = Some(err);
    }

    let fallback_error = delete_fallback_secret(provider_id).err();
    if let Some(err) = keyring_error {
        return Err(err);
    }
    if let Some(err) = fallback_error {
        return Err(err);
    }
    Ok(())
}

fn save_to_keyring(provider_id: &str, api_key: &str) -> Result<(), keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_id)?;
    entry.set_password(api_key)
}

fn load_from_keyring(provider_id: &str) -> Result<Option<String>, keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_id)?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

fn delete_from_keyring(provider_id: &str) -> Result<(), AppErrorDto> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_id).map_err(|e| {
        AppErrorDto::new(
            "SECRET_SERVICE_ERROR",
            "Cannot access credential store",
            true,
        )
        .with_detail(e.to_string())
    })?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(
            AppErrorDto::new("SECRET_DELETE_FAILED", "Cannot delete API key", true)
                .with_detail(e.to_string()),
        ),
    }
}

fn save_fallback_secret(provider_id: &str, api_key: &str) -> Result<(), AppErrorDto> {
    let encrypted = crypto::encrypt(api_key)?;
    let target = fallback_secret_file_path(provider_id)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            AppErrorDto::new(
                "SECRET_SAVE_FAILED",
                "Cannot create local secret directory",
                true,
            )
            .with_detail(err.to_string())
        })?;
    }
    crate::infra::fs_utils::write_file_atomic(&target, &encrypted).map_err(|err| {
        AppErrorDto::new("SECRET_SAVE_FAILED", "Cannot save API key", true)
            .with_detail(err.to_string())
    })
}

fn load_fallback_secret(provider_id: &str) -> Result<Option<String>, AppErrorDto> {
    let path = fallback_secret_file_path(provider_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let payload = fs::read_to_string(&path).map_err(|err| {
        AppErrorDto::new("SECRET_LOAD_FAILED", "Cannot load API key", true)
            .with_detail(err.to_string())
    })?;
    crypto::decrypt(payload.trim()).map(Some).map_err(|err| {
        AppErrorDto::new("SECRET_LOAD_FAILED", "Cannot load API key", true).with_detail(err.message)
    })
}

fn delete_fallback_secret(provider_id: &str) -> Result<(), AppErrorDto> {
    let path = fallback_secret_file_path(provider_id)?;
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path).map_err(|err| {
        AppErrorDto::new("SECRET_DELETE_FAILED", "Cannot delete API key", true)
            .with_detail(err.to_string())
    })
}

fn fallback_secret_file_path(provider_id: &str) -> Result<PathBuf, AppErrorDto> {
    let provider_component = sanitize_provider_id(provider_id);
    if provider_component.is_empty() {
        return Err(AppErrorDto::new(
            "SECRET_PROVIDER_INVALID",
            "Provider id cannot be empty",
            true,
        ));
    }
    let base = app_data_dir()?;
    Ok(base
        .join("secrets")
        .join(format!("{provider_component}.secret")))
}

fn sanitize_provider_id(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use uuid::Uuid;

    #[test]
    fn fallback_roundtrip_succeeds() {
        let provider_id = format!("test-provider-{}", Uuid::new_v4());

        save_fallback_secret(&provider_id, "sk-secret-value").expect("save");
        let loaded = load_fallback_secret(&provider_id)
            .expect("load")
            .expect("has value");
        assert_eq!(loaded, "sk-secret-value");

        delete_fallback_secret(&provider_id).expect("delete");
        let loaded_after_delete = load_fallback_secret(&provider_id).expect("load after delete");
        assert!(loaded_after_delete.is_none());
    }

    #[test]
    fn sanitize_provider_id_rejects_empty() {
        assert_eq!(sanitize_provider_id(""), "");
        assert_eq!(
            sanitize_provider_id(" provider/with*chars "),
            "provider_with_chars"
        );
    }

    #[test]
    fn fallback_path_stays_inside_app_data_dir() {
        let path = fallback_secret_file_path("../provider").expect("path");
        let parent = path.parent().expect("parent");
        assert!(parent.ends_with(Path::new("secrets")));
        assert!(path.ends_with(Path::new(".._provider.secret")));
    }
}
