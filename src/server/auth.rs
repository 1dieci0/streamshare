use sha2::{Sha256, Digest};

pub const SERVER_KEY: &str = "super_secret_key";


pub fn generate_challenge() -> [u8; 32] {
    let mut challenge = [0u8; 32];

    rand::fill(&mut challenge);

    challenge
}


pub fn create_response(key: &str, challenge: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();

    hasher.update(key.as_bytes());
    hasher.update(challenge);

    hasher.finalize().to_vec()
}
