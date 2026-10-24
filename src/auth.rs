use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use parking_lot::Mutex;
use rand::RngCore;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("Password hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash_str: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash_str) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

mod hex {
    pub fn encode(data: [u8; 32]) -> String {
        let mut s = String::with_capacity(64);
        for byte in data {
            use std::fmt::Write;
            let _ = write!(s, "{:02x}", byte);
        }
        s
    }
}

#[derive(Clone)]
pub struct LoginRateLimiter {
    state: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
    max_attempts: usize,
    window: Duration,
}

impl LoginRateLimiter {
    pub fn new(max_attempts: usize, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_attempts,
            window,
        }
    }

    pub fn check_and_record(&self, ip: IpAddr) -> bool {
        let mut map = self.state.lock();
        let now = Instant::now();
        let attempts = map.entry(ip).or_default();

        // Retain only attempts within the time window
        attempts.retain(|&time| now.saturating_duration_since(time) < self.window);

        if attempts.len() >= self.max_attempts {
            false
        } else {
            attempts.push(now);
            true
        }
    }
}
