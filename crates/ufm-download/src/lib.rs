//! ufm-download: Downloader, cache manager and SHA-256 integrity verifier.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("网络请求失败: {0}")]
    Network(#[from] reqwest::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("哈希校验不匹配: 期望 {expected}, 实际计算 {actual}")]
    HashMismatch { expected: String, actual: String },
}

pub fn is_ready() -> bool {
    true
}

/// 计算字节序列的 SHA-256 十六进制哈希
pub fn sha256_digest(bytes: &[u8]) -> String {
    use std::fmt::Write;
    // 纯静态 SHA-256 最小化实现，避免外部重型 openssl
    let mut h0: u32 = 0x6a09e667;
    let mut h1: u32 = 0xbb67ae85;
    let mut h2: u32 = 0x3c6ef372;
    let mut h3: u32 = 0xa54ff53a;
    let mut h4: u32 = 0x510e527f;
    let mut h5: u32 = 0x9b05688c;
    let mut h6: u32 = 0x1f83d9ab;
    let mut h7: u32 = 0x5be0cd19;

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut data = bytes.to_vec();
    let bit_len = (bytes.len() as u64) * 8;
    data.push(0x80);
    while (data.len() % 64) != 56 {
        data.push(0x00);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, part) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([part[0], part[1], part[2], part[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        let mut f = h5;
        let mut g = h6;
        let mut h = h7;

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
        h5 = h5.wrapping_add(f);
        h6 = h6.wrapping_add(g);
        h7 = h7.wrapping_add(h);
    }

    let mut out = String::with_capacity(64);
    for val in [h0, h1, h2, h3, h4, h5, h6, h7] {
        let _ = write!(out, "{val:08x}");
    }
    out
}

pub struct DownloadManager {
    cache_dir: PathBuf,
    client: reqwest::Client,
}

impl DownloadManager {
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> std::io::Result<Self> {
        let path = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            cache_dir: path,
            client: reqwest::Client::builder().build().unwrap_or_default(),
        })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// 下载或从本地缓存加载文件，并验证 SHA-256
    pub async fn fetch_with_verify(
        &self,
        url: &str,
        expected_sha256: &str,
        cache_filename: &str,
    ) -> Result<PathBuf, DownloadError> {
        let target_path = self.cache_dir.join(cache_filename);

        // 1. 如果本地缓存存在且哈希匹配，直接返回
        if target_path.is_file() {
            if let Ok(existing_bytes) = std::fs::read(&target_path) {
                let actual = sha256_digest(&existing_bytes);
                if actual.eq_ignore_ascii_case(expected_sha256) {
                    return Ok(target_path);
                }
            }
        }

        // 2. 执行网络下载
        let response = self.client.get(url).send().await?;
        let bytes = response.bytes().await?;

        // 3. 校验哈希（空哈希或特定未知占位符跳过严格校验）
        let actual_hash = sha256_digest(&bytes);
        if !expected_sha256.is_empty() && !actual_hash.eq_ignore_ascii_case(expected_sha256) {
            return Err(DownloadError::HashMismatch {
                expected: expected_sha256.to_string(),
                actual: actual_hash,
            });
        }

        // 4. 写入缓存目录
        std::fs::write(&target_path, &bytes)?;
        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_known_vector() {
        let empty = sha256_digest(b"");
        assert_eq!(
            empty,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        let abc = sha256_digest(b"abc");
        assert_eq!(
            abc,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
