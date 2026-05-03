use crate::crypto::{decrypt, derive_key, encrypt, generate_salt, maybe_decrypt, maybe_encrypt};

// ── derive_key ────────────────────────────────────────────────────

#[test]
fn test_derive_key_is_deterministic() {
    let salt = [1u8; 16];
    let key1 = derive_key("비밀번호", &salt);
    let key2 = derive_key("비밀번호", &salt);
    assert_eq!(key1, key2);
}

#[test]
fn test_derive_key_different_passwords_produce_different_keys() {
    let salt = [42u8; 16];
    let key_a = derive_key("password_a", &salt);
    let key_b = derive_key("password_b", &salt);
    assert_ne!(key_a, key_b);
}

#[test]
fn test_derive_key_different_salts_produce_different_keys() {
    let key_a = derive_key("same_password", &[0u8; 16]);
    let key_b = derive_key("same_password", &[1u8; 16]);
    assert_ne!(key_a, key_b);
}

#[test]
fn test_derive_key_output_is_32_bytes() {
    let key = derive_key("test", &[0u8; 16]);
    assert_eq!(key.len(), 32);
}

// ── generate_salt ─────────────────────────────────────────────────

#[test]
fn test_generate_salt_is_16_bytes() {
    let salt = generate_salt();
    assert_eq!(salt.len(), 16);
}

#[test]
fn test_generate_salt_produces_different_values() {
    let s1 = generate_salt();
    let s2 = generate_salt();
    // 극히 낮은 확률로 같을 수 있으나 CSPRNG이면 사실상 불가
    assert_ne!(s1, s2, "두 번 호출한 salt가 같으면 RNG가 고장난 것");
}

// ── encrypt / decrypt ─────────────────────────────────────────────

#[test]
fn test_encrypt_decrypt_roundtrip_ascii() {
    let key = derive_key("test_password", &[7u8; 16]);
    let plaintext = "Hello, World!";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn test_encrypt_decrypt_roundtrip_korean() {
    let key = derive_key("비밀번호1234", &[9u8; 16]);
    let plaintext = "홍길동은 활발하고 리더십이 뛰어나며 학급 회의를 주도적으로 이끌었다.";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn test_encrypt_same_plaintext_produces_different_ciphertext() {
    let key = derive_key("pw", &[3u8; 16]);
    let c1 = encrypt("동일한 내용", &key).unwrap();
    let c2 = encrypt("동일한 내용", &key).unwrap();
    // nonce가 매번 랜덤이므로 결과가 달라야 한다
    assert_ne!(c1, c2, "매번 다른 nonce를 써야 한다");
}

#[test]
fn test_decrypt_wrong_key_returns_error() {
    let key_enc = derive_key("correct_key", &[1u8; 16]);
    let key_dec = derive_key("wrong_key", &[1u8; 16]);
    let ciphertext = encrypt("비밀 내용", &key_enc).unwrap();
    let result = decrypt(&ciphertext, &key_dec);
    assert!(result.is_err(), "잘못된 키로 복호화하면 에러여야 한다");
    let err = result.unwrap_err();
    assert!(err.contains("복호화 실패"), "에러 메시지: {err}");
}

#[test]
fn test_decrypt_malformed_no_colon_returns_error() {
    let key = derive_key("pw", &[0u8; 16]);
    let result = decrypt("invaliddatawithnocolon", &key);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("잘못된 암호화 형식"), "에러 메시지: {err}");
}

#[test]
fn test_decrypt_invalid_base64_nonce_returns_error() {
    let key = derive_key("pw", &[0u8; 16]);
    let result = decrypt("not!valid!base64:YWJj", &key);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_invalid_base64_ciphertext_returns_error() {
    let key = derive_key("pw", &[0u8; 16]);
    // 유효한 base64 nonce + 유효하지 않은 ciphertext
    let result = decrypt("dGVzdA==:not!valid!base64", &key);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_invalid_nonce_length_returns_error() {
    let key = derive_key("pw", &[0u8; 16]);
    let result = decrypt("dGVzdA==:YWJj", &key);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("nonce 길이"), "에러 메시지: {err}");
}

#[test]
fn test_encrypt_invalid_key_length_returns_error() {
    let result = encrypt("test", &[0u8; 31]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("키 길이"), "에러 메시지: {err}");
}

#[test]
fn test_decrypt_invalid_key_length_returns_error() {
    let result = decrypt("AAAAAAAAAAAAAAAA:YWJj", &[0u8; 31]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("키 길이"), "에러 메시지: {err}");
}

#[test]
fn test_encrypt_output_format_has_colon_separator() {
    let key = derive_key("pw", &[0u8; 16]);
    let ciphertext = encrypt("test", &key).unwrap();
    let colon_count = ciphertext.chars().filter(|&c| c == ':').count();
    assert_eq!(colon_count, 1, "nonce:ciphertext 형식이어야 한다");
}

#[test]
fn test_encrypt_empty_string_roundtrip() {
    // maybe_encrypt/decrypt가 빈 문자열을 건너뛰므로 직접 encrypt는 성공해야 함
    let key = derive_key("pw", &[0u8; 16]);
    let ct = encrypt("", &key).unwrap();
    let pt = decrypt(&ct, &key).unwrap();
    assert_eq!(pt, "");
}

// ── maybe_decrypt ─────────────────────────────────────────────────

#[test]
fn test_maybe_decrypt_empty_string_returns_empty() {
    let key = derive_key("pw", &[0u8; 16]);
    let result = maybe_decrypt(String::new(), Some(key)).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_maybe_decrypt_no_key_returns_value_unchanged() {
    let result = maybe_decrypt("평문 데이터".to_string(), None).unwrap();
    assert_eq!(result, "평문 데이터");
}

#[test]
fn test_maybe_decrypt_with_key_decrypts() {
    let key = derive_key("pass", &[2u8; 16]);
    let encrypted = encrypt("복호화 대상", &key).unwrap();
    let result = maybe_decrypt(encrypted, Some(key)).unwrap();
    assert_eq!(result, "복호화 대상");
}

// ── maybe_encrypt ─────────────────────────────────────────────────

#[test]
fn test_maybe_encrypt_empty_string_returns_empty() {
    let key = derive_key("pw", &[0u8; 16]);
    let result = maybe_encrypt("", Some(key)).unwrap();
    assert_eq!(result, "");
}

#[test]
fn test_maybe_encrypt_no_key_returns_value_unchanged() {
    let result = maybe_encrypt("평문 그대로", None).unwrap();
    assert_eq!(result, "평문 그대로");
}

#[test]
fn test_maybe_encrypt_with_key_encrypts() {
    let key = derive_key("pass", &[5u8; 16]);
    let result = maybe_encrypt("암호화할 내용", Some(key)).unwrap();
    // 암호화된 결과는 원문과 달라야 한다
    assert_ne!(result, "암호화할 내용");
    // 복호화하면 원문이 나와야 한다
    let recovered = maybe_decrypt(result, Some(key)).unwrap();
    assert_eq!(recovered, "암호화할 내용");
}

#[test]
fn test_maybe_encrypt_decrypt_roundtrip_consistency() {
    let key = derive_key("일관성 테스트", &[11u8; 16]);
    let original = "복호화 일관성 확인용 내용입니다.";
    let encrypted = maybe_encrypt(original, Some(key)).unwrap();
    let decrypted = maybe_decrypt(encrypted, Some(key)).unwrap();
    assert_eq!(decrypted, original);
}
