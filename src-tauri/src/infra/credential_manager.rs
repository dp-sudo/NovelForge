//! API key storage backed only by Windows Credential Manager (keyring backend).

use crate::errors::AppErrorDto;

const KEYRING_SERVICE: &str = "novelforge";

pub fn save_api_key(provider_id: &str, api_key: &str) -> Result<(), AppErrorDto> {
    save_to_keyring(provider_id, api_key).map_err(|err| {
        AppErrorDto::new(
            "SECRET_SERVICE_ERROR",
            "无法保存 API 密钥，请检查 Windows Credential Manager 是否可用",
            true,
        )
        .with_detail(err.to_string())
    })
}

pub fn load_api_key(provider_id: &str) -> Result<Option<String>, AppErrorDto> {
    load_from_keyring(provider_id).map_err(|err| {
        AppErrorDto::new(
            "SECRET_SERVICE_ERROR",
            "无法读取 API 密钥，请检查 Windows Credential Manager 是否可用",
            true,
        )
        .with_detail(err.to_string())
    })
}

pub fn delete_api_key(provider_id: &str) -> Result<(), AppErrorDto> {
    delete_from_keyring(provider_id).map_err(|err| {
        AppErrorDto::new(
            "SECRET_SERVICE_ERROR",
            "无法删除 API 密钥，请检查 Windows Credential Manager 是否可用",
            true,
        )
        .with_detail(err.to_string())
    })
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

fn delete_from_keyring(provider_id: &str) -> Result<(), keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_id)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err),
    }
}
