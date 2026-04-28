//! API Key secure storage via Windows Credential Manager (primary)
//! with encrypted file fallback.
use crate::errors::AppErrorDto;

const KEYRING_SERVICE: &str = "novelforge";

/// Save an API key to Windows Credential Manager.
pub fn save_api_key(provider_id: &str, api_key: &str) -> Result<(), AppErrorDto> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_id)
        .map_err(|e| AppErrorDto::new("SECRET_SERVICE_ERROR", "Cannot access credential store", true).with_detail(e.to_string()))?;
    entry.set_password(api_key)
        .map_err(|e| AppErrorDto::new("SECRET_SAVE_FAILED", "Cannot save API key", true).with_detail(e.to_string()))
}

/// Load an API key from Windows Credential Manager.
pub fn load_api_key(provider_id: &str) -> Result<Option<String>, AppErrorDto> {
    let entry = match keyring::Entry::new(KEYRING_SERVICE, provider_id) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppErrorDto::new("SECRET_LOAD_FAILED", "Cannot load API key", true).with_detail(e.to_string())),
    }
}

/// Delete an API key from Windows Credential Manager.
pub fn delete_api_key(provider_id: &str) -> Result<(), AppErrorDto> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider_id)
        .map_err(|e| AppErrorDto::new("SECRET_SERVICE_ERROR", "Cannot access credential store", true).with_detail(e.to_string()))?;
    entry.delete_credential()
        .map_err(|e| AppErrorDto::new("SECRET_DELETE_FAILED", "Cannot delete API key", true).with_detail(e.to_string()))
}
