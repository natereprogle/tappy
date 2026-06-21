use keyring::Entry;

const KEYRING_SERVICE: &str = "com.tappy.app";
const KEYRING_USER: &str = "admin_token";

/// Persist the admin token to the OS keychain.
pub fn save_admin_token(token: &str) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keychain access error: {e}"))?;
    entry
        .set_password(token)
        .map_err(|e| format!("Failed to save admin token to keychain: {e}"))
}

/// Retrieve the admin token from the OS keychain.
/// Returns `Ok(None)` if no token has been stored yet.
pub fn load_admin_token() -> Result<Option<String>, String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keychain access error: {e}"))?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to load admin token from keychain: {e}")),
    }
}

/// Remove the admin token from the OS keychain (e.g. on token rotation before re-storing).
#[allow(dead_code)]
pub fn delete_admin_token() -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keychain access error: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        // Deleting a non-existent entry is fine — treat as success
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete admin token from keychain: {e}")),
    }
}
