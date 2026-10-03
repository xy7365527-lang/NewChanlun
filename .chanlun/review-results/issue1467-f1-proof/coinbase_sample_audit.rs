#![recursion_limit = "256"]
// #1467/#1468：一个具名历史切片的协议探针，不是生产行情适配器或成交模拟器。
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

type R<T> = Result<T, String>;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn text<'a>(v: &'a Value, k: &str) -> R<&'a str> {
    v[k].as_str()
        .ok_or_else(|| format!("missing string field {k}"))
}
fn integer(v: &Value, k: &str) -> R<u64> {
    v[k].as_u64()
        .or_else(|| v[k].as_str().and_then(|s| s.parse().ok()))
        .ok_or_else(|| format!("invalid integer field {k}"))
}
fn fixed(s: &str) -> R<i64> {
    let (a, b) = s.split_once('.').unwrap_or((s, ""));
    if a.is_empty() || b.len() > 8 || !a.bytes().chain(b.bytes()).all(|c| c.is_ascii_digit()) {
        return Err("decimal outside exact nonnegative scale-8 domain".into());
    }
    let x = a.parse::<i64>().map_err(|_| "decimal whole overflow")?;
    let frac = if b.is_empty() {
        0
    } else {
        b.parse::<i64>().map_err(|_| "decimal fraction invalid")?
    };
    x.checked_mul(100_000_000)
        .and_then(|v| v.checked_add(frac * 10i64.pow(8 - b.len() as u32)))
        .ok_or_else(|| "scaled decimal overflow".into())
}
fn number(v: &Value, k: &str) -> R<i64> {
    fixed(text(v, k)?)
}
fn buy(v: &Value) -> R<bool> {
    match text(v, "side")? {
        "buy" => Ok(true),
        "sell" => Ok(false),
        _ => Err("invalid side".into()),
    }
}
fn clock(s: &str) -> R<(u32, u64)> {
    if s.len() < 20 || !s.is_ascii() || !s.ends_with('Z') || &s[10..11] != "T" {
        return Err("unsupported UTC timestamp".into());
    }
    let date = s[..10]
        .replace('-', "")
        .parse()
        .map_err(|_| "date invalid")?;
    let hh: u64 = s[11..13].parse().map_err(|_| "hour invalid")?;
    let mm: u64 = s[14..16].parse().map_err(|_| "minute invalid")?;
    let ss: u64 = s[17..19].parse().map_err(|_| "second invalid")?;
    let f = &s[19..s.len() - 1];
    let frac = if f.is_empty() {
        ""
    } else {
        f.strip_prefix('.').ok_or("timestamp fraction invalid")?
    };
    if hh >= 24
        || mm >= 60
        || ss >= 60
        || frac.len() > 9
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("timestamp outside admitted precision/range".into());
    }
    let ns = if frac.is_empty() {
        0
    } else {
        frac.parse::<u64>().map_err(|_| "fraction invalid")? * 10u64.pow(9 - frac.len() as u32)
    };
    Ok((date, (hh * 3600 + mm * 60 + ss) * 1_000_000_000 + ns))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Order {
    buy: bool,
    price: i64,
    size: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Record {
    captured_line: usize,
    local: String,
    message: Value,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Trade {
    sequence: u64,
    captured_line: usize,
    local: String,
    event_time: String,
    price: i64,
    size: i64,
    maker_buy: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct State {
    lines: usize,
    snapshots: usize,
    disconnects: usize,
    snapshot_line: Option<usize>,
    snapshot_sequence: Option<u64>,
    frontier: Option<u64>,
    orders: BTreeMap<String, Order>,
    bids: BTreeMap<i64, i64>,
    asks: BTreeMap<i64, i64>,
    pending: BTreeMap<u64, Record>,
    seen: BTreeMap<u64, String>,
    trades: BTreeMap<u64, Trade>,
    raw_types: BTreeMap<String, usize>,
    applied_types: BTreeMap<String, usize>,
    first_local: Option<String>,
    last_local: Option<String>,
    last_local_key: Option<(u32, u64)>,
    last_event_key: Option<(u32, u64)>,
    local_ties: usize,
    event_ties: usize,
    event_time_backwards: usize,
    seq_first: Option<u64>,
    seq_last_capture: Option<u64>,
    seq_gaps_in_capture: usize,
    seq_backwards_in_capture: usize,
    skipped_covered_by_snapshot: usize,
    duplicate_messages: usize,
    ignored_nonresting_done: usize,
    ignored_nonresting_change: usize,
    resting_cancel: usize,
    resting_change: usize,
    applied_match: usize,
    book_changes: usize,
    fixed_best_price_book_changes: usize,
    fixed_best_price_top_quantity_changes: usize,
    best_price_changes: usize,
    crossed_or_locked_states: usize,
    missing_two_sided_states: usize,
    buffered_peak: usize,
    applied_after_later_capture: usize,
}
#[derive(Serialize, Deserialize)]
struct Checkpoint {
    schema: String,
    source_sha256: String,
    state: State,
}

impl State {
    fn level_delta(&mut self, buy: bool, price: i64, delta: i64) -> R<()> {
        let levels = if buy { &mut self.bids } else { &mut self.asks };
        let q = levels
            .get(&price)
            .copied()
            .unwrap_or(0)
            .checked_add(delta)
            .ok_or("level quantity overflow")?;
        if q < 0 {
            return Err("negative price-level quantity".into());
        }
        if q == 0 {
            levels.remove(&price);
        } else {
            levels.insert(price, q);
        }
        Ok(())
    }
    fn add(&mut self, id: String, o: Order) -> R<()> {
        if id.is_empty()
            || id.len() > 256
            || o.price <= 0
            || o.size <= 0
            || self.orders.contains_key(&id)
        {
            return Err("invalid or duplicate resting order".into());
        }
        self.level_delta(o.buy, o.price, o.size)?;
        self.orders.insert(id, o);
        Ok(())
    }
    fn remove(&mut self, id: &str) -> R<Option<Order>> {
        if let Some(o) = self.orders.remove(id) {
            self.level_delta(o.buy, o.price, -o.size)?;
            Ok(Some(o))
        } else {
            Ok(None)
        }
    }
    fn quote(&self) -> Option<(i64, i64, i64, i64)> {
        let (bp, bq) = self.bids.last_key_value()?;
        let (ap, aq) = self.asks.first_key_value()?;
        Some((*bp, *bq, *ap, *aq))
    }
    fn apply(&mut self, record: &Record, available_line: usize) -> R<()> {
        let m = &record.message;
        let kind = text(m, "type")?;
        let before = self.quote();
        let mut changed = false;
        *self.applied_types.entry(kind.into()).or_default() += 1;
        if record.captured_line < available_line {
            self.applied_after_later_capture += 1;
        }
        match kind {
            "received" => {}
            "open" => {
                self.add(
                    text(m, "order_id")?.into(),
                    Order {
                        buy: buy(m)?,
                        price: number(m, "price")?,
                        size: number(m, "remaining_size")?,
                    },
                )?;
                changed = true;
            }
            "done" => {
                if let Some(o) = self.orders.get(text(m, "order_id")?).cloned() {
                    if buy(m)? != o.buy
                        || number(m, "price")? != o.price
                        || number(m, "remaining_size")? != o.size
                    {
                        return Err("resting done fields disagree with book".into());
                    }
                    if text(m, "reason")? == "canceled" {
                        self.resting_cancel += 1;
                    }
                    self.remove(text(m, "order_id")?)?;
                    changed = true;
                } else {
                    self.ignored_nonresting_done += 1;
                }
            }
            "match" => {
                let id = text(m, "maker_order_id")?;
                let o = self
                    .orders
                    .get(id)
                    .cloned()
                    .ok_or("match maker absent from reconstructed book")?;
                let qty = number(m, "size")?;
                if qty <= 0 || qty > o.size || number(m, "price")? != o.price || buy(m)? != o.buy {
                    return Err("match quantity/price/side conflicts with maker".into());
                }
                self.remove(id)?;
                if qty < o.size {
                    self.add(
                        id.into(),
                        Order {
                            size: o.size - qty,
                            ..o
                        },
                    )?;
                }
                self.applied_match += 1;
                changed = true;
            }
            "change" => {
                if m.get("new_price").is_some() {
                    return Err(
                        "modern modify-price message outside this historical profile".into(),
                    );
                }
                let id = text(m, "order_id")?;
                if let Some(o) = self.orders.get(id).cloned() {
                    if number(m, "old_size")? != o.size
                        || number(m, "price")? != o.price
                        || buy(m)? != o.buy
                    {
                        return Err("resting change fields disagree with book".into());
                    }
                    let new_size = number(m, "new_size")?;
                    self.remove(id)?;
                    if new_size > 0 {
                        self.add(
                            id.into(),
                            Order {
                                size: new_size,
                                ..o
                            },
                        )?;
                    }
                    self.resting_change += 1;
                    changed = true;
                } else {
                    self.ignored_nonresting_change += 1;
                }
            }
            _ => return Err("unsupported full-channel event".into()),
        }
        if changed {
            self.book_changes += 1;
            match (before, self.quote()) {
                (Some((bp, bq, ap, aq)), Some((bp2, bq2, ap2, aq2))) => {
                    if bp == bp2 && ap == ap2 {
                        self.fixed_best_price_book_changes += 1;
                        if bq != bq2 || aq != aq2 {
                            self.fixed_best_price_top_quantity_changes += 1;
                        }
                    } else {
                        self.best_price_changes += 1;
                    }
                    if bp2 >= ap2 {
                        self.crossed_or_locked_states += 1;
                    }
                }
                _ => self.missing_two_sided_states += 1,
            }
        }
        Ok(())
    }
    fn flush(&mut self) -> R<()> {
        while let Some(last) = self.frontier {
            let next = last.checked_add(1).ok_or("sequence overflow")?;
            let Some(r) = self.pending.remove(&next) else {
                break;
            };
            self.apply(&r, self.lines)
                .map_err(|e| format!("sequence {next}: {e}"))?;
            self.frontier = Some(next);
        }
        Ok(())
    }
    fn consume(&mut self, line: &str) -> R<()> {
        self.lines += 1;
        if line.is_empty() {
            self.disconnects += 1;
            self.frontier = None;
            self.pending.clear();
            self.seen.clear();
            self.orders.clear();
            self.bids.clear();
            self.asks.clear();
            return Ok(());
        }
        let (local, data) = line
            .split_once(' ')
            .ok_or("missing local timestamp prefix")?;
        let key = clock(local)?;
        if let Some(old) = self.last_local_key {
            if key < old {
                return Err("capture time decreased".into());
            }
            if key == old {
                self.local_ties += 1;
            }
        }
        self.first_local.get_or_insert_with(|| local.into());
        self.last_local = Some(local.into());
        self.last_local_key = Some(key);
        let m: Value = serde_json::from_str(data).map_err(|_| "invalid native JSON")?;
        if text(&m, "product_id")? != "BTC-USD" {
            return Err("unexpected product".into());
        }
        let kind = text(&m, "type")?.to_owned();
        *self.raw_types.entry(kind.clone()).or_default() += 1;
        if let Some(t) = m.get("time").and_then(Value::as_str) {
            let k = clock(t)?;
            if let Some(old) = self.last_event_key {
                if k < old {
                    self.event_time_backwards += 1;
                }
                if k == old {
                    self.event_ties += 1;
                }
            }
            self.last_event_key = Some(k);
        }
        let seq = integer(&m, "sequence")?;
        if kind == "full_snapshot" {
            if m["generated"].as_bool() != Some(true) {
                return Err("snapshot provenance missing".into());
            }
            self.orders.clear();
            self.bids.clear();
            self.asks.clear();
            for (field, is_buy) in [("bids", true), ("asks", false)] {
                for row in m[field].as_array().ok_or("snapshot side missing")? {
                    let a = row.as_array().ok_or("snapshot row not array")?;
                    if a.len() != 3 {
                        return Err("snapshot is not L3".into());
                    }
                    self.add(
                        a[2].as_str().ok_or("snapshot ID missing")?.into(),
                        Order {
                            buy: is_buy,
                            price: fixed(a[0].as_str().ok_or("snapshot price missing")?)?,
                            size: fixed(a[1].as_str().ok_or("snapshot size missing")?)?,
                        },
                    )?;
                }
            }
            self.snapshots += 1;
            self.snapshot_line = Some(self.lines);
            self.snapshot_sequence = Some(seq);
            self.frontier = Some(seq);
            let old = self.pending.len();
            self.pending.retain(|s, _| *s > seq);
            self.skipped_covered_by_snapshot += old - self.pending.len();
            return self.flush();
        }
        if !["received", "open", "done", "match", "change"].contains(&kind.as_str()) {
            return Err("unknown message type".into());
        }
        self.seq_first.get_or_insert(seq);
        if let Some(prev) = self.seq_last_capture {
            if seq > prev + 1 {
                self.seq_gaps_in_capture += 1;
            }
            if seq <= prev {
                self.seq_backwards_in_capture += 1;
            }
        }
        self.seq_last_capture = Some(seq);
        let signature = hash(
            serde_json::to_string(&m)
                .map_err(|_| "canonical JSON failed")?
                .as_bytes(),
        );
        if let Some(old) = self.seen.get(&seq) {
            if old != &signature {
                return Err("same sequence with conflicting payload".into());
            }
            self.duplicate_messages += 1;
            return Ok(());
        }
        self.seen.insert(seq, signature);
        if kind == "match" {
            let id = integer(&m, "trade_id")?;
            let trade = Trade {
                sequence: seq,
                captured_line: self.lines,
                local: local.into(),
                event_time: text(&m, "time")?.into(),
                price: number(&m, "price")?,
                size: number(&m, "size")?,
                maker_buy: buy(&m)?,
            };
            if trade.price <= 0 || trade.size <= 0 || self.trades.contains_key(&id) {
                return Err("invalid or duplicate trade id".into());
            }
            self.trades.insert(id, trade);
        }
        if self.frontier.is_some_and(|s| seq <= s) {
            self.skipped_covered_by_snapshot += 1;
            return Ok(());
        }
        self.pending.insert(
            seq,
            Record {
                captured_line: self.lines,
                local: local.into(),
                message: m,
            },
        );
        self.buffered_peak = self.buffered_peak.max(self.pending.len());
        self.flush()
    }
    fn verify_totals(&self) -> R<()> {
        let (mut bids, mut asks) = (BTreeMap::<i64, i64>::new(), BTreeMap::<i64, i64>::new());
        for o in self.orders.values() {
            if o.price <= 0 || o.size <= 0 {
                return Err("invalid stored order".into());
            }
            let map = if o.buy { &mut bids } else { &mut asks };
            let q = map.entry(o.price).or_default();
            *q = q
                .checked_add(o.size)
                .ok_or("recomputed quantity overflow")?;
        }
        if bids != self.bids || asks != self.asks {
            return Err("independent price-level aggregation differs".into());
        }
        Ok(())
    }
}

fn checkpoint(path: &Path, source: &str, s: &State) -> R<()> {
    let bytes = serde_json::to_vec(&Checkpoint {
        schema: "coinbase-sample-audit/1".into(),
        source_sha256: source.into(),
        state: s.clone(),
    })
    .map_err(|_| "checkpoint serialization failed")?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}
fn run() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 || !["audit", "resume"].contains(&args[1].as_str()) {
        return Err("usage: audit|resume INPUT CHECKPOINT_DIR|CHECKPOINT_JSON REPORT_JSON".into());
    }
    let input = Path::new(&args[2]);
    if fs::metadata(input).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
        return Err("input exceeds byte budget".into());
    }
    let source = hash(&fs::read(input).map_err(|e| e.to_string())?);
    let mut state = if args[1] == "resume" {
        let cp: Checkpoint =
            serde_json::from_slice(&fs::read(&args[3]).map_err(|e| e.to_string())?)
                .map_err(|_| "invalid checkpoint")?;
        if cp.schema != "coinbase-sample-audit/1" || cp.source_sha256 != source {
            return Err("checkpoint/source binding mismatch".into());
        }
        cp.state
    } else {
        fs::create_dir_all(&args[3]).map_err(|e| e.to_string())?;
        State::default()
    };
    let start = state.lines;
    if args[1] == "audit" {
        checkpoint(&Path::new(&args[3]).join("empty.json"), &source, &state)?;
    }
    for (i, line) in BufReader::new(fs::File::open(input).map_err(|e| e.to_string())?)
        .lines()
        .enumerate()
    {
        if i < start {
            continue;
        }
        let line = line.map_err(|e| e.to_string())?;
        let is_snapshot = line
            .split_once(' ')
            .and_then(|(_, s)| serde_json::from_str::<Value>(s).ok())
            .is_some_and(|m| m["type"] == "full_snapshot");
        if args[1] == "audit" && is_snapshot {
            checkpoint(
                &Path::new(&args[3]).join("before-snapshot.json"),
                &source,
                &state,
            )?;
        }
        state
            .consume(&line)
            .map_err(|e| format!("line {}: {e}", i + 1))?;
        if args[1] == "audit" && (is_snapshot || state.lines == 10000) {
            checkpoint(
                &Path::new(&args[3]).join(if is_snapshot {
                    "after-snapshot.json"
                } else {
                    "middle.json"
                }),
                &source,
                &state,
            )?;
        }
    }
    state.verify_totals()?;
    let full = serde_json::to_vec(&state).map_err(|_| "final serialization failed")?;
    let readiness = state.snapshots > 0
        && state.disconnects == 0
        && state.pending.is_empty()
        && state.seq_gaps_in_capture == 0
        && state.seq_backwards_in_capture == 0;
    let report = json!({"profile":"coinbase-full-20210101-first-minute/1","source_sha256":source,"state_sha256":hash(&full),"state_bytes":full.len(),"restored_from_line":start,"lines":state.lines,"snapshots":state.snapshots,"snapshot_line":state.snapshot_line,"snapshot_sequence":state.snapshot_sequence,"frontier":state.frontier,"disconnects":state.disconnects,"pending_at_end":state.pending.len(),"raw_types":state.raw_types,"applied_types":state.applied_types,"raw_trade_count":state.trades.len(),"first_local":state.first_local,"last_local":state.last_local,"local_ties":state.local_ties,"event_ties":state.event_ties,"event_time_backwards":state.event_time_backwards,"seq_first":state.seq_first,"seq_last_capture":state.seq_last_capture,"sequence_gaps":state.seq_gaps_in_capture,"sequence_backwards":state.seq_backwards_in_capture,"duplicates":state.duplicate_messages,"skipped_covered_by_snapshot":state.skipped_covered_by_snapshot,"buffered_peak":state.buffered_peak,"applied_after_later_capture":state.applied_after_later_capture,"ignored_nonresting_done":state.ignored_nonresting_done,"ignored_nonresting_change":state.ignored_nonresting_change,"resting_cancel":state.resting_cancel,"resting_change":state.resting_change,"applied_match":state.applied_match,"book_changes":state.book_changes,"fixed_best_price_book_changes":state.fixed_best_price_book_changes,"fixed_best_price_top_quantity_changes":state.fixed_best_price_top_quantity_changes,"best_price_changes":state.best_price_changes,"crossed_or_locked_states":state.crossed_or_locked_states,"missing_two_sided_states":state.missing_two_sided_states,"final_order_count":state.orders.len(),"bid_price_levels":state.bids.len(),"ask_price_levels":state.asks.len(),"independent_aggregation_matches":true,"sample_sequence_and_reconstruction_pass":readiness,"scope":"one-minute protocol evidence; no PnL, full-venue completeness, queue priority or F2 qualification"});
    fs::write(&args[4], serde_json::to_vec_pretty(&report).unwrap()).map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string(&report).unwrap());
    if !readiness {
        return Err("sample contains unresolved sequence/reconstruction scope".into());
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
