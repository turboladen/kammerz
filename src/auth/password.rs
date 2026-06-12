use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

/// Whether `hash` parses as a PHC-format password hash (e.g. argon2).
/// Used at startup to fail fast on a mangled `KAMMERZ_PASSWORD_HASH`
/// (dotenvy performs `$VAR` substitution on unquoted `.env` values, and an
/// argon2 hash is full of `$` tokens).
pub fn is_valid_hash(hash: &str) -> bool {
    PasswordHash::new(hash).is_ok()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        tracing::error!(
            "KAMMERZ_PASSWORD_HASH is not a valid argon2/PHC hash — login will always fail. \
             If it came from a .env file, single-quote the value: $-substitution mangles \
             unquoted argon2 hashes."
        );
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrips() {
        let h = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn is_valid_hash_accepts_real_hash() {
        let h = hash_password("hunter2").unwrap();
        assert!(is_valid_hash(&h));
    }

    #[test]
    fn is_valid_hash_rejects_dotenvy_mangled_hash() {
        // What an unquoted `KAMMERZ_PASSWORD_HASH=$argon2id$v=19$...` becomes
        // after dotenvy's $-substitution replaces the undefined vars with "".
        assert!(!is_valid_hash("=19=19456,t=2,p=1"));
        assert!(!is_valid_hash(""));
        assert!(!verify_password("hunter2", "=19=19456,t=2,p=1"));
    }
}
