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

// #1467/#1468 Stage16：上方核心与Stage9冻结版本逐字相同；下方仅增加采集前缀视图。
use std::io::{BufWriter, Write};

const TRACE_SCHEMA: &str = "coinbase-causal-view/1";

#[derive(Serialize, Deserialize)]
struct TraceCheckpoint {
    schema: String,
    source_sha256: String,
    executable_sha256: String,
    state_sha256: String,
    trace_prefix_sha256: String,
    state: State,
}

fn create_new(path: &Path) -> R<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|e| e.to_string())
}

fn write_new_json(path: &Path, value: &impl Serialize) -> R<()> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let mut file = create_new(path)?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

fn trace_checkpoint(
    path: &Path,
    source: &str,
    executable: &str,
    chain: &str,
    state: &State,
) -> R<()> {
    let state_bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
    write_new_json(
        path,
        &TraceCheckpoint {
            schema: TRACE_SCHEMA.into(),
            source_sha256: source.into(),
            executable_sha256: executable.into(),
            state_sha256: hash(&state_bytes),
            trace_prefix_sha256: chain.into(),
            state: state.clone(),
        },
    )
}

fn view_status(state: &State) -> &'static str {
    if state.frontier.is_none() {
        return "awaiting_snapshot";
    }
    if !state.pending.is_empty() {
        return "sequence_gap";
    }
    match state.quote() {
        None => "missing_quote",
        Some((b, _, a, _)) if b >= a => "crossed_or_locked_quote",
        Some(_) => "ready",
    }
}

fn sequence_string(sequence: Option<u64>) -> Option<String> {
    sequence.map(|s| s.to_string())
}

fn run_trace() -> R<bool> {
    let args: Vec<_> = std::env::args().collect();
    if !(args.len() == 6 || args.len() == 7) || !["replay", "resume"].contains(&args[1].as_str()) {
        return Err("usage: replay|resume INPUT CHECKPOINT_DIR|CHECKPOINT_JSON ROWS_JSONL REPORT_JSON [STOP_AFTER_LINE]".into());
    }
    let stop = args
        .get(6)
        .map(|s| {
            s.parse::<usize>()
                .map_err(|_| "invalid stop line".to_string())
        })
        .transpose()?;
    let input = Path::new(&args[2]);
    if fs::metadata(input).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
        return Err("input exceeds byte budget".into());
    }
    let source = hash(&fs::read(input).map_err(|e| e.to_string())?);
    let executable = hash(
        &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    let replay = args[1] == "replay";
    let (mut state, mut chain) = if replay {
        fs::create_dir(&args[3]).map_err(|e| e.to_string())?;
        (State::default(), hash(TRACE_SCHEMA.as_bytes()))
    } else {
        let cp: TraceCheckpoint =
            serde_json::from_slice(&fs::read(&args[3]).map_err(|e| e.to_string())?)
                .map_err(|_| "invalid trace checkpoint")?;
        if cp.schema != TRACE_SCHEMA {
            return Err("checkpoint schema mismatch".into());
        }
        if cp.source_sha256 != source {
            return Err("checkpoint/source binding mismatch".into());
        }
        if cp.executable_sha256 != executable {
            return Err("checkpoint/executable binding mismatch".into());
        }
        if hash(&serde_json::to_vec(&cp.state).map_err(|e| e.to_string())?) != cp.state_sha256 {
            return Err("checkpoint state checksum mismatch".into());
        }
        if cp.trace_prefix_sha256.len() != 64
            || !cp
                .trace_prefix_sha256
                .bytes()
                .all(|c| c.is_ascii_hexdigit())
        {
            return Err("invalid trace prefix checksum".into());
        }
        (cp.state, cp.trace_prefix_sha256)
    };
    let start = state.lines;
    if stop.is_some_and(|n| n < start) {
        return Err("stop precedes restored position".into());
    }
    if replay {
        trace_checkpoint(
            &Path::new(&args[3]).join("empty.json"),
            &source,
            &executable,
            &chain,
            &state,
        )?;
    }
    let mut rows = BufWriter::new(create_new(Path::new(&args[4]))?);
    let mut input_lines = BufReader::new(fs::File::open(input).map_err(|e| e.to_string())?).lines();
    for _ in 0..start {
        input_lines
            .next()
            .ok_or("checkpoint line beyond input")?
            .map_err(|e| e.to_string())?;
    }
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut transitions = BTreeMap::<String, usize>::new();
    let mut emitted_trades = 0usize;
    let mut eof = false;
    while stop.is_none_or(|n| state.lines < n) {
        let Some(line) = input_lines.next() else {
            eof = true;
            break;
        };
        let line = line.map_err(|e| e.to_string())?;
        let (local, message) = if line.is_empty() {
            (None, None)
        } else {
            let (local, json) = line.split_once(' ').ok_or("missing capture prefix")?;
            (
                Some(local.to_string()),
                Some(serde_json::from_str::<Value>(json).map_err(|_| "invalid native JSON")?),
            )
        };
        let kind = message
            .as_ref()
            .and_then(|m| m["type"].as_str())
            .unwrap_or("disconnect")
            .to_string();
        let native_sequence = message
            .as_ref()
            .map(|m| integer(m, "sequence"))
            .transpose()?;
        let is_snapshot = kind == "full_snapshot";
        if replay && is_snapshot && state.snapshots == 0 {
            trace_checkpoint(
                &Path::new(&args[3]).join("before-snapshot.json"),
                &source,
                &executable,
                &chain,
                &state,
            )?;
        }
        let previous_status = view_status(&state);
        let old_applied: usize = state.applied_types.values().sum();
        let old_delayed = state.applied_after_later_capture;
        let old_skipped = state.skipped_covered_by_snapshot;
        let old_trades = state.trades.len();
        let old_book_changes = state.book_changes;
        state
            .consume(&line)
            .map_err(|e| format!("line {}: {e}", state.lines))?;
        let status = if line.is_empty() {
            "disconnected"
        } else {
            view_status(&state)
        };
        let applied = state.applied_types.values().sum::<usize>() - old_applied;
        let delayed = state.applied_after_later_capture - old_delayed;
        let transition = if status != "ready" {
            "unavailable"
        } else if is_snapshot {
            "snapshot_batch"
        } else if delayed > 0 || applied > 1 {
            "catchup_batch"
        } else if applied == 1 && previous_status == "ready" {
            "single_event"
        } else if applied == 0 {
            "no_new_book_application"
        } else {
            "reinitialized_or_batch"
        };
        let quote = if status == "ready" {
            state.quote().map(|(bp, bq, ap, aq)| {
                json!({
                    "bid_price":bp.to_string(),"bid_quantity":bq.to_string(),
                    "ask_price":ap.to_string(),"ask_quantity":aq.to_string(),"scale":100_000_000
                })
            })
        } else {
            None
        };
        let trade = if state.trades.len() > old_trades {
            let m = message.as_ref().ok_or("new trade without message")?;
            let trade = state
                .trades
                .get(&integer(m, "trade_id")?)
                .ok_or("new trade missing")?;
            emitted_trades += 1;
            Some(
                json!({"sequence":trade.sequence.to_string(),"event_at":trade.event_time,
                "known_at_capture":trade.local,"known_at_line":trade.captured_line,
                "price":trade.price.to_string(),"quantity":trade.size.to_string(),
                "maker_buy":trade.maker_buy,"scale":100_000_000}),
            )
        } else {
            None
        };
        let applied_from = if applied > 0 {
            state
                .frontier
                .and_then(|s| s.checked_sub(applied as u64 - 1))
        } else {
            None
        };
        let row = json!({"schema":TRACE_SCHEMA,"line":state.lines,"available_at_capture":local,
            "product_id":"BTC-USD","native_type":kind,"native_sequence":sequence_string(native_sequence),
            "native_event_at":message.as_ref().and_then(|m| m["time"].as_str()),
            "status":status,"transition":transition,"quote":quote,"new_trade":trade,
            "frontier":sequence_string(state.frontier),"pending":state.pending.len(),
            "applied_messages":applied,"applied_from":sequence_string(applied_from),
            "applied_to":if applied>0 {sequence_string(state.frontier)} else {None},
            "delayed_applied_messages":delayed,"book_changes":state.book_changes-old_book_changes,
            "covered_by_snapshot":state.skipped_covered_by_snapshot-old_skipped,
            "snapshot_sequence":sequence_string(state.snapshot_sequence)});
        let mut encoded = serde_json::to_vec(&row).map_err(|e| e.to_string())?;
        encoded.push(b'\n');
        let mut link = chain.as_bytes().to_vec();
        link.extend_from_slice(&encoded);
        chain = hash(&link);
        rows.write_all(&encoded).map_err(|e| e.to_string())?;
        *statuses.entry(status.into()).or_default() += 1;
        *transitions.entry(transition.into()).or_default() += 1;
        if replay && ((is_snapshot && state.snapshots == 1) || state.lines == 10000) {
            let name = if is_snapshot {
                "after-snapshot.json"
            } else {
                "middle.json"
            };
            trace_checkpoint(
                &Path::new(&args[3]).join(name),
                &source,
                &executable,
                &chain,
                &state,
            )?;
        }
    }
    rows.flush().map_err(|e| e.to_string())?;
    drop(rows);
    state.verify_totals()?;
    let state_bytes = serde_json::to_vec(&state).map_err(|e| e.to_string())?;
    let book_bytes = serde_json::to_vec(&(&state.orders, &state.bids, &state.asks, state.frontier))
        .map_err(|e| e.to_string())?;
    let final_ready = view_status(&state) == "ready";
    let report = json!({"schema":TRACE_SCHEMA,"source_sha256":source,"executable_sha256":executable,
        "state_sha256":hash(&state_bytes),"book_sha256":hash(&book_bytes),
        "trace_chain_sha256":chain,"trace_file_sha256":hash(&fs::read(&args[4]).map_err(|e| e.to_string())?),
        "restored_from_line":start,"lines_consumed":state.lines,"rows_emitted":state.lines-start,
        "emitted_new_trades":emitted_trades,"raw_trade_count":state.trades.len(),
        "status_counts":statuses,"transition_counts":transitions,"pending_at_end":state.pending.len(),
        "final_frontier":sequence_string(state.frontier),"final_ready":final_ready,
        "bounded_prefix_requested":stop.is_some(),"stop_after_line":stop,"eof_observed":eof,
        "sequence_gaps_in_capture":state.seq_gaps_in_capture,"sequence_backwards_in_capture":state.seq_backwards_in_capture,
        "scope":"fixed-sample causal views; no independent book oracle, signal qualification, execution or PnL"});
    write_new_json(Path::new(&args[5]), &report)?;
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|e| e.to_string())?
    );
    Ok(stop.is_some() || final_ready)
}

fn main() {
    match run_trace() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("final book is not synchronized; views retain unavailable states");
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
