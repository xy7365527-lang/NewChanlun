//! #1527：生产mixed与Native测试臂共用的唯一JSON布局测量核。
use std::mem::size_of;

pub(crate) fn sum(values: impl IntoIterator<Item = usize>) -> usize {
    values.into_iter().fold(0usize, usize::saturating_add)
}
pub(crate) fn vec_bytes<T>(capacity: usize) -> usize {
    capacity.saturating_mul(size_of::<T>())
}
pub(crate) fn tree_slots<K, V>(len: usize) -> usize {
    // 每个条目最多独占一棵含 11 key/value、12 edge 的节点；上界刻意不减费。
    len.saturating_mul(
        size_of::<(K, V)>()
            .saturating_mul(11)
            .saturating_add(16 * size_of::<usize>()),
    )
}

/// 无分配词法计数，只产生内存上界；仍调用原 strict_json/原业务谓词，不赋予语法合法资格。
/// 每个 value token 计一个可能数组槽（包含对象值/键会高估）；Vec 扩容峰值<=3*max(4,2*n)。
/// 每个字段计一完整BTree节点；解码UTF-8不长于原字符串体；serde scratch最多下一2次幂块共存。
#[derive(Default, Debug)]
pub(crate) struct JsonShape {
    pub(crate) text: usize,
    pub(crate) values: usize,
    pub(crate) fields: usize,
    pub(crate) arrays: usize,
    pub(crate) depth: usize,
    longest: usize,
}
impl JsonShape {
    pub(crate) fn scan(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        let mut i = 0;
        let mut depth = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    let start = i + 1;
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'"' {
                        if bytes[i] == b'\\' {
                            i = i.saturating_add(1);
                        }
                        i = i.saturating_add(1);
                    }
                    let n = i.saturating_sub(start);
                    out.text = out.text.saturating_add(n);
                    out.longest = out.longest.max(n);
                    out.values = out.values.saturating_add(1);
                }
                b'{' | b'[' => {
                    out.values = out.values.saturating_add(1);
                    out.arrays += usize::from(bytes[i] == b'[');
                    depth += 1;
                    out.depth = out.depth.max(depth);
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                b':' => out.fields += 1,
                b',' | b' ' | b'\n' | b'\r' | b'\t' => {}
                _ => {
                    out.values += 1;
                    while i + 1 < bytes.len() && !b",]} \r\n\t:".contains(&bytes[i + 1]) {
                        i += 1;
                    }
                }
            }
            i = i.saturating_add(1);
        }
        out
    }
    #[cfg(feature = "s_capture_native_testonly")]
    pub(crate) fn of_value(value: &serde_json::Value) -> Self {
        fn visit(value: &serde_json::Value, depth: usize, out: &mut JsonShape) {
            out.values = out.values.saturating_add(1);
            out.depth = out.depth.max(depth);
            match value {
                serde_json::Value::String(text) => {
                    out.text = out.text.saturating_add(text.len());
                    out.longest = out.longest.max(text.len());
                }
                serde_json::Value::Array(items) => {
                    out.arrays = out.arrays.saturating_add(1);
                    for item in items {
                        visit(item, depth.saturating_add(1), out);
                    }
                }
                serde_json::Value::Object(fields) => {
                    out.fields = out.fields.saturating_add(fields.len());
                    for (key, value) in fields {
                        out.values = out.values.saturating_add(1);
                        out.text = out.text.saturating_add(key.len());
                        out.longest = out.longest.max(key.len());
                        visit(value, depth.saturating_add(1), out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Self::default();
        visit(value, 0, &mut out);
        out
    }
    pub(crate) fn parse_peak(&self) -> usize {
        sum([
            self.text,
            // 各空数组零分配；把4槽初次增长也按所有数组计入，比实际宽。
            vec_bytes::<serde_json::Value>(
                self.values
                    .saturating_mul(6)
                    .saturating_add(self.arrays.saturating_mul(12)),
            ),
            tree_slots::<String, serde_json::Value>(self.fields),
            self.longest
                .checked_next_power_of_two()
                .unwrap_or(usize::MAX)
                .saturating_mul(3),
        ])
    }
    /// canonical_json 的每层 keys/parts、join和format共存。6是单字节转义的最长UTF-8输出。
    #[cfg(feature = "s_capture_native_testonly")]
    pub(crate) fn canonical_peak(&self) -> usize {
        let output = sum([
            self.text.saturating_mul(6),
            self.values.saturating_mul(32),
            self.fields.saturating_mul(3),
        ]);
        sum([
            output
                .saturating_mul(self.depth.saturating_add(1))
                .saturating_mul(3),
            self.values
                .saturating_mul(size_of::<String>() + size_of::<&String>())
                .saturating_mul(2),
        ])
    }
}
