//! #1527：旧 evidence-graph/2、/3 的纯行验证器，沿用原算法及原错误顺序。
//! 只证明原行；不证明 SEQ 展开、来源、闭包或准入费用，不授旧域生产资格。

/// 节点种类：0 = 精确字节叶；1 = 有序拼接节点。
pub const KIND_LEAF: i64 = 0;
pub const KIND_SEQ: i64 = 1;

pub fn node_id(kind: i64, payload: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update([kind as u8]);
    h.update(payload);
    h.finalize().into()
}

pub fn verify_node_row(id: &[u8; 32], kind: i64, payload: &[u8], len: i64) -> Result<u64, String> {
    if kind != KIND_LEAF && kind != KIND_SEQ {
        return Err(format!(
            "StorageUnavailable：私有节点 {} 种类 `{kind}` 未声明",
            hex(id)
        ));
    }
    if node_id(kind, payload) != *id {
        return Err(format!(
            "StorageUnavailable：私有节点 {} 同 ID 异内容（内容寻址不符）",
            hex(id)
        ));
    }
    let declared = u64::try_from(len)
        .map_err(|_| format!("StorageUnavailable：私有节点 {} 长度非负越界", hex(id)))?;
    if kind == KIND_LEAF {
        if declared != payload.len() as u64 {
            return Err(format!(
                "StorageUnavailable：私有叶 {} 声明长度与实际不符",
                hex(id)
            ));
        }
    }
    Ok(declared)
}

pub fn hex(id: &[u8; 32]) -> String {
    hex_bytes(id)
}

pub fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 经实际原行验证构造的 opaque 凭据；不持有 payload，也不分配长期集合。
/// 所有数字均从原验证成功的输入派生；没有 Usage、反序列化或 unchecked 构造口。
pub struct CheckedLegacyEvidenceRow {
    id: [u8; 32],
    kind: i64,
    payload_len: usize,
    declared: u64,
}

impl CheckedLegacyEvidenceRow {
    pub fn verify(id: &[u8; 32], kind: i64, payload: &[u8], len: i64) -> Result<Self, String> {
        let declared = verify_node_row(id, kind, payload, len)?;
        Ok(Self {
            id: *id,
            kind,
            payload_len: payload.len(),
            declared,
        })
    }

    pub fn id(&self) -> &[u8; 32] {
        &self.id
    }
    pub fn kind(&self) -> i64 {
        self.kind
    }
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }
    pub fn declared_length(&self) -> u64 {
        self.declared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOT_RUN：#1527 授权仅静态与小模型；不是已执行的 Rust 对拍。
    #[test]
    fn original_error_order_and_checked_row_inputs() {
        let payload = b"abc";
        let id = node_id(KIND_LEAF, payload);
        let checked = CheckedLegacyEvidenceRow::verify(&id, KIND_LEAF, payload, 3).unwrap();
        assert_eq!(checked.id(), &id);
        assert_eq!(checked.kind(), KIND_LEAF);
        assert_eq!(checked.payload_len(), 3);
        assert_eq!(checked.declared_length(), 3);
        let hex_id = hex(&id);
        for (kind, bytes, len, expected) in [
            (
                256,
                &b"changed"[..],
                -1,
                format!("StorageUnavailable：私有节点 {hex_id} 种类 `256` 未声明"),
            ),
            (
                KIND_LEAF,
                &b"changed"[..],
                -1,
                format!("StorageUnavailable：私有节点 {hex_id} 同 ID 异内容（内容寻址不符）"),
            ),
            (
                KIND_LEAF,
                &payload[..],
                -1,
                format!("StorageUnavailable：私有节点 {hex_id} 长度非负越界"),
            ),
            (
                KIND_LEAF,
                &payload[..],
                4,
                format!("StorageUnavailable：私有叶 {hex_id} 声明长度与实际不符"),
            ),
        ] {
            assert_eq!(
                verify_node_row(&id, kind, bytes, len),
                Err(expected.clone())
            );
            assert_eq!(
                CheckedLegacyEvidenceRow::verify(&id, kind, bytes, len).err(),
                Some(expected)
            );
        }
        // 行凭据从不替代 SEQ 语法/展开验证；沿用旧行验证器的接受域。
        let seq = b"not a sequence";
        let seq_id = node_id(KIND_SEQ, seq);
        assert_eq!(
            CheckedLegacyEvidenceRow::verify(&seq_id, KIND_SEQ, seq, i64::MAX)
                .unwrap()
                .declared_length(),
            i64::MAX as u64
        );
    }
}
