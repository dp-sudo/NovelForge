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
            "无法访问凭据存储",
            true,
        )
        .with_detail(e.to_string())
    })?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(
            AppErrorDto::new("SECRET_DELETE_FAILED", "无法删除 API 密钥", true)
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
                "无法创建本地密钥目录",
                true,
            )
            .with_detail(err.to_string())
        })?;
    }
    crate::infra::fs_utils::write_file_atomic(&target, &encrypted).map_err(|err| {
        AppErrorDto::new("SECRET_SAVE_FAILED", "无法保存 API 密钥", true)
            .with_detail(err.to_string())
    })
}

fn load_fallback_secret(provider_id: &str) -> Result<Option<String>, AppErrorDto> {
    let path = fallback_secret_file_path(provider_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let payload = fs::read_to_string(&path).map_err(|err| {
        AppErrorDto::new("SECRET_LOAD_FAILED", "无法读取 API 密钥", true)
            .with_detail(err.to_string())
    })?;
    crypto::decrypt(payload.trim()).map(Some).map_err(|err| {
        AppErrorDto::new("SECRET_LOAD_FAILED", "无法读取 API 密钥", true).with_detail(err.message)
    })
}

fn delete_fallback_secret(provider_id: &str) -> Result<(), AppErrorDto> {
    let path = fallback_secret_file_path(provider_id)?;
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path).map_err(|err| {
        AppErrorDto::new("SECRET_DELETE_FAILED", "无法删除 API 密钥", true)
            .with_detail(err.to_string())
    })
}

fn fallback_secret_file_path(provider_id: &str) -> Result<PathBuf, AppErrorDto> {
    let provider_component = sanitize_provider_id(provider_id);
    if provider_component.is_empty() {
        return Err(AppErrorDto::new(
            "SECRET_PROVIDER_INVALID",
            "供应商ID不能为空",
            true,
        ));
    }
    let base = app_data_dir()?;
    Ok(base
        .join("secrets")
        .join(format!("{provider_component}.secret")))
}

fn sanitize_provider_id(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            sanitized.push(ch);
            continue;
        }
        let mut buffer = [0_u8; 4];
        for byte in ch.encode_utf8(&mut buffer).as_bytes() {
            sanitized.push('_');
            sanitized.push_str(&format!("{byte:02X}"));
        }
    }
    sanitized
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
            "provider_2Fwith_2Achars"
        );
    }

    #[test]
    fn fallback_path_stays_inside_app_data_dir() {
        let path = fallback_secret_file_path("../provider").expect("path");
        let parent = path.parent().expect("parent");
        assert!(parent.ends_with(Path::new("secrets")));
        assert!(path.ends_with(Path::new(".._2Fprovider.secret")));
    }

    #[test]
    fn fallback_paths_do_not_collide_for_alias_like_provider_ids() {
        let slash_path = fallback_secret_file_path("foo/bar").expect("slash path");
        let underscore_path = fallback_secret_file_path("foo_bar").expect("underscore path");
        let spaced_path = fallback_secret_file_path("foo bar").expect("spaced path");

        assert_ne!(slash_path, underscore_path);
        assert_ne!(slash_path, spaced_path);
        assert_ne!(underscore_path, spaced_path);
    }
}

