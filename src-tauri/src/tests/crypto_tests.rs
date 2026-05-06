use crate::crypto::{decrypt, derive_key, encrypt, generate_salt, maybe_decrypt, maybe_encrypt};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

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

// ── 추가 엣지 케이스: 긴 문자열 / 특수 입력 ──────────────────────

#[test]
fn test_encrypt_decrypt_roundtrip_long_string() {
    let key = derive_key("long_string_pw", &[10u8; 16]);
    // 14자 × 357 ≈ 5000자 한국어 문자열
    let plaintext = "가나다라마바사아자차카타파하".repeat(357);
    let ciphertext = encrypt(&plaintext, &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn test_encrypt_decrypt_roundtrip_special_chars() {
    let key = derive_key("special_chars_pw", &[20u8; 16]);
    // 이모지 · 한중일 · 제어문자 · 특수기호 혼합
    let plaintext = "안녕😀🎉你好世界こんにちは\0\t\n\r\x1b<>&\"'`\\|{}[]";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn test_encrypt_decrypt_roundtrip_whitespace_only() {
    let key = derive_key("whitespace_pw", &[30u8; 16]);
    // 공백·탭·개행만 있는 문자열 – maybe_encrypt의 is_empty() 검사를 통과해야 한다
    let plaintext = "   \t\n  ";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn test_encrypt_decrypt_roundtrip_content_with_colon() {
    let key = derive_key("colon_pw", &[40u8; 16]);
    // 평문에 콜론 다수 포함 – split_once(':')가 첫 콜론만 분리하므로 형식 충돌 없음
    let plaintext = "홍:길:동:의:발:표";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    // 암호화 출력은 정확히 콜론 1개 (nonce:ciphertext)
    let colon_count = ciphertext.chars().filter(|&c| c == ':').count();
    assert_eq!(colon_count, 1, "암호화 출력은 nonce:ciphertext 형식이어야 한다");
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

// ── derive_key 경계 조건 ──────────────────────────────────────────

#[test]
fn test_derive_key_empty_password_is_32_bytes() {
    // 빈 패스워드로도 PBKDF2가 32바이트 비영 키를 생성해야 한다
    let key = derive_key("", &[1u8; 16]);
    assert_eq!(key.len(), 32);
    assert_ne!(key, [0u8; 32]);
}

#[test]
fn test_derive_key_unicode_password_is_32_bytes() {
    let key = derive_key("비밀번호🔐안전", &[99u8; 16]);
    assert_eq!(key.len(), 32);
    assert_ne!(key, [0u8; 32]);
}

#[test]
fn test_derive_key_long_password_is_32_bytes() {
    // 1000자 패스워드 – PBKDF2는 길이 제한 없음
    let long_pw = "비밀번호".repeat(250);
    let key = derive_key(&long_pw, &[1u8; 16]);
    assert_eq!(key.len(), 32);
    assert_ne!(key, [0u8; 32]);
}

#[test]
fn test_derive_key_all_zero_salt_produces_nonzero_key() {
    let key = derive_key("some_password", &[0u8; 16]);
    assert_eq!(key.len(), 32);
    assert_ne!(key, [0u8; 32], "all-zero salt에서도 키가 영벡터면 안 된다");
}

#[test]
fn test_derive_key_consistency_multiple_calls() {
    let password = "일관성 테스트 패스워드";
    let salt = [123u8; 16];
    let keys: Vec<[u8; 32]> = (0..10).map(|_| derive_key(password, &salt)).collect();
    for k in &keys {
        assert_eq!(*k, keys[0], "derive_key는 항상 동일한 결과를 반환해야 한다");
    }
}

// ── 잘못된 키 길이 전체 케이스 ────────────────────────────────────

#[test]
fn test_encrypt_various_invalid_key_lengths_return_error() {
    // 32바이트가 아닌 키는 모두 에러여야 한다
    for len in [0usize, 1, 16, 31, 33, 64] {
        let key = vec![0u8; len];
        let result = encrypt("test", &key);
        assert!(result.is_err(), "{}바이트 키는 에러여야 한다", len);
        assert!(
            result.unwrap_err().contains("키 길이"),
            "{}바이트 키 에러 메시지 확인",
            len
        );
    }
}

// ── 경계 평문 ─────────────────────────────────────────────────────

#[test]
fn test_encrypt_all_zero_32byte_key_roundtrip() {
    // 모든 0 키(32바이트)도 유효한 AES-256 키
    let key = [0u8; 32];
    let plaintext = "영벡터 키로도 암호화 가능해야 한다";
    let ciphertext = encrypt(plaintext, &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn test_encrypt_single_byte_plaintext_roundtrip() {
    let key = derive_key("single_byte_pw", &[5u8; 16]);
    let ciphertext = encrypt("A", &key).unwrap();
    let recovered = decrypt(&ciphertext, &key).unwrap();
    assert_eq!(recovered, "A");
}

// ── 복호화 오류 경계 ─────────────────────────────────────────────

#[test]
fn test_decrypt_empty_string_returns_error() {
    let key = derive_key("empty_decrypt_pw", &[0u8; 16]);
    let result = decrypt("", &key);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("잘못된 암호화 형식"));
}

#[test]
fn test_decrypt_colon_only_returns_error() {
    let key = derive_key("colon_only_pw", &[0u8; 16]);
    // ":" → nonce_b64="" → B64.decode("") = [] → 길이 0 ≠ 12 → nonce 길이 에러
    let result = decrypt(":", &key);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("nonce 길이"), "에러 메시지: {err}");
}

#[test]
fn test_decrypt_correct_key_succeeds_wrong_key_fails() {
    let correct_key = derive_key("correct", &[1u8; 16]);
    let wrong_key1 = derive_key("wrong1", &[1u8; 16]);
    let wrong_key2 = derive_key("correct", &[2u8; 16]); // 동일 패스워드, 다른 salt
    let ciphertext = encrypt("비밀 데이터", &correct_key).unwrap();
    assert!(decrypt(&ciphertext, &correct_key).is_ok());
    assert!(decrypt(&ciphertext, &wrong_key1).is_err());
    assert!(decrypt(&ciphertext, &wrong_key2).is_err());
}

// ── 변조 감지 (AEAD 무결성 보증) ─────────────────────────────────

#[test]
fn test_decrypt_tampered_ciphertext_returns_error() {
    let key = derive_key("tamper_pw", &[55u8; 16]);
    let ciphertext = encrypt("변조되면 안 되는 내용", &key).unwrap();
    let (nonce_b64, cipher_b64) = ciphertext.split_once(':').unwrap();
    let mut raw = B64.decode(cipher_b64).unwrap();
    raw[0] ^= 0xFF; // 1바이트 반전
    let tampered = format!("{}:{}", nonce_b64, B64.encode(&raw));
    let result = decrypt(&tampered, &key);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("복호화 실패"));
}

#[test]
fn test_decrypt_tampered_nonce_returns_error() {
    let key = derive_key("nonce_tamper_pw", &[66u8; 16]);
    let ciphertext = encrypt("nonce 변조 테스트", &key).unwrap();
    let (nonce_b64, cipher_b64) = ciphertext.split_once(':').unwrap();
    let mut nonce_bytes = B64.decode(nonce_b64).unwrap();
    nonce_bytes[0] ^= 0x01;
    let tampered = format!("{}:{}", B64.encode(&nonce_bytes), cipher_b64);
    let result = decrypt(&tampered, &key);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("복호화 실패"));
}

// ── nonce 유일성 / 이중 암호화 ────────────────────────────────────

#[test]
fn test_encrypt_nonce_uniqueness_repeated_calls() {
    let key = derive_key("nonce_test_pw", &[77u8; 16]);
    let mut results = std::collections::HashSet::new();
    for _ in 0..100 {
        let ct = encrypt("동일한 평문", &key).unwrap();
        results.insert(ct);
    }
    assert_eq!(results.len(), 100, "100회 암호화 결과가 모두 달라야 한다 (nonce 재사용 없음)");
}

#[test]
fn test_maybe_encrypt_double_encrypt_requires_double_decrypt() {
    let key = derive_key("double_pw", &[22u8; 16]);
    let original = "이중 암호화 테스트";
    let once = maybe_encrypt(original, Some(key)).unwrap();
    let twice = maybe_encrypt(&once, Some(key)).unwrap();
    assert_ne!(twice, original);
    // 두 번 복호화해야 원문이 나온다
    let step1 = maybe_decrypt(twice, Some(key)).unwrap();
    let step2 = maybe_decrypt(step1, Some(key)).unwrap();
    assert_eq!(step2, original);
}

// ── 출력 형식 검증 ────────────────────────────────────────────────

#[test]
fn test_encrypt_output_contains_only_ascii() {
    let key = derive_key("ascii_test_pw", &[33u8; 16]);
    let ciphertext = encrypt("한국어 내용 테스트 Korean", &key).unwrap();
    assert!(
        ciphertext.is_ascii(),
        "암호화 출력은 ASCII(base64)만 포함해야 한다: {ciphertext}"
    );
}

// ── maybe_decrypt 엣지 ────────────────────────────────────────────

#[test]
fn test_maybe_decrypt_plaintext_with_key_returns_error() {
    let key = derive_key("plain_with_key_pw", &[88u8; 16]);
    // 콜론 없는 평문 → 잘못된 암호화 형식 에러
    let result = maybe_decrypt("평문데이터입니다".to_string(), Some(key));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("잘못된 암호화 형식"));
    // 콜론 있지만 유효하지 않은 base64 nonce → 에러
    let result2 = maybe_decrypt("평문:데이터".to_string(), Some(key));
    assert!(result2.is_err());
}
