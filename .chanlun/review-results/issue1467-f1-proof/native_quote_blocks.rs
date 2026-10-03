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

// Stage26：下方为独立驱动和观察层；上方冻结核心不改。
use std::io::{BufWriter, Write};
const PROFILE: &str = "rd-q-native-available/1";
const SCALE: u64 = 100_000_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QuoteValue {
    bid_price: String,
    bid_quantity: String,
    ask_price: String,
    ask_quantity: String,
    scale: u64,
}
fn quote_value(s: &State) -> (Option<QuoteValue>, String) {
    match s.quote() {
        Some((b, bq, a, aq)) if b < a => (
            Some(QuoteValue {
                bid_price: b.to_string(),
                bid_quantity: bq.to_string(),
                ask_price: a.to_string(),
                ask_quantity: aq.to_string(),
                scale: SCALE,
            }),
            "valid".into(),
        ),
        Some(_) => (None, "crossed_or_locked".into()),
        None if s.bids.is_empty() && s.asks.is_empty() => (None, "empty_both".into()),
        None if s.bids.is_empty() => (None, "empty_bid".into()),
        None => (None, "empty_ask".into()),
    }
}
fn feed_status(s: &State, disconnected: bool) -> &'static str {
    if disconnected {
        return "disconnected";
    }
    if s.frontier.is_none() {
        return "awaiting_snapshot";
    }
    if !s.pending.is_empty() {
        return "sequence_gap";
    }
    match s.quote() {
        None => "missing_quote",
        Some((b, _, a, _)) if b >= a => "crossed_or_locked_quote",
        Some(_) => "ready",
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QuoteRef {
    epoch: usize,
    model_state: usize,
    native_sequence: String,
    available_line: usize,
    available_capture: String,
    origin: String,
    value: Option<QuoteValue>,
    status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ModelEvent {
    epoch: usize,
    index: usize,
    native_sequence: String,
    native_type: String,
    native_event_at: Option<String>,
    captured_line: usize,
    captured_at: String,
    available_line: usize,
    available_capture: String,
    model_action: String,
    normalization: String,
    buy: bool,
    price: String,
    amount: String,
    positive: bool,
    fragment: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Fragment {
    rank: i64,
    order: Order,
}
#[derive(Clone, Serialize, Deserialize)]
struct BlockState {
    epoch: usize,
    ordinal: usize,
    positive: bool,
    start: usize,
    finish: usize,
    first_known_line: usize,
    first_known_capture: String,
    start_quote: QuoteRef,
    end_quote: QuoteRef,
    all_quotes_valid: bool,
    events: Vec<ModelEvent>,
}
impl BlockState {
    fn geometry(&self) -> R<Option<Value>> {
        if !self.all_quotes_valid {
            return Ok(None);
        }
        let a = self
            .start_quote
            .value
            .as_ref()
            .ok_or("valid block lacks start quote")?;
        let b = self
            .end_quote
            .value
            .as_ref()
            .ok_or("valid block lacks end quote")?;
        let (sp, ep) = if self.positive {
            (&a.bid_price, &b.ask_price)
        } else {
            (&a.ask_price, &b.bid_price)
        };
        let x = sp
            .parse::<i64>()
            .map_err(|_| "invalid stored start price")?;
        let y = ep.parse::<i64>().map_err(|_| "invalid stored end price")?;
        if (self.positive && x >= y) || (!self.positive && x <= y) {
            return Err("quote-leg direction violation".into());
        }
        Ok(Some(
            json!({"direction":if self.positive {"up"}else{"down"},"start_model_state":self.start,"end_model_state":self.finish,"start_price":sp,"end_price":ep,"low":x.min(y).to_string(),"high":x.max(y).to_string(),"scale":SCALE,"value_kind":"selected_quote_edges_not_trade_prices"}),
        ))
    }
    fn view(&self, full: bool) -> R<Value> {
        let mut v = json!({"epoch":self.epoch,"ordinal":self.ordinal,"positive":self.positive,
            "start":self.start,"finish":self.finish,"first_known_model":self.start+1,
            "first_known_line":self.first_known_line,"first_known_capture":self.first_known_capture,
            "start_quote":self.start_quote,"end_quote":self.end_quote,"all_quotes_valid":self.all_quotes_valid,
            "geometry":self.geometry()?,"event_count":self.events.len(),"provisional":true});
        if full {
            v["events"] = serde_json::to_value(&self.events).map_err(|e| e.to_string())?;
        }
        Ok(v)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Observer {
    next_epoch: usize,
    epoch: Option<usize>,
    model_index: usize,
    next_block: usize,
    next_rank: i64,
    quote: Option<QuoteRef>,
    active: Option<BlockState>,
    fragments: BTreeMap<String, Fragment>,
    model_events: usize,
    completed: usize,
    completed_without_geometry: usize,
    ended_active: usize,
    applied_records: usize,
    delayed_records: usize,
    barriers: usize,
    native_effects: BTreeMap<String, usize>,
    normalizations: BTreeMap<String, usize>,
    noops: BTreeMap<String, usize>,
    shape_chain: String,
}
impl Observer {
    fn new() -> Self {
        Self {
            next_epoch: 0,
            epoch: None,
            model_index: 0,
            next_block: 0,
            next_rank: -1,
            quote: None,
            active: None,
            fragments: BTreeMap::new(),
            model_events: 0,
            completed: 0,
            completed_without_geometry: 0,
            ended_active: 0,
            applied_records: 0,
            delayed_records: 0,
            barriers: 0,
            native_effects: BTreeMap::new(),
            normalizations: BTreeMap::new(),
            noops: BTreeMap::new(),
            shape_chain: hash(PROFILE.as_bytes()),
        }
    }
    fn shape(&mut self, v: Value) -> R<()> {
        let mut bytes = self.shape_chain.as_bytes().to_vec();
        bytes.extend(serde_json::to_vec(&v).map_err(|e| e.to_string())?);
        self.shape_chain = hash(&bytes);
        Ok(())
    }
    fn end(&mut self, reason: &str, line: usize, local: Option<&str>) -> R<Vec<Value>> {
        let mut out = Vec::new();
        if let Some(b) = self.active.take() {
            self.ended_active += 1;
            out.push(json!({"type":"epoch_end","reason":reason,"available_line":line,"available_capture":local,"tail_status":"unfinished_on_domain_exit","tail":b.view(true)?}));
        }
        self.epoch = None;
        self.quote = None;
        self.fragments.clear();
        Ok(out)
    }
    fn anchor(
        &mut self,
        s: &State,
        seq: u64,
        line: usize,
        local: &str,
        reason: &str,
    ) -> R<Vec<Value>> {
        let mut out = self.end(reason, line, Some(local))?;
        let epoch = self.next_epoch;
        self.next_epoch += 1;
        self.epoch = Some(epoch);
        self.model_index = 0;
        self.next_block = 0;
        self.next_rank = -1;
        for (i, (id, o)) in s.orders.iter().enumerate() {
            self.fragments.insert(
                id.clone(),
                Fragment {
                    rank: i64::try_from(i).map_err(|_| "fragment index overflow")?,
                    order: o.clone(),
                },
            );
        }
        let (value, status) = quote_value(s);
        let q = QuoteRef {
            epoch,
            model_state: 0,
            native_sequence: seq.to_string(),
            available_line: line,
            available_capture: local.into(),
            origin: reason.into(),
            value,
            status,
        };
        self.shape(json!({"anchor":epoch,"sequence":seq.to_string(),"reason":reason,"quote":q.value,"status":q.status}))?;
        out.push(json!({"type":"anchor","reason":reason,"quote":q}));
        self.quote = Some(q);
        Ok(out)
    }
    fn mutation(&mut self, e: ModelEvent, after: QuoteRef, confirmed: &mut Vec<Value>) -> R<()> {
        let before = self.quote.clone().ok_or("mutation before anchor")?;
        if before.model_state != e.index || e.epoch != before.epoch {
            return Err("model state/event mismatch".into());
        }
        self.shape(
            json!({"epoch":e.epoch,"index":e.index,"sequence":e.native_sequence,
            "action":e.model_action,"normalization":e.normalization,"buy":e.buy,"price":e.price,
            "amount":e.amount,"fragment":e.fragment,"before":before.value,"after":after.value,
            "before_status":before.status,"after_status":after.status}),
        )?;
        if self
            .active
            .as_ref()
            .is_some_and(|b| b.positive != e.positive)
        {
            let b = self.active.take().unwrap();
            if b.finish != e.index {
                return Err("nonadjacent model blocks".into());
            }
            let mut v = b.view(true)?;
            v["provisional"] = json!(false);
            v["known_at_model"] = json!(e.index + 1);
            v["known_at_line"] = json!(e.available_line);
            v["known_at_capture"] = json!(e.available_capture);
            v["confirmation_event"] = serde_json::to_value(&e).map_err(|e| e.to_string())?;
            if b.geometry()?.is_none() {
                self.completed_without_geometry += 1;
            }
            self.completed += 1;
            confirmed.push(v);
        }
        if self.active.is_none() {
            let ordinal = self.next_block;
            self.next_block += 1;
            self.active = Some(BlockState {
                epoch: e.epoch,
                ordinal,
                positive: e.positive,
                start: e.index,
                finish: e.index,
                first_known_line: e.available_line,
                first_known_capture: e.available_capture.clone(),
                start_quote: before.clone(),
                end_quote: before.clone(),
                all_quotes_valid: before.value.is_some(),
                events: Vec::new(),
            });
        }
        let b = self.active.as_mut().unwrap();
        b.finish = e.index + 1;
        b.all_quotes_valid &= after.value.is_some();
        b.end_quote = after.clone();
        b.events.push(e.clone());
        b.geometry()?;
        self.model_index += 1;
        self.model_events += 1;
        self.quote = Some(after);
        *self
            .native_effects
            .entry(e.native_type.clone())
            .or_default() += 1;
        *self
            .normalizations
            .entry(e.normalization.clone())
            .or_default() += 1;
        Ok(())
    }
}

struct Effect {
    id: String,
    before_order: Option<Order>,
    buy: bool,
    price: i64,
    delta: i64,
    action: &'static str,
    reason: &'static str,
    barrier: bool,
}
fn effect_before(s: &State, m: &Value) -> R<(Option<Effect>, &'static str)> {
    let kind = text(m, "type")?;
    match kind {
        "received" => Ok((None, "received_no_book_effect")),
        "open" => Ok((
            Some(Effect {
                id: text(m, "order_id")?.into(),
                before_order: None,
                buy: buy(m)?,
                price: number(m, "price")?,
                delta: number(m, "remaining_size")?,
                action: "add",
                reason: "resting_open",
                barrier: false,
            }),
            "",
        )),
        "done" | "change" | "match" => {
            let id = text(
                m,
                if kind == "match" {
                    "maker_order_id"
                } else {
                    "order_id"
                },
            )?;
            let Some(o) = s.orders.get(id).cloned() else {
                return if kind == "match" {
                    Err("missing maker in observer".into())
                } else {
                    Ok((
                        None,
                        if kind == "done" {
                            "nonresting_done"
                        } else {
                            "nonresting_change"
                        },
                    ))
                };
            };
            let (delta, action, reason, barrier) = match kind {
                "match" => (-number(m, "size")?, "execute", "resting_match", false),
                "done" => (
                    -o.size,
                    "cancel",
                    if text(m, "reason")? == "canceled" {
                        "resting_canceled_done"
                    } else {
                        "resting_done_unmapped_reason"
                    },
                    text(m, "reason")? != "canceled",
                ),
                _ => {
                    let new_size = number(m, "new_size")?;
                    if new_size == o.size {
                        return Ok((None, "resting_size_unchanged"));
                    }
                    if new_size < o.size {
                        (new_size - o.size, "cancel", "resting_size_decrease", false)
                    } else {
                        (
                            new_size - o.size,
                            "add",
                            "resting_size_increase_model_barrier",
                            true,
                        )
                    }
                }
            };
            Ok((
                Some(Effect {
                    id: id.into(),
                    before_order: Some(o.clone()),
                    buy: o.buy,
                    price: o.price,
                    delta,
                    action,
                    reason,
                    barrier,
                }),
                "",
            ))
        }
        _ => Err("observer unsupported native type".into()),
    }
}
fn level_qty(s: &State, buy: bool, p: i64) -> i64 {
    if buy { s.bids.get(&p) } else { s.asks.get(&p) }
        .copied()
        .unwrap_or(0)
}
fn book_shadow(s: &State) -> State {
    State {
        orders: s.orders.clone(),
        bids: s.bids.clone(),
        asks: s.asks.clone(),
        frontier: s.frontier,
        ..State::default()
    }
}
fn same_book(a: &State, b: &State) -> R<()> {
    if a.orders != b.orders || a.bids != b.bids || a.asks != b.asks {
        return Err("observer/main full book mismatch".into());
    }
    Ok(())
}
fn book_digest(s: &State) -> String {
    let mut bytes = Vec::new();
    for (tag, levels) in [("B", &s.bids), ("A", &s.asks)] {
        for (p, q) in levels {
            bytes.extend_from_slice(format!("{tag},{p},{q}\n").as_bytes());
        }
    }
    hash(&bytes)
}
fn observe_application(
    shadow: &mut State,
    o: &mut Observer,
    r: &Record,
    line: usize,
    local: &str,
    traces: &mut Vec<Value>,
    confirmed: &mut Vec<Value>,
) -> R<bool> {
    let seq = integer(&r.message, "sequence")?;
    if shadow.frontier.and_then(|x| x.checked_add(1)) != Some(seq) {
        return Err("noncontiguous observed application".into());
    }
    let before = quote_value(shadow);
    let (effect, noop) = effect_before(shadow, &r.message)?;
    let q_before = effect.as_ref().map(|e| level_qty(shadow, e.buy, e.price));
    shadow.apply(r, line)?;
    shadow.frontier = Some(seq);
    let after = quote_value(shadow);
    o.applied_records += 1;
    if r.captured_line < line {
        o.delayed_records += 1;
    }
    let kind = text(&r.message, "type")?.to_string();
    let mut item = json!({"type":"applied","native_sequence":seq.to_string(),"native_type":kind,
        "captured_line":r.captured_line,"captured_at":r.local,"available_line":line,
        "available_capture":local,"native_event_at":r.message["time"].as_str()});
    let Some(e) = effect else {
        if before != after {
            return Err("no-op changed observed quote".into());
        }
        *o.noops.entry(noop.into()).or_default() += 1;
        item["disposition"] = json!("noop");
        item["normalization"] = json!(noop);
        traces.push(item);
        return Ok(false);
    };
    let q_after = level_qty(shadow, e.buy, e.price);
    if q_after as i128 - q_before.unwrap() as i128 != e.delta as i128 || e.delta == 0 {
        return Err("normalized price-level balance mismatch".into());
    }
    item["before_quote"] = json!(before.0);
    item["before_status"] = json!(before.1);
    item["after_quote"] = json!(after.0);
    item["after_status"] = json!(after.1);
    item["buy"] = json!(e.buy);
    item["price"] = json!(e.price.to_string());
    item["signed_delta"] = json!(e.delta.to_string());
    item["level_before"] = json!(q_before.unwrap().to_string());
    item["level_after"] = json!(q_after.to_string());
    item["normalization"] = json!(e.reason);
    if e.barrier {
        item["disposition"] = json!("model_barrier");
        traces.push(item);
        o.barriers += 1;
        traces.extend(o.anchor(shadow, seq, line, local, e.reason)?);
        return Ok(true);
    }
    let epoch = o.epoch.ok_or("mapped effect without epoch")?;
    let rank = if e.action == "add" {
        if e.before_order.is_some() || o.fragments.contains_key(&e.id) {
            return Err("add is not a fresh order fragment".into());
        }
        let rank = o.next_rank;
        o.next_rank = o.next_rank.checked_sub(1).ok_or("fragment rank overflow")?;
        let order = shadow
            .orders
            .get(&e.id)
            .cloned()
            .ok_or("new fragment absent")?;
        o.fragments.insert(e.id.clone(), Fragment { rank, order });
        rank
    } else {
        let f = o.fragments.get(&e.id).ok_or("missing source fragment")?;
        if Some(&f.order) != e.before_order.as_ref() {
            return Err("source fragment/order mismatch".into());
        }
        let rank = f.rank;
        if let Some(order) = shadow.orders.get(&e.id) {
            o.fragments.insert(
                e.id.clone(),
                Fragment {
                    rank,
                    order: order.clone(),
                },
            );
        } else {
            o.fragments.remove(&e.id);
        }
        rank
    };
    let positive = if e.buy {
        e.action == "add"
    } else {
        e.action != "add"
    };
    if let (Some(q), Some(s)) = (&before.0, &after.0) {
        let (b0, a0, b1, a1) = (
            q.bid_price.parse::<i64>().unwrap(),
            q.ask_price.parse::<i64>().unwrap(),
            s.bid_price.parse::<i64>().unwrap(),
            s.ask_price.parse::<i64>().unwrap(),
        );
        if (positive && (b1 < b0 || a1 < a0)) || (!positive && (b1 > b0 || a1 > a0)) {
            return Err("model quote monotonicity violation".into());
        }
    }
    let previous = o.quote.as_ref().ok_or("missing model quote")?;
    if previous.value != before.0 || previous.status != before.1 {
        return Err("canonical boundary quote differs from before-state".into());
    }
    let event = ModelEvent {
        epoch,
        index: o.model_index,
        native_sequence: seq.to_string(),
        native_type: kind,
        native_event_at: r.message["time"].as_str().map(str::to_owned),
        captured_line: r.captured_line,
        captured_at: r.local.clone(),
        available_line: line,
        available_capture: local.into(),
        model_action: e.action.into(),
        normalization: e.reason.into(),
        buy: e.buy,
        price: e.price.to_string(),
        amount: e
            .delta
            .checked_abs()
            .ok_or("effect amount overflow")?
            .to_string(),
        positive,
        fragment: rank.to_string(),
    };
    let after_ref = QuoteRef {
        epoch,
        model_state: o.model_index + 1,
        native_sequence: seq.to_string(),
        available_line: line,
        available_capture: local.into(),
        origin: "applied_mutation".into(),
        value: after.0,
        status: after.1,
    };
    item["disposition"] = json!("mutation");
    item["model_event"] = serde_json::to_value(&event).map_err(|e| e.to_string())?;
    item["before_model_quote"] = serde_json::to_value(previous).map_err(|e| e.to_string())?;
    item["after_model_quote"] = serde_json::to_value(&after_ref).map_err(|e| e.to_string())?;
    traces.push(item);
    o.mutation(event, after_ref, confirmed)?;
    Ok(true)
}

fn create_new(p: &Path) -> R<fs::File> {
    let mut o = fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(p).map_err(|e| e.to_string())
}
fn save_json(p: &Path, v: &impl Serialize) -> R<()> {
    let mut f = create_new(p)?;
    f.write_all(&serde_json::to_vec(v).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())
}
#[derive(Serialize, Deserialize)]
struct NativeCheckpoint {
    profile: String,
    source_sha256: String,
    executable_sha256: String,
    state_sha256: String,
    observer_sha256: String,
    output_chain_sha256: String,
    state: State,
    observer: Observer,
}
fn checkpoint_native(
    p: &Path,
    source: &str,
    exe: &str,
    chain: &str,
    s: &State,
    o: &Observer,
) -> R<()> {
    save_json(
        p,
        &NativeCheckpoint {
            profile: PROFILE.into(),
            source_sha256: source.into(),
            executable_sha256: exe.into(),
            state_sha256: hash(&serde_json::to_vec(s).map_err(|e| e.to_string())?),
            observer_sha256: hash(&serde_json::to_vec(o).map_err(|e| e.to_string())?),
            output_chain_sha256: chain.into(),
            state: s.clone(),
            observer: o.clone(),
        },
    )
}
fn run_native() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    if !(args.len() == 8 || args.len() == 9) || !["replay", "resume"].contains(&args[1].as_str()) {
        return Err("usage: replay|resume INPUT CP_DIR|FILE ROWS REPORT CP_LINES_OR_DASH [STOP_LINE] must include PROFILE as final argument".into());
    }
    let (stop_arg, profile_arg) = if args.len() == 9 {
        (Some(&args[7]), &args[8])
    } else {
        (None, &args[7])
    };
    if profile_arg != PROFILE {
        return Err("requested profile mismatch".into());
    }
    let stop = stop_arg
        .map(|s| s.parse::<usize>().map_err(|_| "invalid stop".to_string()))
        .transpose()?;
    let checkpoints: Vec<usize> = if args[6] == "-" {
        vec![]
    } else {
        args[6]
            .split(',')
            .map(|s| {
                s.parse::<usize>()
                    .map_err(|_| "invalid checkpoint line".to_string())
            })
            .collect::<R<_>>()?
    };
    if fs::metadata(&args[2]).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
        return Err("input exceeds byte budget".into());
    }
    let source = hash(&fs::read(&args[2]).map_err(|e| e.to_string())?);
    let exe = hash(
        &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    let replay = args[1] == "replay";
    let (mut state, mut observer, mut chain) = if replay {
        fs::create_dir(&args[3]).map_err(|e| e.to_string())?;
        (State::default(), Observer::new(), hash(PROFILE.as_bytes()))
    } else {
        let cp: NativeCheckpoint =
            serde_json::from_slice(&fs::read(&args[3]).map_err(|e| e.to_string())?)
                .map_err(|_| "invalid checkpoint")?;
        if cp.profile != PROFILE || cp.source_sha256 != source || cp.executable_sha256 != exe {
            return Err("checkpoint profile/source/executable mismatch".into());
        }
        if cp.state_sha256 != hash(&serde_json::to_vec(&cp.state).map_err(|e| e.to_string())?)
            || cp.observer_sha256
                != hash(&serde_json::to_vec(&cp.observer).map_err(|e| e.to_string())?)
        {
            return Err("checkpoint payload checksum mismatch".into());
        }
        if cp.output_chain_sha256.len() != 64
            || !cp
                .output_chain_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err("invalid output chain".into());
        }
        (cp.state, cp.observer, cp.output_chain_sha256)
    };
    let restored = state.lines;
    if stop.is_some_and(|n| n < restored) {
        return Err("stop before checkpoint".into());
    }
    let mut shadow = book_shadow(&state);
    let mut journal = state.pending.clone();
    let mut output = BufWriter::new(create_new(Path::new(&args[4]))?);
    let mut input = BufReader::new(fs::File::open(&args[2]).map_err(|e| e.to_string())?).lines();
    for _ in 0..restored {
        input
            .next()
            .ok_or("checkpoint beyond input")?
            .map_err(|e| e.to_string())?;
    }
    while stop.is_none_or(|n| state.lines < n) {
        let Some(raw) = input.next() else { break };
        let raw = raw.map_err(|e| e.to_string())?;
        let row = state.lines + 1;
        let mut traces = Vec::new();
        let mut confirmed = Vec::new();
        let mut active_update = false;
        let (local, msg) = if raw.is_empty() {
            (None, None)
        } else {
            let (t, j) = raw.split_once(' ').ok_or("missing source prefix")?;
            (
                Some(t.to_string()),
                Some(serde_json::from_str::<Value>(j).map_err(|_| "invalid source JSON")?),
            )
        };
        let kind = msg
            .as_ref()
            .and_then(|m| m["type"].as_str())
            .unwrap_or("disconnect");
        let seq = msg.as_ref().map(|m| integer(m, "sequence")).transpose()?;
        let old_frontier = state.frontier;
        let was_seen = seq.is_some_and(|s| state.seen.contains_key(&s));
        let mut snapshot_state = None;
        let mut snapshot_reaffirmed = false;
        if kind == "full_snapshot" {
            let base = seq.unwrap();
            if old_frontier.is_some_and(|s| base < s) {
                return Err("snapshot sequence rewinds admitted history".into());
            }
            let mut candidate = State::default();
            candidate.consume(&raw)?;
            if old_frontier == Some(base) {
                same_book(&candidate, &shadow)
                    .map_err(|_| "same-sequence snapshot contradicts reconstructed book")?;
                snapshot_reaffirmed = true;
            }
            snapshot_state = Some(candidate);
        }
        state
            .consume(&raw)
            .map_err(|e| format!("capture line {row}: {e}"))?;
        if raw.is_empty() {
            journal.clear();
            shadow = State::default();
            traces.extend(observer.end("disconnect", row, None)?);
            active_update = true;
        } else {
            let local = local.as_deref().unwrap();
            if kind == "full_snapshot" {
                let base = seq.unwrap();
                journal.retain(|s, _| *s > base);
                if snapshot_reaffirmed {
                    traces.push(json!({"type":"snapshot_reaffirmed","native_sequence":base.to_string(),"available_line":row,"available_capture":local}));
                } else {
                    shadow = snapshot_state
                        .take()
                        .ok_or("missing snapshot observer state")?;
                    traces.extend(observer.anchor(&shadow, base, row, local, "snapshot")?);
                    active_update = true;
                }
            } else if !was_seen && old_frontier.is_none_or(|s| seq.unwrap() > s) {
                journal.insert(
                    seq.unwrap(),
                    Record {
                        captured_line: row,
                        local: local.into(),
                        message: msg.clone().unwrap(),
                    },
                );
            }
            if let (Some(from), Some(to)) = (shadow.frontier, state.frontier) {
                if to < from {
                    return Err("observer frontier regression".into());
                }
                for next in from + 1..=to {
                    let r = journal
                        .remove(&next)
                        .ok_or("applied source absent from journal")?;
                    active_update |= observe_application(
                        &mut shadow,
                        &mut observer,
                        &r,
                        row,
                        local,
                        &mut traces,
                        &mut confirmed,
                    )?;
                }
            }
        }
        if shadow.frontier != state.frontier
            || shadow.quote() != state.quote()
            || journal.len() != state.pending.len()
        {
            return Err("observer/source frontier or quote mismatch".into());
        }
        let status = feed_status(&state, raw.is_empty());
        let v = json!({"profile":PROFILE,"line":row,"available_at_capture":local,"native_type":kind,
            "native_sequence":seq.map(|s|s.to_string()),"status":status,"frontier":state.frontier.map(|s|s.to_string()),
            "pending":state.pending.len(),"current_quote":if status=="ready"{quote_value(&state).0}else{None},
            "applications":traces,"confirmed":confirmed,"active_update":active_update,
            "active":if active_update {observer.active.as_ref().map(|b|b.view(false)).transpose()?}else{None}});
        let mut bytes = serde_json::to_vec(&v).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let mut link = chain.as_bytes().to_vec();
        link.extend_from_slice(&bytes);
        chain = hash(&link);
        output.write_all(&bytes).map_err(|e| e.to_string())?;
        if replay && checkpoints.contains(&row) {
            state.verify_totals()?;
            same_book(&state, &shadow)?;
            checkpoint_native(
                &Path::new(&args[3]).join(format!("line-{row}.json")),
                &source,
                &exe,
                &chain,
                &state,
                &observer,
            )?;
        }
    }
    output.flush().map_err(|e| e.to_string())?;
    drop(output);
    state.verify_totals()?;
    same_book(&state, &shadow)?;
    let report = json!({"profile":PROFILE,"source_sha256":source,"executable_sha256":exe,
        "state_sha256":hash(&serde_json::to_vec(&state).map_err(|e|e.to_string())?),
        "observer_sha256":hash(&serde_json::to_vec(&observer).map_err(|e|e.to_string())?),
        "price_book_sha256":book_digest(&state),"output_chain_sha256":chain,
        "output_file_sha256":hash(&fs::read(&args[4]).map_err(|e|e.to_string())?),
        "restored_from_line":restored,"lines":state.lines,"snapshots":state.snapshots,"epochs":observer.next_epoch,
        "frontier":state.frontier.map(|s|s.to_string()),"pending":state.pending.len(),"applied_records":observer.applied_records,
        "delayed_records":observer.delayed_records,"model_events":observer.model_events,"completed_blocks":observer.completed,
        "completed_without_geometry":observer.completed_without_geometry,"unfinished_domain_exits":observer.ended_active,
        "barriers":observer.barriers,"native_effects":observer.native_effects,"normalizations":observer.normalizations,
        "noops":observer.noops,"shape_chain_sha256":observer.shape_chain,
        "active_at_end":observer.active.as_ref().map(|b|b.view(true)).transpose()?,"eof_confirms_tail":false,
        "scope":"one historical Coinbase profile; native logical path published at capture availability; no F2, prediction or execution qualification"});
    save_json(Path::new(&args[5]), &report)?;
    println!(
        "{}",
        json!({"profile":PROFILE,"lines":state.lines,"model_events":observer.model_events,"completed":observer.completed,"barriers":observer.barriers,"report":args[5]})
    );
    Ok(())
}
fn main() {
    if let Err(e) = run_native() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
