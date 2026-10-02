//! #1394 业务原始账的入口存储选择与同一借用判据。Owned失败直接Err，不转回std。
use super::*;
use aj::{ArrayIter, ArrayView, JsonView};
use newchan_rust::s_content as sc;
use newchan_rust::sqlite_checked::{CheckedRow, CheckedStatement};
use sc::capture_allocation::{audit_json as aj, OwnedBudget};
use sha2::{Digest, Sha256};
use std::time::Instant;

pub(crate) enum RawEvents {
    Plain(Vec<Value>),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(aj::OwnedJson),
}
impl Default for RawEvents {
    fn default() -> Self {
        Self::Plain(Vec::new())
    }
}
impl RawEvents {
    pub fn view(&self) -> RawEventsView<'_> {
        match self {
            Self::Plain(v) => RawEventsView::plain(v),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => RawEventsView {
                storage: RawStorage::Arena(v.view().as_array().unwrap()),
                len: v.view().as_array().unwrap().len(),
                budget: Some(v.budget()),
            },
        }
    }
    pub fn budget(&self) -> Option<&OwnedBudget> {
        self.view().budget()
    }
    pub fn len(&self) -> usize {
        self.view().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn iter(&self) -> RawIter<'_> {
        self.view().iter()
    }
    pub fn plain(&self) -> Result<&[Value], String> {
        match self {
            Self::Plain(v) => Ok(v),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(_) => {
                Err("prototype_audit_allocation_incomplete：owned原始账不可擦除为std值".into())
            }
        }
    }
}
impl serde::Serialize for RawEvents {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        let mut a = s.serialize_seq(Some(self.len()))?;
        for v in self.iter() {
            a.serialize_element(&v)?;
        }
        a.end()
    }
}
impl std::fmt::Debug for RawEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
#[derive(Clone, Copy)]
enum RawStorage<'a> {
    Plain(&'a [Value]),
    Arena(ArrayView<'a>),
    Borrowed(&'a [JsonView<'a>]),
}
#[derive(Clone, Copy)]
pub(crate) struct RawEventsView<'a> {
    storage: RawStorage<'a>,
    len: usize,
    budget: Option<&'a OwnedBudget>,
}
impl<'a> RawEventsView<'a> {
    pub fn plain(v: &'a [Value]) -> Self {
        Self {
            storage: RawStorage::Plain(v),
            len: v.len(),
            budget: None,
        }
    }
    pub fn len(self) -> usize {
        self.len
    }
    pub fn is_empty(self) -> bool {
        self.len == 0
    }
    pub fn prefix(self, n: usize) -> Result<Self, String> {
        if n > self.len {
            return Err("StorageUnavailable：原始账前缀越界".into());
        }
        Ok(Self { len: n, ..self })
    }
    pub fn get(self, i: usize) -> Option<JsonView<'a>> {
        if i >= self.len {
            return None;
        }
        match self.storage {
            RawStorage::Plain(v) => Some((&v[i]).into()),
            RawStorage::Arena(v) => v.get(i),
            RawStorage::Borrowed(v) => v.get(i).copied(),
        }
    }
    pub fn iter(self) -> RawIter<'a> {
        RawIter {
            storage: match self.storage {
                RawStorage::Plain(v) => RawIterStorage::Plain(v.iter()),
                RawStorage::Arena(v) => RawIterStorage::Arena(v.iter()),
                RawStorage::Borrowed(v) => RawIterStorage::Borrowed(v.iter()),
            },
            left: self.len,
        }
    }
    pub fn budget(self) -> Option<&'a OwnedBudget> {
        self.budget
    }
}
impl<'a> From<&'a [Value]> for RawEventsView<'a> {
    fn from(v: &'a [Value]) -> Self {
        Self::plain(v)
    }
}
impl<'a> From<&'a Vec<Value>> for RawEventsView<'a> {
    fn from(v: &'a Vec<Value>) -> Self {
        Self::plain(v)
    }
}
impl<'a> From<&'a RawEvents> for RawEventsView<'a> {
    fn from(v: &'a RawEvents) -> Self {
        v.view()
    }
}
impl<'a> IntoIterator for RawEventsView<'a> {
    type Item = JsonView<'a>;
    type IntoIter = RawIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
enum RawIterStorage<'a> {
    Plain(std::slice::Iter<'a, Value>),
    Arena(ArrayIter<'a>),
    Borrowed(std::slice::Iter<'a, JsonView<'a>>),
}
pub(crate) struct RawIter<'a> {
    storage: RawIterStorage<'a>,
    left: usize,
}
impl<'a> Iterator for RawIter<'a> {
    type Item = JsonView<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        match &mut self.storage {
            RawIterStorage::Plain(i) => i.next().map(JsonView::Plain),
            RawIterStorage::Arena(i) => i.next(),
            RawIterStorage::Borrowed(i) => i.next().copied(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}
impl ExactSizeIterator for RawIter<'_> {}
pub(crate) fn field<'a>(v: JsonView<'a>, k: &str) -> JsonView<'a> {
    v.get(k).unwrap_or(JsonView::Null)
}
pub(crate) fn text<'a>(v: JsonView<'a>, k: &str) -> Result<&'a str, String> {
    field(v, k)
        .as_str()
        .ok_or_else(|| format!("StorageUnavailable：{k}非文本"))
}
pub(crate) fn coord(v: JsonView<'_>) -> Result<i64, String> {
    crate::parse_source_coord(text(v, "source_coord")?)
        .map_err(|e| format!("StorageUnavailable：{e}"))
}
pub(crate) fn new_work<T>(b: Option<&OwnedBudget>) -> sc::CodecVec<T> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = b {
        return aj::owned_vec(b);
    }
    let _ = b;
    sc::CodecVec::from_plain(Vec::new())
}
pub(crate) use aj::{BusinessIter, BusinessMap};
pub(crate) fn new_map<K: Eq + std::hash::Hash, V>(b: Option<&OwnedBudget>) -> BusinessMap<K, V> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = b {
        return aj::BusinessMap::owned(b);
    }
    let _ = b;
    aj::BusinessMap::plain()
}
fn workmap<K: Eq + std::hash::Hash, V>(b: Option<&OwnedBudget>) -> BusinessMap<K, V> {
    new_map(b)
}
pub(crate) fn bytes_extend(bytes: &mut sc::CodecVec<u8>, value: &[u8]) -> Result<(), String> {
    bytes.try_extend_bytes(value).map_err(|e| e.to_string())
}
pub(crate) struct Effective<'a> {
    values: sc::CodecVec<JsonView<'a>>,
    budget: Option<&'a OwnedBudget>,
}
impl<'a> Effective<'a> {
    pub fn raw_view(&self) -> RawEventsView<'_> {
        RawEventsView {
            storage: RawStorage::Borrowed(&self.values),
            len: self.values.len(),
            budget: self.budget,
        }
    }
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = JsonView<'a>> + '_ {
        self.values.iter().copied()
    }
    pub fn view(&self) -> JsonView<'_> {
        JsonView::ArraySlice(&self.values)
    }
}
pub(crate) fn effective<'a>(
    all: RawEventsView<'a>,
    deadline: Option<Instant>,
) -> Result<Effective<'a>, String> {
    check_read_deadline(deadline)?;
    // Same max-revision/first-on-tie decision, ordered in O(n log n); no payload/key clones.
    let mut candidates = new_work::<(&str, i64, usize, JsonView<'a>)>(all.budget());
    for (ordinal, ev) in all.iter().enumerate() {
        check_read_deadline(deadline)?;
        candidates
            .try_push((
                field(ev, "identity_key").as_str().unwrap_or(""),
                field(ev, "revision").as_i64().unwrap_or(0),
                ordinal,
                ev,
            ))
            .map_err(|e| e.to_string())?;
    }
    candidates.sort_unstable_by(|a, b| {
        a.0.cmp(b.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut ordered = new_work::<(i64, JsonView<'a>)>(all.budget());
    let mut last = None;
    for (identity, _, _, v) in candidates {
        check_read_deadline(deadline)?;
        if last == Some(identity) {
            continue;
        }
        last = Some(identity);
        ordered
            .try_push((coord(v)?, v))
            .map_err(|e| e.to_string())?;
    }
    ordered.sort_unstable_by_key(|(c, _)| *c);
    if ordered.windows(2).any(|v| v[0].0 == v[1].0) {
        return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖业务身份".into());
    }
    let mut values = new_work(all.budget());
    for (_, v) in ordered {
        values.try_push(v).map_err(|e| e.to_string())?;
    }
    Ok(Effective {
        values,
        budget: all.budget(),
    })
}

fn hex(bytes: &[u8; 32]) -> [u8; 64] {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut out = [0; 64];
    for (i, v) in bytes.iter().enumerate() {
        out[2 * i] = H[(v >> 4) as usize];
        out[2 * i + 1] = H[(v & 15) as usize];
    }
    out
}
pub(crate) fn sha_json(v: JsonView<'_>, b: Option<&OwnedBudget>) -> Result<[u8; 32], String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = b {
        let out = aj::canonical_bytes(v, b).map_err(|e| e.to_string())?;
        return Ok(Sha256::digest(&*out).into());
    }
    let _ = b;
    let out = serde_json::to_vec(&v).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(&out).into())
}
pub(crate) fn validate_raw(
    rows: RawEventsView<'_>,
    deadline: Option<Instant>,
) -> Result<(), String> {
    let mut owners = workmap::<i64, &str>(rows.budget());
    let mut positions = workmap::<&str, i64>(rows.budget());
    let mut prior = workmap::<&str, i64>(rows.budget());
    for (expected_seq, event) in rows.iter().enumerate() {
        check_read_deadline(deadline)?;
        let c = coord(event)?;
        let identity = text(event, "identity_key")?;
        if owners
            .insert(c, identity)
            .map_err(|e| e.to_string())?
            .is_some_and(|old| old != identity)
            || positions
                .insert(identity, c)
                .map_err(|e| e.to_string())?
                .is_some_and(|old| old != c)
        {
            return Err("StorageUnavailable：原始源坐标与业务身份的唯一归属被破坏".into());
        }
        let price = text(event, "price")?;
        let volume = text(event, "volume")?;
        let ohlc = crate::tb02a::stored_prices_view(event)?;
        crate::tb02a::validate_prices_view(price, volume, ohlc)
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let input_rev = text(event, "input_revision")?;
        let revision = parse_canonical_i64(input_rev, "input_revision")
            .map_err(|e| format!("StorageUnavailable：{e}"))?;
        let previous = prior
            .insert(identity, revision)
            .map_err(|e| e.to_string())?;
        let mut fields = [("", JsonView::Null); 12];
        let mut used = 0;
        for (key, raw_key) in [
            ("event_id", "event_id"),
            ("revision", "input_revision"),
            ("seq", "source_coord"),
            ("received_at", "received_at"),
            ("raw_text", "raw_text"),
            ("timestamp", "ts"),
            ("volume", "volume"),
        ] {
            fields[used] = (key, field(event, raw_key));
            used += 1;
        }
        if let Some(p) = ohlc {
            for (k, v) in [
                ("open", p.open),
                ("high", p.high),
                ("low", p.low),
                ("close", p.close),
            ] {
                fields[used] = (k, JsonView::String(v));
                used += 1;
            }
        } else {
            fields[used] = ("price", JsonView::String(price));
            used += 1;
        }
        let content = JsonView::object_fields(&fields[..used]).map_err(|e| e.to_string())?;
        let payload = hex(&sha_json(content, rows.budget())?);
        let payload = std::str::from_utf8(&payload).unwrap();
        let mut h = Sha256::new();
        h.update(identity.as_bytes());
        h.update(b"|");
        h.update(payload.as_bytes());
        let receipt_hex = hex(&h.finalize().into());
        let mut receipt = [0; 21];
        receipt[..5].copy_from_slice(b"rcpt-");
        receipt[5..].copy_from_slice(&receipt_hex[..16]);
        let receipt = std::str::from_utf8(&receipt).unwrap();
        let ns = text(event, "source_namespace")?;
        let epoch = text(event, "source_epoch")?;
        let instrument = text(event, "instrument")?;
        let event_id = text(event, "event_id")?;
        let expected_id = ns
            .bytes()
            .chain([b'|'])
            .chain(epoch.bytes())
            .chain([b'|'])
            .chain(instrument.bytes())
            .chain([b'|'])
            .chain(event_id.bytes());
        let supersedes = field(event, "supersedes_revision");
        let sup_matches = match previous {
            None => supersedes.is_null(),
            Some(v) => supersedes.as_i64() == Some(v),
        };
        if field(event, "seq").as_i64() != i64::try_from(expected_seq).ok()
            || revision < 1
            || field(event, "revision").as_i64() != Some(revision)
            || previous.is_some_and(|p| p >= revision)
            || !sup_matches
            || !identity.bytes().eq(expected_id)
            || text(event, "payload_hash")? != payload
            || text(event, "receipt_id")? != receipt
        {
            return Err("StorageUnavailable：原始身份/接纳序/修订链/内容hash/receipt矛盾".into());
        }
        for (name, value) in [
            ("price", price),
            ("timestamp", text(event, "ts")?),
            ("volume", volume),
        ] {
            parse_canonical_i64(value, name).map_err(|e| format!("StorageUnavailable：{e}"))?;
        }
    }
    check_read_deadline(deadline)
}
#[cfg(feature = "s_allocation_json_testonly")]
pub(crate) fn read_owned(
    conn: &Connection,
    deadline: Option<Instant>,
    b: &OwnedBudget,
) -> Result<RawEvents, String> {
    check_read_deadline(deadline)?;
    let mut tree = aj::OwnedJson::array(b).map_err(|e| e.to_string())?;
    let mut stmt=CheckedStatement::prepare(conn,c"SELECT identity_key, revision, input_revision, payload_hash, receipt_id, seq, source_namespace,source_epoch, instrument, event_id, received_at, raw_text, price,ts, volume, source_coord, supersedes_revision FROM raw_events ORDER BY seq ASC").map_err(|e|format!("prepare 读取失败：{e}"))?;
    while let Some(mut row) = stmt.next().map_err(|e| format!("collect 失败：{e}"))? {
        check_read_deadline(deadline)?;
        tree.append_object(|o| {
            for (i, k) in [
                (0, "identity_key"),
                (2, "input_revision"),
                (3, "payload_hash"),
                (4, "receipt_id"),
                (6, "source_namespace"),
                (7, "source_epoch"),
                (8, "instrument"),
                (9, "event_id"),
                (10, "received_at"),
                (11, "raw_text"),
                (12, "price"),
                (13, "ts"),
                (14, "volume"),
                (15, "source_coord"),
            ] {
                let value = row.text(i).map_err(|e| {
                    sc::CodecError::new("StorageUnavailable", format!("SQL raw文本列读取失败：{e}"))
                })?;
                o.insert_str(k, value)?;
            }
            for (i, k) in [(1, "revision"), (5, "seq")] {
                o.insert_i64(
                    k,
                    row.integer(i).map_err(|e| {
                        sc::CodecError::new(
                            "StorageUnavailable",
                            format!("SQL raw整数列读取失败：{e}"),
                        )
                    })?,
                )?;
            }
            match row.optional_integer(16).map_err(|e| {
                sc::CodecError::new(
                    "StorageUnavailable",
                    format!("SQL raw supersedes列读取失败：{e}"),
                )
            })? {
                Some(v) => o.insert_i64("supersedes_revision", v)?,
                None => o.insert_null("supersedes_revision")?,
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;
    }
    stmt.finalize()
        .map_err(|e| format!("finalize 读取失败：{e}"))?;
    crate::tb02a::enrich_raw_owned(conn, &mut tree, deadline)?;
    let result = RawEvents::Owned(tree);
    validate_raw(result.view(), deadline)?;
    Ok(result)
}

pub(crate) struct Decimal {
    bytes: [u8; 20],
    len: usize,
}
impl std::fmt::Write for Decimal {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
impl Decimal {
    pub(crate) fn new(v: i64) -> Self {
        use std::fmt::Write;
        let mut out = Self {
            bytes: [0; 20],
            len: 0,
        };
        write!(&mut out, "{v}").expect("i64 at most20 ASCII bytes");
        out
    }
    pub(crate) fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
}
/// 撤回 wire 的唯一投影核；typed 保留完整原件，shape 只将三个整数变成规范文本。
/// 借用的子值与栈内十进制文本不制造临时 JSON 树；数组工作集仍由调用者实际账本持有。
pub(crate) struct WithdrawalWire<'a> {
    original: JsonView<'a>,
    windows: Option<[Decimal; 3]>,
}
impl<'a> WithdrawalWire<'a> {
    pub(crate) fn new(original: JsonView<'a>) -> Result<Self, String> {
        let windows = if original.get("fact_key").is_some() {
            None
        } else {
            let number = |key| {
                field(original, key)
                    .as_i64()
                    .map(Decimal::new)
                    .ok_or_else(|| "持久整数必须是 i64，不能是浮点、布尔、空值或文本".to_owned())
            };
            Some([
                number("window_start")?,
                number("window_mid")?,
                number("window_end")?,
            ])
        };
        Ok(Self { original, windows })
    }
    pub(crate) fn view(&self) -> JsonView<'_> {
        if self.windows.is_none() {
            return self.original;
        }
        JsonView::borrowed_object(self).expect("撤回 wire 六个固定字段唯一")
    }
}
impl aj::JsonObject for WithdrawalWire<'_> {
    fn len(&self) -> usize {
        6
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        let windows = self.windows.as_ref()?;
        Some(match index {
            0 => ("object_id", field(self.original, "object_id")),
            1 => ("reason", field(self.original, "reason")),
            2 => ("superseded_by", field(self.original, "superseded_by")),
            3 => ("window_end", JsonView::String(windows[2].text())),
            4 => ("window_mid", JsonView::String(windows[1].text())),
            5 => ("window_start", JsonView::String(windows[0].text())),
            _ => return None,
        })
    }
}
pub(crate) struct WithdrawalArray<'a>(pub sc::CodecVec<WithdrawalWire<'a>>);
impl WithdrawalArray<'_> {
    pub(crate) fn view(&self) -> JsonView<'_> {
        JsonView::BorrowedArray(self)
    }
}
impl aj::JsonArray for WithdrawalArray<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(WithdrawalWire::view)
    }
}
/// 源引用wire/numeric共用字段投影，只让revision/seq的值出现差异。
pub(crate) struct RawReference<'a> {
    event: JsonView<'a>,
    revision: Decimal,
    seq: Decimal,
    numeric: bool,
}
impl<'a> RawReference<'a> {
    pub fn new(event: JsonView<'a>, numeric: bool) -> Result<Self, String> {
        Ok(Self {
            event,
            revision: Decimal::new(
                field(event, "revision")
                    .as_i64()
                    .ok_or("StorageUnavailable：revision非i64")?,
            ),
            seq: Decimal::new(
                field(event, "seq")
                    .as_i64()
                    .ok_or("StorageUnavailable：seq非i64")?,
            ),
            numeric,
        })
    }
    pub fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("fixed unique raw-reference keys")
    }
}
impl aj::JsonObject for RawReference<'_> {
    fn len(&self) -> usize {
        8
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        let k = *[
            "identity_key",
            "payload_hash",
            "receipt_id",
            "event_id",
            "input_revision",
            "revision",
            "seq",
            "source_coord",
        ]
        .get(index)?;
        let v = match k {
            "revision" if !self.numeric => JsonView::String(self.revision.text()),
            "seq" if !self.numeric => JsonView::String(self.seq.text()),
            _ => field(self.event, k),
        };
        Some((k, v))
    }
}
pub(crate) struct RawWire<'a> {
    event: JsonView<'a>,
    revision: Decimal,
    seq: Decimal,
    supersedes: Option<Decimal>,
    ohlc: bool,
}
impl<'a> RawWire<'a> {
    pub fn new(event: JsonView<'a>) -> Result<Self, String> {
        Ok(Self {
            event,
            revision: Decimal::new(
                field(event, "revision")
                    .as_i64()
                    .ok_or("StorageUnavailable：revision非i64")?,
            ),
            seq: Decimal::new(
                field(event, "seq")
                    .as_i64()
                    .ok_or("StorageUnavailable：seq非i64")?,
            ),
            supersedes: field(event, "supersedes_revision")
                .as_i64()
                .map(Decimal::new),
            ohlc: field(event, "schema_revision").as_str() == Some(tb02a::RAW_SCHEMA),
        })
    }
    pub fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("fixed unique raw-wire keys")
    }
}
impl aj::JsonObject for RawWire<'_> {
    fn len(&self) -> usize {
        if self.ohlc {
            21
        } else {
            17
        }
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        const BASE: [&str; 17] = [
            "identity_key",
            "revision",
            "input_revision",
            "payload_hash",
            "receipt_id",
            "seq",
            "source_namespace",
            "source_epoch",
            "instrument",
            "event_id",
            "received_at",
            "raw_text",
            "price",
            "ts",
            "volume",
            "source_coord",
            "supersedes_revision",
        ];
        let k = if self.ohlc {
            let mut position = 0;
            let mut found = None;
            for name in BASE.iter().copied().filter(|k| *k != "price").chain([
                "schema_revision",
                "open",
                "high",
                "low",
                "close",
            ]) {
                if position == index {
                    found = Some(name);
                    break;
                }
                position += 1;
            }
            found?
        } else {
            *BASE.get(index)?
        };
        let v = match k {
            "revision" => JsonView::String(self.revision.text()),
            "seq" => JsonView::String(self.seq.text()),
            "supersedes_revision" => self
                .supersedes
                .as_ref()
                .map(|v| JsonView::String(v.text()))
                .unwrap_or(JsonView::Null),
            _ => field(self.event, k),
        };
        Some((k, v))
    }
}
pub(crate) struct ReferenceArray<'a>(pub sc::CodecVec<RawReference<'a>>);
impl aj::JsonArray for ReferenceArray<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(RawReference::view)
    }
}
pub(crate) struct ValuesArray<'a>(pub &'a [Value]);
impl aj::JsonArray for ValuesArray<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(JsonView::Plain)
    }
}

impl aj::JsonArray for RawEventsView<'_> {
    fn len(&self) -> usize {
        self.len
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        (*self).get(index)
    }
}

impl serde::Serialize for RawReference<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.view().serialize(s)
    }
}

impl<'a, const N: usize> From<&'a [Value; N]> for RawEventsView<'a> {
    fn from(v: &'a [Value; N]) -> Self {
        Self::plain(v)
    }
}

/// 新业务返回文本的入口存储选择。Owned实际字节持续持账；Clone只开放fallible深复制。
pub(crate) enum TextValue {
    Plain(String),
    Owned(aj::TextOwner),
}
impl TextValue {
    pub fn from_owned_bytes(bytes: sc::CodecVec<u8>) -> Result<Self, String> {
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            return aj::TextOwner::from_owned_bytes(bytes)
                .map(Self::Owned)
                .map_err(|e| e.to_string());
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = bytes;
            Err("prototype_audit_allocation_incomplete：Owned文本入口未启用".into())
        }
    }
    /// 存储选择来自传入真实Vec owner；Owned错误直接返回，不回普通分支。
    pub fn from_bytes(bytes: sc::CodecVec<u8>) -> Result<Self, String> {
        if bytes.allocation_budget().is_some() {
            return Self::from_owned_bytes(bytes);
        }
        let bytes = bytes.try_into_plain().map_err(|e| e.to_string())?;
        String::from_utf8(bytes)
            .map(Self::Plain)
            .map_err(|e| e.to_string())
    }
    pub fn as_str(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Owned(s) => s.as_str(),
        }
    }
    pub fn capacity(&self) -> usize {
        match self {
            Self::Plain(s) => s.capacity(),
            Self::Owned(s) => s.capacity(),
        }
    }
    pub fn try_clone(&self) -> Result<Self, String> {
        match self {
            Self::Plain(s) => Ok(Self::Plain(s.clone())),
            Self::Owned(s) => {
                #[cfg(feature = "s_allocation_json_testonly")]
                {
                    Ok(Self::Owned(s.try_clone().map_err(|e| e.to_string())?))
                }
                #[cfg(not(feature = "s_allocation_json_testonly"))]
                {
                    let _ = s;
                    Err("prototype_audit_allocation_incomplete：owned text clone未启用".into())
                }
            }
        }
    }
    pub fn from_str(s: &str, b: Option<&OwnedBudget>) -> Result<Self, String> {
        #[cfg(feature = "s_allocation_json_testonly")]
        if let Some(b) = b {
            return Ok(Self::Owned(
                aj::TextOwner::copy_from(s, b).map_err(|e| e.to_string())?,
            ));
        }
        let _ = b;
        Ok(Self::Plain(s.to_owned()))
    }
    pub fn from_sql(
        row: &mut CheckedRow<'_>,
        i: usize,
        b: Option<&OwnedBudget>,
    ) -> Result<Self, String> {
        let value = row.text(i).map_err(|e| e.to_string())?;
        Self::from_str(value, b)
    }
    pub fn optional_sql(
        row: &mut CheckedRow<'_>,
        i: usize,
        b: Option<&OwnedBudget>,
    ) -> Result<Option<Self>, String> {
        row.optional_text(i)
            .map_err(|e| e.to_string())?
            .map(|value| Self::from_str(value, b))
            .transpose()
    }
}
impl std::borrow::Borrow<str> for TextValue {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl std::ops::Deref for TextValue {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Debug for TextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl std::fmt::Display for TextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl serde::Serialize for TextValue {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl PartialEq for TextValue {
    fn eq(&self, o: &Self) -> bool {
        self.as_str() == o.as_str()
    }
}
impl Eq for TextValue {}
impl PartialEq<str> for TextValue {
    fn eq(&self, o: &str) -> bool {
        self.as_str() == o
    }
}
impl PartialEq<&str> for TextValue {
    fn eq(&self, o: &&str) -> bool {
        self.as_str() == *o
    }
}
impl PartialEq<String> for TextValue {
    fn eq(&self, o: &String) -> bool {
        self.as_str() == o
    }
}
impl PartialOrd for TextValue {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for TextValue {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(o.as_str())
    }
}
impl std::hash::Hash for TextValue {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}
impl From<String> for TextValue {
    fn from(s: String) -> Self {
        Self::Plain(s)
    }
}
impl From<&str> for TextValue {
    fn from(s: &str) -> Self {
        Self::Plain(s.to_owned())
    }
}

pub(crate) enum JsonOwner {
    Plain(Value),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(aj::OwnedJson),
}
impl JsonOwner {
    pub fn view(&self) -> JsonView<'_> {
        match self {
            Self::Plain(v) => v.into(),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => v.view(),
        }
    }
}
/// 同一读取/渲染/严格解码核；预算随实际store选择，拒绝后没有Plain分支。
pub(crate) fn render_json(
    s: &mut (impl super::audit_store::CodecRead + ?Sized),
    r: sc::RefView<'_>,
) -> Result<JsonOwner, String> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(b) = s.allocation_budget() {
        struct Out {
            bytes: sc::CodecVec<u8>,
            error: Option<sc::CodecError>,
        }
        impl std::io::Write for Out {
            fn write(&mut self, v: &[u8]) -> std::io::Result<usize> {
                match aj::extend_bytes(&mut self.bytes, v) {
                    Ok(()) => Ok(v.len()),
                    Err(e) => {
                        self.error = Some(e);
                        Err(std::io::ErrorKind::Other.into())
                    }
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut out = Out {
            bytes: aj::owned_vec(&b),
            error: None,
        };
        let result = s.render_ref(r, &mut out);
        if let Some(e) = out.error {
            return Err(e.to_string());
        }
        result.map_err(|e| e.to_string())?;
        return aj::OwnedJson::parse(&out.bytes, &b)
            .map(JsonOwner::Owned)
            .map_err(|e| e.to_string());
    }
    let mut out = Vec::new();
    s.render_ref(r, &mut out).map_err(|e| e.to_string())?;
    newchan_rust::session_protocol::strict_json(&out).map(JsonOwner::Plain)
}
pub(crate) fn read_text(
    s: &mut (impl super::audit_store::CodecRead + ?Sized),
    r: sc::RefView<'_>,
) -> Result<TextValue, String> {
    let value = render_json(s, r)?;
    let text = value
        .view()
        .as_str()
        .ok_or("StorageUnavailable：预期文本")?;
    TextValue::from_str(text, s.allocation_budget().as_ref())
}
impl PartialEq<TextValue> for String {
    fn eq(&self, o: &TextValue) -> bool {
        self.as_str() == o.as_str()
    }
}
impl PartialEq<TextValue> for str {
    fn eq(&self, o: &TextValue) -> bool {
        self == o.as_str()
    }
}

impl rusqlite::ToSql for TextValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Borrowed(
            rusqlite::types::ValueRef::Text(self.as_str().as_bytes()),
        ))
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod collection_owner_tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    struct PhaseQuery<'a>(i64, &'a str);
    impl Hash for PhaseQuery<'_> {
        fn hash<H: Hasher>(&self, h: &mut H) {
            (self.0, self.1).hash(h);
        }
    }
    impl hashbrown::Equivalent<(i64, TextValue)> for PhaseQuery<'_> {
        fn equivalent(&self, key: &(i64, TextValue)) -> bool {
            self.0 == key.0 && self.1 == key.1.as_str()
        }
    }
    struct MessageQuery<'a>(&'a str, &'a str, i64);
    impl Hash for MessageQuery<'_> {
        fn hash<H: Hasher>(&self, h: &mut H) {
            (self.0, self.1, self.2).hash(h);
        }
    }
    impl hashbrown::Equivalent<(TextValue, TextValue, i64)> for MessageQuery<'_> {
        fn equivalent(&self, key: &(TextValue, TextValue, i64)) -> bool {
            self.0 == key.0.as_str() && self.1 == key.1.as_str() && self.2 == key.2
        }
    }
    // 强制同hash，只检验既有hashbrown删除后的探测链；不另写桶或删除算法。
    #[derive(Eq, PartialEq)]
    struct Collision(TextValue);
    impl Hash for Collision {
        fn hash<H: Hasher>(&self, h: &mut H) {
            0u8.hash(h);
        }
    }
    struct CollisionQuery<'a>(&'a str);
    impl Hash for CollisionQuery<'_> {
        fn hash<H: Hasher>(&self, h: &mut H) {
            0u8.hash(h);
        }
    }
    impl hashbrown::Equivalent<Collision> for CollisionQuery<'_> {
        fn equivalent(&self, key: &Collision) -> bool {
            self.0 == key.0.as_str()
        }
    }

    #[test]
    fn owned_business_map_borrowed_unicode_tuple_and_collision_delete_chains() {
        let budget = OwnedBudget::new(1_000_000);
        {
            let mut plain = new_map::<TextValue, i64>(None);
            let mut owned = new_map::<TextValue, i64>(Some(&budget));
            for (n, key) in ["中文😀", "é", "e\u{301}", "消息\\\""].iter().enumerate() {
                plain
                    .insert(TextValue::from_str(key, None).unwrap(), n as i64)
                    .unwrap();
                owned
                    .insert(TextValue::from_str(key, Some(&budget)).unwrap(), n as i64)
                    .unwrap();
            }
            let attempts = budget.snapshot().allocation_attempts;
            for key in ["中文😀", "é", "e\u{301}", "消息\\\"", "不存在"] {
                assert_eq!(plain.get(key), owned.get(key));
                assert_eq!(plain.contains_key(key), owned.contains_key(key));
            }
            *owned.get_mut("中文😀").unwrap() = 9;
            assert_eq!(owned.remove("中文😀"), Some(9));
            assert_eq!(owned.remove("中文😀"), None);
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            assert_eq!(owned.iter().len(), owned.len());

            let mut phase = new_map::<(i64, TextValue), i64>(Some(&budget));
            phase
                .insert(
                    (7, TextValue::from_str("批次😀", Some(&budget)).unwrap()),
                    42,
                )
                .unwrap();
            let mut messages = new_map::<(TextValue, TextValue, i64), i64>(Some(&budget));
            messages
                .insert(
                    (
                        TextValue::from_str("消息", Some(&budget)).unwrap(),
                        TextValue::from_str("阶段é", Some(&budget)).unwrap(),
                        3,
                    ),
                    11,
                )
                .unwrap();
            let attempts = budget.snapshot().allocation_attempts;
            assert_eq!(phase.get(&PhaseQuery(7, "批次😀")), Some(&42));
            assert_eq!(phase.remove(&PhaseQuery(7, "批次😀")), Some(42));
            assert!(phase.is_empty());
            assert_eq!(messages.get(&MessageQuery("消息", "阶段é", 3)), Some(&11));
            assert_eq!(messages.remove(&MessageQuery("消息", "阶段é", 3)), Some(11));
            assert_eq!(budget.snapshot().allocation_attempts, attempts);

            let mut collisions = new_map::<Collision, i64>(Some(&budget));
            let keys = [
                "头😀",
                "中文",
                "é",
                "e\u{301}",
                "尾",
                "中间",
                "再次😀",
                "终",
            ];
            for (n, key) in keys.iter().enumerate() {
                collisions
                    .insert(
                        Collision(TextValue::from_str(key, Some(&budget)).unwrap()),
                        n as i64,
                    )
                    .unwrap();
            }
            let attempts = budget.snapshot().allocation_attempts;
            for removed in [0usize, 3, 5, 7] {
                assert_eq!(
                    collisions.remove(&CollisionQuery(keys[removed])),
                    Some(removed as i64)
                );
            }
            for (n, key) in keys.iter().enumerate() {
                assert_eq!(
                    collisions.get(&CollisionQuery(key)).copied(),
                    if [0usize, 3, 5, 7].contains(&n) {
                        None
                    } else {
                        Some(n as i64)
                    }
                );
            }
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            for n in [1usize, 2, 4, 6] {
                assert_eq!(collisions.remove(&CollisionQuery(keys[n])), Some(n as i64));
            }
            assert!(collisions.is_empty());
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn owned_business_map_old_new_live_rehash_one_byte_short_preserves_values() {
        let budget = OwnedBudget::new(1_000_000);
        {
            let mut old = new_map::<TextValue, TextValue>(Some(&budget));
            old.insert(
                TextValue::from_str("旧键😀", Some(&budget)).unwrap(),
                TextValue::from_str("旧值é", Some(&budget)).unwrap(),
            )
            .unwrap();
            let mut new = new_map::<i64, i64>(Some(&budget));
            new.insert(0, 10).unwrap();
            for n in 1..new.capacity() {
                new.insert(n as i64, n as i64 + 10).unwrap();
            }
            let count = new.len();
            let mut probe = new_map::<i64, i64>(Some(&budget));
            for n in 0..count {
                probe.insert(n as i64, n as i64 + 10).unwrap();
            }
            probe.insert(count as i64, 99).unwrap();
            let request = budget.allocation_request_probe().last_request_bytes;
            drop(probe);
            let live = budget.snapshot().live_requested_layout_bytes;
            let fill = sc::capture_allocation::CaptureSlab::new(
                0,
                budget.snapshot().limit_bytes - live - request + 1,
                &budget,
            )
            .unwrap();
            let before_refusal = budget.snapshot();
            let before_capacity = new.capacity();
            let error = new.insert(count as i64, 99).unwrap_err();
            assert_eq!(error.code(), sc::capture_allocation::LIMIT);
            assert_eq!(new.len(), count);
            assert_eq!(new.capacity(), before_capacity);
            for n in 0..count {
                assert_eq!(new.get(&(n as i64)), Some(&(n as i64 + 10)));
            }
            assert!(!new.contains_key(&(count as i64)));
            assert_eq!(old.get("旧键😀").unwrap().as_str(), "旧值é");
            let refusal = budget.allocation_request_probe();
            assert_eq!(refusal.refused_bytes, request);
            assert_eq!(
                refusal.live_before_refusal + request,
                budget.snapshot().limit_bytes + 1
            );
            // reserve_on先拒绝超额请求，再计实际allocator尝试；预检拒绝不能要求attempts增加。
            let after_refusal = budget.snapshot();
            assert_eq!(
                after_refusal.rejected_layouts,
                before_refusal.rejected_layouts + 1
            );
            assert_eq!(
                after_refusal.allocation_attempts,
                before_refusal.allocation_attempts
            );
            assert_eq!(
                after_refusal.allocated_layouts,
                before_refusal.allocated_layouts
            );
            assert_eq!(
                after_refusal.released_layouts,
                before_refusal.released_layouts
            );
            assert_eq!(
                after_refusal.live_requested_layout_bytes,
                before_refusal.live_requested_layout_bytes
            );
            let admitted_attempts = after_refusal.allocation_attempts;
            // 删除和已有值替换无需table分配，在同一拒绝头寸下仍保持其既有语义。
            assert_eq!(new.remove(&0), Some(10));
            assert_eq!(new.insert(1, 77).unwrap(), Some(11));
            assert_eq!(new.get(&1), Some(&77));
            assert_eq!(budget.snapshot().allocation_attempts, admitted_attempts);
            let removed = old.remove("旧键😀").unwrap();
            assert_eq!(removed.as_str(), "旧值é");
            assert!(old.is_empty());
            assert_eq!(budget.snapshot().allocation_attempts, admitted_attempts);
            drop(fill);
            drop(removed);
            new.insert(count as i64, 99).unwrap();
            assert_eq!(new.get(&(count as i64)), Some(&99));
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn owned_text_consumes_real_bytes_zero_copy_utf8_and_drops_zero() {
        let budget = OwnedBudget::new(10000);
        {
            let mut bytes = new_work(Some(&budget));
            bytes.try_reserve_exact("中文😀é".len()).unwrap();
            bytes_extend(&mut bytes, "中文😀é".as_bytes()).unwrap();
            let pointer = bytes.as_ptr();
            let live = budget.snapshot().live_requested_layout_bytes;
            let attempts = budget.snapshot().allocation_attempts;
            let text = TextValue::from_owned_bytes(bytes).unwrap();
            assert!(matches!(text, TextValue::Owned(_)));
            assert_eq!(text.as_str(), "中文😀é");
            assert_eq!(text.as_str().as_ptr(), pointer);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, live);
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            let clone = text.try_clone().unwrap();
            assert_eq!(clone.as_str(), text.as_str());
            assert!(budget.snapshot().live_requested_layout_bytes > live);
            drop(clone);
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let plain = sc::CodecVec::from_plain("普通😀".as_bytes().to_vec());
        let pointer = plain.as_ptr();
        let text = TextValue::from_bytes(plain).unwrap();
        assert!(matches!(text, TextValue::Plain(_)));
        assert_eq!(text.as_str().as_ptr(), pointer);
        assert!(TextValue::from_owned_bytes(sc::CodecVec::from_plain(vec![b'a'])).is_err());
        let mut invalid = new_work(Some(&budget));
        bytes_extend(&mut invalid, &[0xff]).unwrap();
        let attempts = budget.snapshot().allocation_attempts;
        assert!(TextValue::from_owned_bytes(invalid).is_err());
        assert_eq!(budget.snapshot().allocation_attempts, attempts);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod checked_sql_owner_tests {
    use super::*;

    #[test]
    fn checked_sql_text_owners_match_legacy_types_null_utf8_and_order() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE conversion_rows(position INTEGER, text_value, optional_value, integer_value, optional_integer, blob_value); INSERT INTO conversion_rows VALUES(2,'中文😀',NULL,9223372036854775807,NULL,x'00ff'); INSERT INTO conversion_rows VALUES(1,'é','é',-9223372036854775808,7,x'');").unwrap();
        let ordinary=conn.prepare("SELECT text_value,optional_value,integer_value,optional_integer,blob_value FROM conversion_rows ORDER BY position").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<i64>>(3)?,r.get::<_,Vec<u8>>(4)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        let budget = OwnedBudget::new(1_000_000);
        for owned in [false, true] {
            let mut stmt=CheckedStatement::prepare(&conn,c"SELECT text_value,optional_value,integer_value,optional_integer,blob_value FROM conversion_rows ORDER BY position").unwrap();
            let mut ordinal = 0;
            while let Some(mut row) = stmt.next().unwrap() {
                let owner = owned.then_some(&budget);
                let text = TextValue::from_sql(&mut row, 0, owner).unwrap();
                let optional = TextValue::optional_sql(&mut row, 1, owner).unwrap();
                assert_eq!(text.as_str(), ordinary[ordinal].0);
                assert_eq!(
                    optional.as_ref().map(TextValue::as_str),
                    ordinary[ordinal].1.as_deref()
                );
                assert_eq!(row.integer(2).unwrap(), ordinary[ordinal].2);
                assert_eq!(row.optional_integer(3).unwrap(), ordinary[ordinal].3);
                assert_eq!(row.blob(4).unwrap(), &ordinary[ordinal].4);
                assert_eq!(matches!(text, TextValue::Owned(_)), owned);
                ordinal += 1;
            }
            assert_eq!(ordinal, ordinary.len());
            stmt.finalize().unwrap();
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        let mut stmt =
            CheckedStatement::prepare(&conn, c"SELECT NULL,7,x'ff',CAST(x'ff' AS TEXT)").unwrap();
        let mut row = stmt.next().unwrap().unwrap();
        assert!(TextValue::from_sql(&mut row, 0, Some(&budget)).is_err());
        assert!(TextValue::optional_sql(&mut row, 0, Some(&budget))
            .unwrap()
            .is_none());
        for column in [1, 2, 3] {
            assert!(TextValue::from_sql(&mut row, column, Some(&budget)).is_err());
            assert!(TextValue::optional_sql(&mut row, column, Some(&budget)).is_err());
        }
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        drop(row);
        stmt.finalize().unwrap();
    }
}
