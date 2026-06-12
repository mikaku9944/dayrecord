use dayrecord_core::ports::SecretStore;
use std::error::Error;

const SERVICE: &str = "com.dayrecord.desktop";

pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringSecretStore {
    fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let entry = keyring::Entry::new(SERVICE, key)?;
        entry.set_password(value)?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let entry = keyring::Entry::new(SERVICE, key)?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => {
                const LEGACY_SERVICE: &str = "com.dayrecord.app";
                if SERVICE == LEGACY_SERVICE {
                    return Ok(None);
                }
                let legacy = keyring::Entry::new(LEGACY_SERVICE, key)?;
                match legacy.get_password() {
                    Ok(v) => {
                        entry.set_password(&v)?;
                        let _ = legacy.delete_credential();
                        Ok(Some(v))
                    }
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(e) => Err(Box::new(e)),
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}
