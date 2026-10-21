//! The login's challenge response, from `Client_t::MakeChallengeResponse`
//! and the helpers it calls in `Interfaces.dll`.
//!
//! The server sends a 32-character challenge. The client picks a random
//! 128-bit `r` and answers `"<A>-<E>"`, where
//!
//! * `A = 5^r mod N` (N the 1024-bit prime below), in lowercase hex;
//! * `K = S^r mod N` (S the server's fixed public value below); the first
//!   32 hex digits of K, read as 16 bytes, key a TEA cipher;
//! * `E` is the hex of `"<user>|<challenge>|<password>"` encrypted with TEA
//!   (32 rounds) in a CBC-like chain: the plaintext is 8 random bytes, the
//!   message length (big-endian u32) and the message, padded with spaces to
//!   a multiple of eight bytes.

use num_bigint::BigUint;

/// The prime N.
const PRIME: &str = "eca2e8c85d863dcdc26a429a71a9815ad052f6139669dd659f98ae159d313d13c6bf2838e10a69b6478b64a24bd054ba8248e8fa778703b418408249440b2c1edd28853e240d8a7e49540b76d120d3b1ad2878b1b99490eb4a2a5e84caa8a91cecbdb1aa7c816e8be343246f80c637abc653b893fd91686cf8d32d6cfe5f2a6f";
/// The server's public value S.
const SERVER_KEY: &str = "9c32cc23d559ca90fc31be72df817d0e124769e809f936bc14360ff4bed758f260a0d596584eacbbc2b88bdd410416163e11dbf62173393fbc0c6fefb2d855f1a03dec8e9f105bbad91b3437d8eb73fe2f44159597aa4053cf788d2f9d7012fb8d7c4ce3876f7d6cd5d0c31754f4cd96166708641958de54a6def5657b9f2e92";
const GENERATOR: u32 = 5;

fn tea_encrypt(block: &mut [u32; 2], key: &[u32; 4]) {
    let (mut v0, mut v1) = (block[0], block[1]);
    let mut sum = 0u32;
    for _ in 0..32 {
        sum = sum.wrapping_sub(0x61c8_8647);
        v0 = v0.wrapping_add((v1 << 4).wrapping_add(key[0]) ^ v1.wrapping_add(sum) ^ (v1 >> 5).wrapping_add(key[1]));
        v1 = v1.wrapping_add((v0 << 4).wrapping_add(key[2]) ^ v0.wrapping_add(sum) ^ (v0 >> 5).wrapping_add(key[3]));
    }
    *block = [v0, v1];
}

#[cfg(test)]
fn tea_decrypt(block: &mut [u32; 2], key: &[u32; 4]) {
    let (mut v0, mut v1) = (block[0], block[1]);
    let mut sum = 0u32.wrapping_sub(0x61c8_8647u32.wrapping_mul(32));
    for _ in 0..32 {
        v1 = v1.wrapping_sub((v0 << 4).wrapping_add(key[2]) ^ v0.wrapping_add(sum) ^ (v0 >> 5).wrapping_add(key[3]));
        v0 = v0.wrapping_sub((v1 << 4).wrapping_add(key[0]) ^ v1.wrapping_add(sum) ^ (v1 >> 5).wrapping_add(key[1]));
        sum = sum.wrapping_add(0x61c8_8647);
    }
    *block = [v0, v1];
}

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
}

/// The message encrypted as the original does (see the module docs);
/// `iv` is the 8 random bytes. The result is the ciphertext's bytes.
pub fn encrypt_message(key: &[u8; 16], iv: [u8; 8], message: &[u8]) -> Vec<u8> {
    let length = message.len() + 12;
    let total = length + (8 - length % 8);
    let mut buffer = vec![0x20u8; total];
    buffer[..8].copy_from_slice(&iv);
    buffer[8..12].copy_from_slice(&(message.len() as u32).to_be_bytes());
    buffer[12..length].copy_from_slice(message);
    let k = words(key);
    let k = [k[0], k[1], k[2], k[3]];
    let mut previous = [0u32; 2];
    for chunk in buffer.chunks_exact_mut(8) {
        let w = words(chunk);
        let mut block = [w[0], w[1]];
        // The original only chains when the previous block's first word is
        // not zero (and then both words).
        if previous[0] != 0 {
            block[0] ^= previous[0];
            block[1] ^= previous[1];
        }
        tea_encrypt(&mut block, &k);
        chunk[..4].copy_from_slice(&block[0].to_le_bytes());
        chunk[4..].copy_from_slice(&block[1].to_le_bytes());
        previous = block;
    }
    buffer
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The response to `challenge` from the 128-bit `r` and 8 bytes `iv`.
/// Returns `None` only if the shared key's hex is shorter than 32 digits
/// (the original reports an error then).
pub fn response_with(user: &str, challenge: &str, password: &str, r: [u8; 16], iv: [u8; 8]) -> Option<String> {
    let n = BigUint::parse_bytes(PRIME.as_bytes(), 16).unwrap();
    let s = BigUint::parse_bytes(SERVER_KEY.as_bytes(), 16).unwrap();
    let exponent = BigUint::from_bytes_be(&r);
    let a = BigUint::from(GENERATOR).modpow(&exponent, &n);
    let shared = s.modpow(&exponent, &n).to_str_radix(16);
    if shared.len() < 32 {
        return None;
    }
    let mut key = [0u8; 16];
    for (i, k) in key.iter_mut().enumerate() {
        *k = u8::from_str_radix(&shared[2 * i..2 * i + 2], 16).ok()?;
    }
    let message = format!("{user}|{challenge}|{password}");
    let cipher = encrypt_message(&key, iv, message.as_bytes());
    Some(format!("{}-{}", a.to_str_radix(16), hex(&cipher)))
}

/// The response with fresh randomness.
pub fn response(user: &str, challenge: &str, password: &str) -> Result<String, getrandom::Error> {
    loop {
        let mut random = [0u8; 24];
        getrandom::fill(&mut random)?;
        let (r, iv) = random.split_at(16);
        if let Some(text) = response_with(user, challenge, password, r.try_into().unwrap(), iv.try_into().unwrap()) {
            return Ok(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference vector of TEA with a zero key and block.
    #[test]
    fn tea_reference_vector() {
        let mut b = [0u32; 2];
        tea_encrypt(&mut b, &[0; 4]);
        assert_eq!(b, [0x41ea_3a0a, 0x94ba_a940]);
    }

    #[test]
    fn tea_round_trip() {
        let key = [1, 2, 3, 4];
        let mut b = [0xdead_beef, 0x0123_4567];
        tea_encrypt(&mut b, &key);
        tea_decrypt(&mut b, &key);
        assert_eq!(b, [0xdead_beef, 0x0123_4567]);
    }

    /// The shape of a real response: 256 hex digits, a dash, and the
    /// ciphertext of a 6+1+32+1+pw message (64 bytes for a 5-11 letter
    /// password), as in the capture.
    #[test]
    fn response_shape() {
        let r = response_with("nerfit", "75185ea0b3e792676b76b0741410e9d4", "secret1", [0x55; 16], [7; 8]).unwrap();
        let (a, e) = r.split_once('-').unwrap();
        assert!(a.len() >= 250 && a.len() <= 256);
        assert_eq!(e.len(), 128);
    }

    /// The ciphertext decrypts (chained back) to the iv, the length and the
    /// message.
    #[test]
    fn message_decrypts() {
        let key = [9u8; 16];
        let cipher = encrypt_message(&key, [3; 8], b"hello|world|pw");
        let k = words(&key);
        let k = [k[0], k[1], k[2], k[3]];
        let mut previous = [0u32; 2];
        let mut plain = Vec::new();
        for chunk in cipher.chunks_exact(8) {
            let w = words(chunk);
            let mut block = [w[0], w[1]];
            tea_decrypt(&mut block, &k);
            if previous[0] != 0 {
                block[0] ^= previous[0];
                block[1] ^= previous[1];
            }
            previous = [w[0], w[1]];
            plain.extend_from_slice(&block[0].to_le_bytes());
            plain.extend_from_slice(&block[1].to_le_bytes());
        }
        assert_eq!(&plain[..8], &[3; 8]);
        assert_eq!(&plain[8..12], &14u32.to_be_bytes());
        assert_eq!(&plain[12..26], b"hello|world|pw");
        assert!(plain[26..].iter().all(|&b| b == 0x20));
    }
}
