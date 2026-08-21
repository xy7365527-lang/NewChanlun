//! SHA-256 摘要辅助（小写十六进制）。
//!
//! 全部内容身份（blob、bundle、Comparison Set Manifest、Lead 基线）都用 SHA-256
//! 十六进制小写表示，与 ADR 0025 的「SHA-256 content-addressed」一致。秘密值永不进入
//! 摘要身份（ADR 0025 §8.1）。

use sha2::{Digest, Sha256};

/// 对字节串求 SHA-256，返回小写十六进制串。
pub fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
