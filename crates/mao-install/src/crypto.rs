//! The checks Funcom's patcher makes on what it downloads: the MD5 of each patch file against the step's `.CHK`
//! list, and the list's signature (`.SIG`) against the key in `cd_image/data/launcher/funcom.pub`.
//!
//! The signature check is `AnarchyPatcher.exe`'s `FUN_0046a800`, which is Crypto++'s `RSAVerifyFile` example word
//! for word: the key file and the signature file are hex (other characters skipped), the key an X.509
//! SubjectPublicKeyInfo, the scheme RSASSA-PKCS1-v1_5 with SHA-1 over the list's bytes as they are.
//!
//! Only verifying is here, with the little arithmetic it takes; nothing secret passes through this code.

/// MD5 (RFC 1321).
pub fn md5(data: &[u8]) -> [u8; 16] {
    const SHIFTS: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4,
        11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // (the table of the RFC: the whole part of 2^32 * |sin(i + 1)|)
    let table: [u32; 64] = std::array::from_fn(|i| ((i as f64 + 1.0).sin().abs() * 4294967296.0) as u32);
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&(data.len() as u64).wrapping_mul(8).to_le_bytes());
    for block in message.as_chunks::<64>().0 {
        let m: [u32; 16] = std::array::from_fn(|i| u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap()));
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let turned = a.wrapping_add(f).wrapping_add(table[i]).wrapping_add(m[g]).rotate_left(SHIFTS[i]);
            (a, d, c, b) = (d, c, b, b.wrapping_add(turned));
        }
        for (s, v) in state.iter_mut().zip([a, b, c, d]) {
            *s = s.wrapping_add(v);
        }
    }
    let mut out = [0; 16];
    for (i, s) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&s.to_le_bytes());
    }
    out
}

/// Bytes as lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bytes a text of hex digits stands for; anything that is no hex digit is skipped, as Crypto++'s decoder skips
/// it (line ends, spaces).
pub fn unhex(text: &[u8]) -> Vec<u8> {
    let digits: Vec<u8> = text.iter().filter_map(|&c| (c as char).to_digit(16).map(|d| d as u8)).collect();
    digits.as_chunks::<2>().0.iter().map(|pair| pair[0] << 4 | pair[1]).collect()
}

/// A whole number of any size, least significant word first, no zero words on top.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Big(Vec<u32>);

impl Big {
    fn from_be(bytes: &[u8]) -> Big {
        let mut words = Vec::with_capacity(bytes.len() / 4 + 1);
        for chunk in bytes.rchunks(4) {
            words.push(chunk.iter().fold(0u32, |w, &b| w << 8 | b as u32));
        }
        Big(words).trimmed()
    }

    fn to_be(&self, len: usize) -> Option<Vec<u8>> {
        let mut out = vec![0u8; len];
        for (i, word) in self.0.iter().enumerate() {
            for (j, byte) in word.to_le_bytes().into_iter().enumerate() {
                match len.checked_sub(i * 4 + j + 1) {
                    Some(at) => out[at] = byte,
                    None if byte == 0 => {}
                    None => return None,
                }
            }
        }
        Some(out)
    }

    fn trimmed(mut self) -> Big {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    fn bits(&self) -> usize {
        match self.0.last() {
            Some(top) => self.0.len() * 32 - top.leading_zeros() as usize,
            None => 0,
        }
    }

    fn bit(&self, i: usize) -> bool {
        self.0.get(i / 32).is_some_and(|word| word >> (i % 32) & 1 != 0)
    }

    fn less_than(&self, other: &Big) -> bool {
        if self.0.len() != other.0.len() {
            return self.0.len() < other.0.len();
        }
        for (a, b) in self.0.iter().rev().zip(other.0.iter().rev()) {
            if a != b {
                return a < b;
            }
        }
        false
    }

    fn sub(&mut self, other: &Big) {
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let v = self.0[i] as i64 - other.0.get(i).copied().unwrap_or(0) as i64 - borrow;
            borrow = (v < 0) as i64;
            self.0[i] = v as u32;
        }
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn doubled(&self) -> Big {
        let mut out = Vec::with_capacity(self.0.len() + 1);
        let mut carry = 0;
        for &word in &self.0 {
            out.push(word << 1 | carry);
            carry = word >> 31;
        }
        if carry != 0 {
            out.push(carry);
        }
        Big(out)
    }

    /// `self * other mod modulus`, bit by bit (slow and plain: one signature a patch step).
    fn mul_mod(&self, other: &Big, modulus: &Big) -> Big {
        let mut sum = Big(Vec::new());
        for i in (0..other.bits()).rev() {
            sum = sum.doubled();
            if !sum.less_than(modulus) {
                sum.sub(modulus);
            }
            if other.bit(i) {
                let mut carry = 0u64;
                let mut words = Vec::with_capacity(sum.0.len().max(self.0.len()) + 1);
                for j in 0..sum.0.len().max(self.0.len()) {
                    let v = sum.0.get(j).copied().unwrap_or(0) as u64 + self.0.get(j).copied().unwrap_or(0) as u64 + carry;
                    words.push(v as u32);
                    carry = v >> 32;
                }
                if carry != 0 {
                    words.push(carry as u32);
                }
                sum = Big(words);
                if !sum.less_than(modulus) {
                    sum.sub(modulus);
                }
            }
        }
        sum
    }

    fn pow_mod(&self, exponent: &Big, modulus: &Big) -> Big {
        let mut result = Big(vec![1]);
        for i in (0..exponent.bits()).rev() {
            result = result.mul_mod(&result, modulus);
            if exponent.bit(i) {
                result = result.mul_mod(self, modulus);
            }
        }
        result
    }
}

/// One DER element at the start of `d`: its tag, its contents, and what follows it.
fn der(d: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = d.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (len, rest) = if first < 0x80 {
        (first as usize, rest)
    } else {
        let n = (first & 0x7f) as usize;
        if n == 0 || n > 4 || rest.len() < n {
            return None;
        }
        (rest[..n].iter().fold(0usize, |v, &b| v << 8 | b as usize), &rest[n..])
    };
    (rest.len() >= len).then(|| (tag, &rest[..len], &rest[len..]))
}

/// An RSA public key from its X.509 SubjectPublicKeyInfo: the modulus and the exponent, big-endian.
fn rsa_key(spki: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let (0x30, outer, _) = der(spki)? else { return None };
    let (0x30, algorithm, rest) = der(outer)? else { return None };
    // (rsaEncryption, 1.2.840.113549.1.1.1)
    let (0x06, oid, _) = der(algorithm)? else { return None };
    if oid != [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01] {
        return None;
    }
    let (0x03, bits, _) = der(rest)? else { return None };
    let (0x30, key, _) = der(bits.split_first().filter(|(unused, _)| **unused == 0)?.1)? else { return None };
    let (0x02, modulus, rest) = der(key)? else { return None };
    let (0x02, exponent, _) = der(rest)? else { return None };
    let strip = |n: &[u8]| n.iter().skip_while(|&&b| b == 0).copied().collect::<Vec<u8>>();
    Some((strip(modulus), strip(exponent)))
}

/// Whether `signature` is the key's RSASSA-PKCS1-v1_5 signature of a message whose SHA-1 is `digest`. The key is an
/// X.509 SubjectPublicKeyInfo (DER).
pub fn rsa_sha1_verify(key: &[u8], digest: &[u8; 20], signature: &[u8]) -> bool {
    let Some((modulus, exponent)) = rsa_key(key) else { return false };
    // (the encoded message: 00 01, ff bytes, 00, the DigestInfo of SHA-1, the digest)
    const DIGEST_INFO: [u8; 15] = [0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14];
    let k = modulus.len();
    if signature.len() != k || k < DIGEST_INFO.len() + 20 + 11 || exponent.is_empty() {
        return false;
    }
    let n = Big::from_be(&modulus);
    let s = Big::from_be(signature);
    if !s.less_than(&n) {
        return false;
    }
    let Some(encoded) = s.pow_mod(&Big::from_be(&exponent), &n).to_be(k) else { return false };
    let mut expected = vec![0xff; k];
    expected[0] = 0;
    expected[1] = 1;
    let tail = k - 20 - DIGEST_INFO.len();
    expected[tail - 1] = 0;
    expected[tail..tail + DIGEST_INFO.len()].copy_from_slice(&DIGEST_INFO);
    expected[k - 20..].copy_from_slice(digest);
    encoded == expected
}

/// A checksum list (`checksums.chk`, a step's `.CHK`): each line `name:md5` in hex; lines that start with `#` are
/// remarks.
pub fn checksum_list(text: &str) -> Vec<(String, [u8; 16])> {
    let mut list = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((name, sum)) = line.rsplit_once(':') else { continue };
        if let Ok(sum) = <[u8; 16]>::try_from(unhex(sum.as_bytes())) {
            list.push((name.to_owned(), sum));
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_of_the_rfc() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(hex(&md5(b"message digest")), "f96b697d7cb7938d525a2f31aaf161d0");
        let long = b"12345678901234567890123456789012345678901234567890123456789012345678901234567890";
        assert_eq!(hex(&md5(long)), "57edf4a22be3c955ac49da2e2107b67a");
    }

    #[test]
    fn hex_both_ways() {
        assert_eq!(unhex(b"0aFF\r\n10"), [0x0a, 0xff, 0x10]);
        assert_eq!(hex(&[0x0a, 0xff, 0x10]), "0aff10");
    }

    #[test]
    fn big_numbers() {
        let n = Big::from_be(&[0x01, 0x00, 0x00, 0x00, 0x07]);
        assert_eq!(n.0, [7, 1]);
        assert_eq!(n.to_be(6).unwrap(), [0, 1, 0, 0, 0, 7]);
        assert!(n.to_be(4).is_none());
        // 4^13 mod 497 = 445; and with a modulus of more than one word
        let pow = |b: u32, e: u32, m: u64| {
            Big(vec![b]).pow_mod(&Big(vec![e]), &Big(vec![m as u32, (m >> 32) as u32]).trimmed())
        };
        assert_eq!(pow(4, 13, 497).0, [445]);
        let m = 0x1_0000_000fu64;
        let expected = (0..100).fold(1u128, |v, _| v * 3 % m as u128) as u64;
        assert_eq!(pow(3, 100, m), Big(vec![expected as u32, (expected >> 32) as u32]).trimmed());
    }

    #[test]
    fn a_key_and_no_signature() {
        // A made-up modulus (odd, 64 bytes) in a SubjectPublicKeyInfo: the key is read, and nothing that was not
        // made with a key's secret half passes. (A real signature is checked in tests/patches.rs.)
        let modulus: Vec<u8> = (0..64u32).map(|i| (i * 37 + 0x81) as u8 | (i == 63) as u8).collect();
        let wrap = |tag: u8, body: &[u8]| {
            let mut out = vec![tag];
            if body.len() < 0x80 {
                out.push(body.len() as u8);
            } else {
                out.extend_from_slice(&[0x81, body.len() as u8]);
            }
            out.extend_from_slice(body);
            out
        };
        let mut integer = vec![0];
        integer.extend_from_slice(&modulus);
        let key = [wrap(0x02, &integer), wrap(0x02, &[1, 0, 1])].concat();
        let mut bits = vec![0];
        bits.extend_from_slice(&wrap(0x30, &key));
        let algorithm = wrap(0x30, &[wrap(0x06, &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01]), vec![5, 0]].concat());
        let spki = wrap(0x30, &[algorithm, wrap(0x03, &bits)].concat());
        assert_eq!(rsa_key(&spki).unwrap(), (modulus.clone(), vec![1, 0, 1]));
        let digest = [7u8; 20];
        let mut signature = vec![0x5a; 64];
        signature[0] = 0;
        assert!(!rsa_sha1_verify(&spki, &digest, &signature));
        assert!(!rsa_sha1_verify(&spki, &digest, &signature[1..]));
        assert!(!rsa_sha1_verify(&spki[1..], &digest, &signature));
        assert!(!rsa_sha1_verify(&spki, &digest, &modulus));
    }

    #[test]
    fn checksum_lists() {
        let list = checksum_list("# remark\r\nA.RTP:2559878c6b6c0edf3007632b79f91a2f\r\ncd_image/x:y.z:d41d8cd98f00b204e9800998ecf8427e\r\nbad:12\r\n");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "A.RTP");
        assert_eq!(list[1], ("cd_image/x:y.z".to_owned(), md5(b"")));
    }
}
