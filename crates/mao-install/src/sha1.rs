//! SHA-1 (FIPS 180-4): what Inno Setup 5.3.9 and later store for each packed file, what Funcom's signatures of the
//! patches' checksum lists are made over, and what the installer this code knows is told by.

/// A running SHA-1 sum.
pub struct Sha1 {
    state: [u32; 5],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    pub fn new() -> Self {
        Sha1 {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0],
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.length += data.len() as u64;
        if self.filled > 0 {
            let take = (64 - self.filled).min(data.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&data[..take]);
            self.filled += take;
            data = &data[take..];
            if self.filled < 64 {
                return;
            }
            let block = self.block;
            compress(&mut self.state, &block);
            self.filled = 0;
        }
        let (blocks, rest) = data.as_chunks::<64>();
        for block in blocks {
            compress(&mut self.state, block);
        }
        self.block[..rest.len()].copy_from_slice(rest);
        self.filled = rest.len();
    }

    pub fn finish(mut self) -> [u8; 20] {
        let bits = self.length.wrapping_mul(8);
        let mut pad = [0u8; 72];
        pad[0] = 0x80;
        let pad_len = if self.filled < 56 { 56 - self.filled } else { 120 - self.filled };
        pad[pad_len..pad_len + 8].copy_from_slice(&bits.to_be_bytes());
        self.update(&pad[..pad_len + 8]);
        let mut out = [0u8; 20];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

fn compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*word);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for (i, &word) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | (!b & d), 0x5a827999),
            20..=39 => (b ^ c ^ d, 0x6ed9eba1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
            _ => (b ^ c ^ d, 0xca62c1d6u32),
        };
        let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(word);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

#[cfg(test)]
mod tests {
    use super::Sha1;

    fn hex(sum: [u8; 20]) -> String {
        sum.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn known_sums() {
        assert_eq!(hex(Sha1::new().finish()), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        let mut s = Sha1::new();
        s.update(b"abc");
        assert_eq!(hex(s.finish()), "a9993e364706816aba3e25717850c26c9cd0d89d");
        let mut s = Sha1::new();
        s.update(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
        assert_eq!(hex(s.finish()), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
    }

    #[test]
    fn pieces_give_the_same_sum() {
        // A million 'a's, fed in uneven pieces.
        let data = vec![b'a'; 1_000_000];
        let mut s = Sha1::new();
        let mut at = 0;
        let mut step = 1;
        while at < data.len() {
            let end = (at + step).min(data.len());
            s.update(&data[at..end]);
            at = end;
            step = step * 3 % 1000 + 1;
        }
        assert_eq!(hex(s.finish()), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    }
}
