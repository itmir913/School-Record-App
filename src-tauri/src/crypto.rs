use aes_gcm::{
    aead::{Aead, Generate, KeyInit, Nonce},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use pbkdf2::pbkdf2_hmac;
use rand::Rng;
use sha2::Sha256;

const AES_256_KEY_LEN: usize = 32;
const AES_GCM_NONCE_LEN: usize = 12;
const PBKDF2_ITERATIONS: u32 = 600_000;

fn cipher_from_key(key: &[u8]) -> Result<Aes256Gcm, String> {
    if key.len() != AES_256_KEY_LEN {
        return Err(format!("잘못된 암호화 키 길이입니다: {} bytes", key.len()));
    }
    let key = <&Key<Aes256Gcm>>::try_from(key)
        .map_err(|e| format!("암호화 키 변환 실패: {e}"))?;
    Ok(Aes256Gcm::new(key))
}

pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::rng().fill_bytes(&mut salt);
    salt
}

pub fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

pub fn encrypt(plaintext: &str, key: &[u8]) -> Result<String, String> {
    let cipher = cipher_from_key(key)?;
    let nonce = Nonce::<Aes256Gcm>::try_generate()
        .map_err(|e| format!("nonce 생성 실패: {e}"))?;
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("암호화 실패: {e}"))?;
    let nonce_b64 = B64.encode(nonce.as_slice());
    let cipher_b64 = B64.encode(&ciphertext);
    Ok(format!("{nonce_b64}:{cipher_b64}"))
}

pub fn decrypt(value: &str, key: &[u8]) -> Result<String, String> {
    let (nonce_b64, cipher_b64) = value
        .split_once(':')
        .ok_or_else(|| "잘못된 암호화 형식입니다.".to_string())?;
    let nonce_bytes = B64
        .decode(nonce_b64)
        .map_err(|e| format!("nonce 디코딩 실패: {e}"))?;
    if nonce_bytes.len() != AES_GCM_NONCE_LEN {
        return Err(format!(
            "잘못된 nonce 길이입니다: {} bytes",
            nonce_bytes.len()
        ));
    }
    let ciphertext = B64
        .decode(cipher_b64)
        .map_err(|e| format!("암호문 디코딩 실패: {e}"))?;
    let cipher = cipher_from_key(key)?;
    let nonce = <&Nonce<Aes256Gcm>>::try_from(&nonce_bytes[..])
        .map_err(|e| format!("nonce 변환 실패: {e}"))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        // 인증 실패는 키가 틀린 경우와 저장된 값이 손상된 경우를 구분하지 못한다.
        // 잠금 해제는 검증 토큰으로 따로 판정하므로(commands/crypto.rs의 verify_password),
        // 여기까지 오는 것은 대개 이미 해제된 세션에서 셀 하나가 풀리지 않는 경우다.
        // "비밀번호가 틀렸다"고 단정하면 사용자가 멀쩡한 비밀번호를 의심하게 된다.
        .map_err(|_| "복호화 실패: 데이터가 손상되었거나 암호화 키가 맞지 않습니다.".to_string())?;
    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 변환 실패: {e}"))
}

/// 암호화 키가 있으면 복호화, 없거나 빈 문자열이면 그대로 반환
pub fn maybe_decrypt(s: String, key: Option<[u8; 32]>) -> Result<String, String> {
    if s.is_empty() {
        return Ok(s);
    }
    match key {
        Some(k) => decrypt(&s, &k),
        None => Ok(s),
    }
}

/// 암호화 키가 있으면 암호화, 없거나 빈 문자열이면 그대로 반환
pub fn maybe_encrypt(s: &str, key: Option<[u8; 32]>) -> Result<String, String> {
    if s.is_empty() {
        return Ok(s.to_string());
    }
    match key {
        Some(k) => encrypt(s, &k),
        None => Ok(s.to_string()),
    }
}
