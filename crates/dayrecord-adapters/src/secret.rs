pub use crate::secret_keyring::KeyringSecretStore;

use dayrecord_core::ports::SecretStore;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Mutex;

/// In-memory secret store for tests and non-Windows dev.
pub struct MapSecretStore {
    inner: Mutex<HashMap<String, String>>,
}

impl MapSecretStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MapSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MapSecretStore {
    fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.inner.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        Ok(self.inner.lock().unwrap().get(key).cloned())
    }
}

#[cfg(windows)]
pub struct DpapiSecretStore;

#[cfg(windows)]
impl DpapiSecretStore {
    pub fn new() -> Self {
        Self
    }

    fn protect(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

        let mut plain = data.to_vec();
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: plain.len() as u32,
            pbData: plain.as_mut_ptr(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(&mut input, None, None, None, None, 0, &mut output)
                .map_err(|e| format!("CryptProtectData failed: {e}"))?;
            let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
            let out = slice.to_vec();
            windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(
                output.pbData as _,
            ));
            Ok(out)
        }
    }

    fn unprotect(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

        let mut cipher = data.to_vec();
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: cipher.len() as u32,
            pbData: cipher.as_mut_ptr(),
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(&mut input, None, None, None, None, 0, &mut output)
                .map_err(|e| format!("CryptUnprotectData failed: {e}"))?;
            let slice = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
            let out = slice.to_vec();
            windows::Win32::Foundation::LocalFree(windows::Win32::Foundation::HLOCAL(
                output.pbData as _,
            ));
            Ok(out)
        }
    }
}

#[cfg(windows)]
impl SecretStore for DpapiSecretStore {
    fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let protected = Self::protect(value.as_bytes())?;
        let encoded = base64_encode(&protected);
        let path = secret_path(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, encoded)?;
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let path = secret_path(key);
        if !path.exists() {
            return Ok(None);
        }
        let encoded = std::fs::read_to_string(path)?;
        let bytes = base64_decode(encoded.trim())?;
        let plain = Self::unprotect(&bytes)?;
        Ok(Some(String::from_utf8(plain)?))
    }
}

#[cfg(windows)]
fn secret_path(key: &str) -> std::path::PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    std::path::PathBuf::from(base)
        .join("DayRecord")
        .join(format!("{key}.secret"))
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(input: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input.bytes().filter(|b| *b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let v: Vec<u32> = chunk.iter().filter_map(|b| val(*b)).collect();
        if v.len() < 2 {
            break;
        }
        let n = (v[0] << 18) | (v[1] << 12) | (v.get(2).unwrap_or(&0) << 6) | v.get(3).unwrap_or(&0);
        out.push((n >> 16) as u8);
        if v.len() > 2 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if v.len() > 3 {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_store_roundtrip() {
        let store = MapSecretStore::new();
        store.set("deepseek_api_key", "sk-test").unwrap();
        assert_eq!(
            store.get("deepseek_api_key").unwrap(),
            Some("sk-test".into())
        );
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("LOCALAPPDATA", dir.path());
        let store = DpapiSecretStore::new();
        store.set("test_key", "secret-value").unwrap();
        assert_eq!(store.get("test_key").unwrap(), Some("secret-value".into()));
    }
}
