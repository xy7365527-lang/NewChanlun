//! #1394 Native 的独立上界账。这里从不写入 Owned requested Layout 账。
//! 许可先占完整新块，扩容返回后才释放旧块；临时许可由 Drop 归还。
use crate::v3::audit_retention::{hash_slots, sum, tree_slots, vec_bytes, Retained};
use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    hash::Hash,
    mem::size_of,
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BudgetExceeded;
#[derive(Debug)]
pub(crate) enum Failure {
    BudgetExceeded,
    Invalid(String),
}
impl From<BudgetExceeded> for Failure {
    fn from(_: BudgetExceeded) -> Self {
        Self::BudgetExceeded
    }
}
impl From<String> for Failure {
    fn from(e: String) -> Self {
        Self::Invalid(e)
    }
}
#[derive(Clone)]
pub(crate) struct Budget(Rc<Ledger>);
struct Ledger {
    limit: usize,
    live: Cell<usize>,
    peak: Cell<usize>,
}
pub(crate) struct Permit {
    budget: Budget,
    bytes: usize,
}
/// 字段按声明顺序析构：被保护的工作集先真正释放，许可随后归还。
/// 避免 `(value, permit)` 局部绑定逆序析构时，先放掉许可再释放 value。
pub(crate) struct Reserved<T> {
    value: T,
    _permit: Permit,
}
impl<T> std::ops::Deref for Reserved<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.budget.release(self.bytes);
    }
}
impl Permit {
    pub(crate) fn hold<T>(self, value: T) -> Reserved<T> {
        Reserved {
            value,
            _permit: self,
        }
    }
    /// 只在成功构造且真实旧载荷已释放后结算。actual 是约定的保守保留费，不是 Layout 实测。
    pub(crate) fn commit(mut self, old: usize, actual: usize) {
        assert!(actual <= self.bytes, "预留必须覆盖新保留费");
        self.budget.release(self.bytes - actual);
        self.budget.release(old);
        self.bytes = 0;
    }
}
impl Budget {
    pub(crate) fn new(limit: usize, retained: usize) -> Result<Self, BudgetExceeded> {
        let live = retained
            .checked_add(size_of::<Ledger>() + 2 * size_of::<usize>())
            .ok_or(BudgetExceeded)?;
        if live > limit {
            return Err(BudgetExceeded);
        }
        Ok(Self(Rc::new(Ledger {
            limit,
            live: Cell::new(live),
            peak: Cell::new(live),
        })))
    }
    pub(crate) fn reserve(&self, bytes: usize) -> Result<Permit, BudgetExceeded> {
        let live = self.live().checked_add(bytes).ok_or(BudgetExceeded)?;
        if live > self.0.limit {
            return Err(BudgetExceeded);
        }
        self.0.live.set(live);
        self.0.peak.set(self.peak().max(live));
        Ok(Permit {
            budget: self.clone(),
            bytes,
        })
    }
    pub(crate) fn live(&self) -> usize {
        self.0.live.get()
    }
    pub(crate) fn peak(&self) -> usize {
        self.0.peak.get()
    }
    pub(crate) fn release(&self, bytes: usize) {
        self.0.live.set(
            self.live()
                .checked_sub(bytes)
                .expect("Native 归属账不得释放未持有费用"),
        );
    }
    pub(crate) fn string(&self, value: &str) -> Result<String, BudgetExceeded> {
        let permit = self.reserve(value.len())?;
        let mut out = String::new();
        out.try_reserve_exact(value.len())
            .map_err(|_| BudgetExceeded)?;
        out.push_str(value);
        permit.commit(0, out.capacity());
        Ok(out)
    }
    pub(crate) fn vec_grow<T>(
        &self,
        v: &mut Vec<T>,
        additional: usize,
    ) -> Result<(), BudgetExceeded> {
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        if needed <= v.capacity() {
            return Ok(());
        }
        let next = needed
            .max(v.capacity().checked_mul(2).ok_or(BudgetExceeded)?)
            .max(4);
        let permit = self.reserve(vec_bytes::<T>(next))?;
        let old = vec_bytes::<T>(v.capacity());
        v.try_reserve_exact(next - v.len())
            .map_err(|_| BudgetExceeded)?;
        permit.commit(old, vec_bytes::<T>(v.capacity()));
        Ok(())
    }
    pub(crate) fn codec_vec_grow<T>(
        &self,
        v: &mut newchan_rust::s_content::CodecVec<T>,
        additional: usize,
    ) -> Result<(), Failure> {
        if v.allocation_budget().is_some() {
            return Err(Failure::Invalid("Native预留不接Owned容器".into()));
        }
        let needed = v.len().checked_add(additional).ok_or(BudgetExceeded)?;
        if needed <= v.capacity() {
            return Ok(());
        }
        let next = needed
            .max(v.capacity().checked_mul(2).ok_or(BudgetExceeded)?)
            .max(4);
        let permit = self.reserve(vec_bytes::<T>(next))?;
        let old = vec_bytes::<T>(v.capacity());
        v.try_reserve_exact(next - v.len())
            .map_err(|_| BudgetExceeded)?;
        permit.commit(old, vec_bytes::<T>(v.capacity()));
        Ok(())
    }
    pub(crate) fn hash_room<K: Eq + Hash, V>(
        &self,
        m: &mut HashMap<K, V>,
        key: &K,
    ) -> Result<(), BudgetExceeded> {
        if m.contains_key(key) || m.len() < m.capacity() {
            return Ok(());
        }
        let cap = hash_capacity(m.len().checked_add(1).ok_or(BudgetExceeded)?)?;
        let permit = self.reserve(hash_slots::<K, V>(cap))?;
        let old = hash_slots::<K, V>(m.capacity());
        m.try_reserve(1).map_err(|_| BudgetExceeded)?;
        permit.commit(old, hash_slots::<K, V>(m.capacity()));
        Ok(())
    }
    pub(crate) fn tree_insert<K: Ord + Retained, V: Retained>(
        &self,
        m: &mut BTreeMap<K, V>,
        k: K,
        v: V,
    ) -> Result<(), BudgetExceeded> {
        // k/v 的独立载荷已由调用点持账；这里只管理节点与真正被覆盖的旧载荷。
        if let Some(old) = m.get_mut(&k) {
            let released = sum([k.heap_bytes(), old.heap_bytes()]);
            *old = v;
            drop(k);
            self.release(released);
            return Ok(());
        }
        let permit = self.reserve(tree_growth::<K, V>(m.len()))?;
        m.insert(k, v);
        permit.commit(0, tree_slots::<K, V>(1));
        Ok(())
    }
}
/// 固定 std SwissTable：小表 4/8 桶只空一槽，>=16 桶的装载率 7/8；控制组最多16字节。
/// 返回下一次实际可用 capacity 的上界，再沿既有 hash_slots 公式结算。
pub(crate) fn hash_capacity(needed: usize) -> Result<usize, BudgetExceeded> {
    if needed == 0 {
        return Ok(0);
    }
    if needed <= 3 {
        return Ok(3);
    }
    if needed <= 7 {
        return Ok(7);
    }
    let buckets = needed
        .checked_mul(8)
        .and_then(|n| n.checked_add(6))
        .map(|n| n / 7)
        .and_then(usize::checked_next_power_of_two)
        .ok_or(BudgetExceeded)?;
    Ok(buckets / 8 * 7)
}
/// std BTree B=6；每层至多一次 split 和一个新根。最小分支数>=2，故 log2(n)+2 覆盖节点数。
/// tree_slots 保留公式仍每项一个完整节点；额外 split 仅在许可活期计入峰值。
pub(crate) fn tree_growth<K, V>(len: usize) -> usize {
    let height = if len == 0 {
        0
    } else {
        usize::BITS as usize - len.leading_zeros() as usize
    };
    tree_slots::<K, V>(height.saturating_add(2))
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_permits_release_and_growth_keeps_old_block_until_reserve() {
        let b = Budget::new(4096, 0).unwrap();
        let base = b.live();
        let p = b.reserve(1000).unwrap();
        assert_eq!(b.live(), base + 1000);
        assert!(b.reserve(4096).is_err());
        drop(p);
        assert_eq!(b.live(), base);
        let mut v = Vec::<u64>::new();
        b.vec_grow(&mut v, 4).unwrap();
        v.extend([1, 2, 3, 4]);
        let old = v.capacity() * size_of::<u64>();
        let before = b.live();
        b.vec_grow(&mut v, 1).unwrap();
        assert_eq!(b.live(), base + v.capacity() * size_of::<u64>());
        assert!(b.peak() >= before + v.capacity() * size_of::<u64>());
        let mut blocked = vec![1u64, 2, 3, 4];
        let old_block = vec_bytes::<u64>(blocked.capacity());
        let tight = Budget::new(base + old_block + vec_bytes::<u64>(8) - 1, old_block).unwrap();
        let capacity = blocked.capacity();
        assert!(tight.vec_grow(&mut blocked, 1).is_err());
        assert_eq!(blocked.capacity(), capacity);
        assert_eq!(&blocked[..], &[1, 2, 3, 4], "新块被拒绝时旧Vec必须完整保留");
        assert_eq!(&v[..], &[1, 2, 3, 4]);
        assert!(v.capacity() * size_of::<u64>() > old);
        assert!(matches!(
            Budget::new(usize::MAX, usize::MAX),
            Err(BudgetExceeded)
        ));
        assert!(b.reserve(usize::MAX).is_err());
        struct BeforeRelease(Budget, usize);
        impl Drop for BeforeRelease {
            fn drop(&mut self) {
                assert_eq!(self.0.live(), self.1, "实际工作集析构前许可仍须占账");
            }
        }
        let live = b.live();
        let permit = b.reserve(100).unwrap();
        let held = permit.hold(BeforeRelease(b.clone(), live + 100));
        drop(held);
        assert_eq!(b.live(), live);
    }
    #[test]
    fn native_hash_bucket_bound_covers_actual_growth_and_json_shapes_cover_heap() {
        let mut map = HashMap::<String, u64>::new();
        for i in 0..1024 {
            let need = map.len() + 1;
            if need > map.capacity() {
                let upper = hash_capacity(need).unwrap();
                map.try_reserve(1).unwrap();
                assert!(
                    map.capacity() <= upper,
                    "{need}: {} > {upper}",
                    map.capacity()
                );
            }
            map.insert(i.to_string(), i);
        }
        for bytes in [
            br#"{"a":[1,2,3,4,5,6,7,8,9],"b":{"x":"\u4e2d\u6587\n\uD83D\uDE00"}}"#.as_slice(),
            br#"[[],{},[[],[true,false,null]],"abcdefghijklmnopqrstuvwxyz"]"#.as_slice(),
        ] {
            let shape = JsonShape::scan(bytes);
            let value = newchan_rust::session_protocol::strict_json(bytes).unwrap();
            assert!(value.heap_bytes() <= shape.parse_peak(), "{shape:?}");
            let canonical = newchan_rust::session_protocol::canonical_json(&value);
            assert!(canonical.capacity() <= shape.canonical_peak());
        }
    }
}
