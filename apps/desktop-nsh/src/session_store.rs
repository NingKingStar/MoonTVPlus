//! A restart should not force the password again, but the token must not sit
//! in a plain-text JSON file either. The saved blob is sealed with Windows
//! DPAPI under the current user and kept in the app's own storage.
//!
//! Nothing here protects against malware already running as this user; it
//! only keeps the token out of reach of casual inspection, backups and other
//! accounts. "断开连接" clears it.

use nsh_core::storage::Storage;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData,
};
use windows::core::PCWSTR;

const SESSION_KEY: &str = "moontv.nsh.session.v1";
/// Refuse to show a DPAPI prompt: the app is not allowed to block on UI.
const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Saved {
    pub base: String,
    pub username: String,
    pub token: String,
}

fn protect(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptProtectData(
            &input,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .ok()?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        Some(bytes)
    }
}

fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    if data.is_empty() {
        return None;
    }
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .ok()?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        Some(bytes)
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if text.is_empty() || !text.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

pub fn save(base: &str, username: &str, token: &str) {
    if base.is_empty() || token.is_empty() {
        return;
    }
    let payload = Saved {
        base: base.to_owned(),
        username: username.to_owned(),
        token: token.to_owned(),
    };
    let Ok(json) = serde_json::to_vec(&payload) else {
        return;
    };
    let Some(sealed) = protect(&json) else {
        log::warn!("session could not be sealed");
        return;
    };
    if let Err(error) = Storage::set_item(SESSION_KEY, to_hex(&sealed)) {
        log::warn!("session storage refused: {error}");
    }
}

pub fn load() -> Option<Saved> {
    let raw = Storage::get_item(SESSION_KEY)?;
    let sealed = from_hex(&raw)?;
    let json = unprotect(&sealed)?;
    serde_json::from_slice(&json).ok()
}

pub fn clear() {
    if let Err(error) = Storage::set_item(SESSION_KEY, String::new()) {
        log::warn!("session storage refused on clear: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{from_hex, protect, to_hex, unprotect};

    #[test]
    fn hex_round_trips() {
        let bytes = [0u8, 1, 15, 16, 255];
        assert_eq!(from_hex(&to_hex(&bytes)).unwrap(), bytes.to_vec());
        assert!(from_hex("xyz").is_none());
        assert!(from_hex("abc").is_none());
    }

    #[test]
    fn dpapi_round_trips_for_this_user() {
        let plain = b"moontv session payload";
        let sealed = protect(plain).expect("protect");
        assert_ne!(sealed, plain.to_vec());
        assert_eq!(unprotect(&sealed).expect("unprotect"), plain.to_vec());
    }
}
