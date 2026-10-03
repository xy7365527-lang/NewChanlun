// #1467/#1468：接收前缀发布状态的具名研究适配，不是原生序号路径或交易价格。
#[path = "directional_change.rs"]
mod dc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

type R<T> = Result<T, String>;
const PROFILE: &str = "rw-dc-received-prefix/1";
const SCALE: u64 = 100_000_000;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn create(path: &Path) -> R<fs::File> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())
}
fn write_json(path: &Path, value: &impl Serialize) -> R<()> {
    let mut file = create(path)?;
    file.write_all(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
fn positive_integer(s: &str) -> R<i64> {
    let n = s.parse::<i64>().map_err(|_| "invalid exact integer")?;
    if n <= 0 || n.to_string() != s {
        return Err("noncanonical or nonpositive integer".into());
    }
    Ok(n)
}
fn capture_key(s: &str) -> R<(u32, u64)> {
    if s.len() < 20 || !s.is_ascii() || !s.ends_with('Z') || &s[10..11] != "T" {
        return Err("unsupported capture timestamp".into());
    }
    let date = s[..10]
        .replace('-', "")
        .parse()
        .map_err(|_| "invalid capture date")?;
    let h: u64 = s[11..13].parse().map_err(|_| "invalid hour")?;
    let m: u64 = s[14..16].parse().map_err(|_| "invalid minute")?;
    let sec: u64 = s[17..19].parse().map_err(|_| "invalid second")?;
    let tail = &s[19..s.len() - 1];
    let f = if tail.is_empty() {
        ""
    } else {
        tail.strip_prefix('.').ok_or("invalid fraction")?
    };
    if h >= 24 || m >= 60 || sec >= 60 || f.len() > 9 || !f.bytes().all(|x| x.is_ascii_digit()) {
        return Err("capture timestamp outside domain".into());
    }
    let ns = if f.is_empty() {
        0
    } else {
        f.parse::<u64>().map_err(|_| "invalid fraction")? * 10u64.pow(9 - f.len() as u32)
    };
    Ok((date, (h * 3600 + m * 60 + sec) * 1_000_000_000 + ns))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Quote {
    bid_price: String,
    ask_price: String,
    bid_quantity: String,
    ask_quantity: String,
    scale: u64,
}
impl Quote {
    fn values(&self) -> R<(i64, i64, i64, i64)> {
        if self.scale != SCALE {
            return Err("unsupported quote scale".into());
        }
        let b = positive_integer(&self.bid_price)?;
        let a = positive_integer(&self.ask_price)?;
        let qb = positive_integer(&self.bid_quantity)?;
        let qa = positive_integer(&self.ask_quantity)?;
        if b >= a {
            return Err("ready quote is not strictly two-sided".into());
        }
        Ok((b, a, qb, qa))
    }
    fn project(&self) -> R<(i64, i64)> {
        let (b, a, qb, qa) = self.values()?;
        let n = (a as i128)
            .checked_mul(qb as i128)
            .and_then(|x| {
                (b as i128)
                    .checked_mul(qa as i128)
                    .and_then(|y| x.checked_add(y))
            })
            .ok_or("projection numerator overflow")?;
        let d = qb as i128 + qa as i128;
        let rounded = n
            .checked_mul(2)
            .and_then(|x| x.checked_add(d))
            .ok_or("projection rounding overflow")?
            / (2 * d);
        let weighted = i64::try_from(rounded).map_err(|_| "projection outside i64")?;
        let mid =
            i64::try_from((b as i128 + a as i128 + 1) / 2).map_err(|_| "midpoint outside i64")?;
        if weighted < b || weighted > a || mid < b || mid > a {
            return Err("projection escaped spread".into());
        }
        Ok((weighted, mid))
    }
}

#[derive(Deserialize)]
struct View {
    schema: String,
    line: usize,
    product_id: String,
    status: String,
    transition: String,
    available_at_capture: Option<String>,
    native_type: String,
    frontier: Option<String>,
    quote: Option<Quote>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Observation {
    index: usize,
    source_line: usize,
    capture: String,
    frontier: String,
    quote: Quote,
    weighted: String,
    midpoint: String,
    kind: String,
    native_type: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Epoch {
    id: usize,
    observations: Vec<Observation>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Saved {
    rows_seen: usize,
    next_epoch: usize,
    observations_total: usize,
    weighted_total: usize,
    midpoint_total: usize,
    ended_epochs: usize,
    reference_checks: usize,
    epoch: Option<Epoch>,
    last_capture: Option<(u32, u64)>,
    weighted_confirmation_kinds: BTreeMap<String, usize>,
    midpoint_confirmation_kinds: BTreeMap<String, usize>,
}
#[derive(Serialize, Deserialize)]
struct Checkpoint {
    profile: String,
    source_sha256: String,
    executable_sha256: String,
    delta: i64,
    payload_sha256: String,
    engine_sha256: String,
    output_chain_sha256: String,
    saved: Saved,
}

fn engine_hash(engines: &Option<(dc::Constructor, dc::Constructor)>) -> String {
    hash(format!("{engines:?}").as_bytes())
}
fn rebuild(saved: &Saved, delta: i64) -> R<Option<(dc::Constructor, dc::Constructor)>> {
    let Some(e) = &saved.epoch else {
        return Ok(None);
    };
    if e.observations.is_empty() || e.observations.len() > 5000 {
        return Err("invalid checkpoint epoch length".into());
    }
    let (mut w, mut m) = (dc::Constructor::new(delta), dc::Constructor::new(delta));
    let mut prior = 0usize;
    for (i, o) in e.observations.iter().enumerate() {
        if o.index != i || o.source_line <= prior || o.source_line > saved.rows_seen {
            return Err("invalid observation provenance".into());
        }
        let (wv, mv) = o.quote.project()?;
        if o.weighted != wv.to_string() || o.midpoint != mv.to_string() {
            return Err("saved projection differs from quote".into());
        }
        w.push(wv);
        m.push(mv);
        prior = o.source_line;
    }
    Ok(Some((w, m)))
}
fn point(e: &Epoch, p: dc::Point) -> Value {
    let o = &e.observations[p.index];
    json!({"observation":p.index,"source_line":o.source_line,"capture_time_lower_bound":o.capture,
        "frontier":o.frontier,"value_ticks":p.value.to_string()})
}
fn at(e: &Epoch, i: usize) -> Value {
    let o = &e.observations[i];
    json!({"observation":i,"source_line":o.source_line,"capture_time_lower_bound":o.capture})
}
fn unit(e: &Epoch, c: &dc::Constructor, i: usize) -> Value {
    let u = c.units[i];
    let first = if i == 0 {
        c.initialized_at.expect("completed without initialization")
    } else {
        c.units[i - 1].known_at
    };
    json!({"epoch":e.id,"ordinal":i,"direction":if u.direction==dc::Direction::Up{"up"}else{"down"},
        "start":point(e,u.start),"end":point(e,u.end),"active_since":at(e,first),"confirmed_at":at(e,u.known_at)})
}
fn active(e: &Epoch, c: &dc::Constructor) -> Value {
    let since = c
        .units
        .last()
        .map(|u| u.known_at)
        .or(c.initialized_at)
        .unwrap_or(0);
    match c.active {
        None => Value::Null,
        Some(dc::Active::Undecided { low, high }) => {
            json!({"phase":"undecided","provisional":true,"active_since":at(e,0),"low":point(e,low),"high":point(e,high)})
        }
        Some(dc::Active::Up { start, peak }) => {
            json!({"phase":"up","provisional":true,"active_since":at(e,since),"start":point(e,start),"extreme":point(e,peak)})
        }
        Some(dc::Active::Down { start, trough }) => {
            json!({"phase":"down","provisional":true,"active_since":at(e,since),"start":point(e,start),"extreme":point(e,trough)})
        }
    }
}
fn checkpoint(
    path: &Path,
    source: &str,
    exe: &str,
    delta: i64,
    chain: &str,
    saved: &Saved,
    engines: &Option<(dc::Constructor, dc::Constructor)>,
) -> R<()> {
    write_json(
        path,
        &Checkpoint {
            profile: PROFILE.into(),
            source_sha256: source.into(),
            executable_sha256: exe.into(),
            delta,
            payload_sha256: hash(&serde_json::to_vec(saved).map_err(|e| e.to_string())?),
            engine_sha256: engine_hash(engines),
            output_chain_sha256: chain.into(),
            saved: saved.clone(),
        },
    )
}
fn reference_check(c: &dc::Constructor) -> R<()> {
    if dc::reference(&c.history, c.delta) != c.units {
        return Err("completed units differ from first-hit reference".into());
    }
    let (initial, tail) = dc::reference_active(&c.history, c.delta);
    if initial != c.initialized_at || tail != c.active {
        return Err("active state differs from first-hit reference".into());
    }
    dc::audit(&c.history, c.delta, &c.units);
    Ok(())
}

fn run() -> R<()> {
    let args: Vec<_> = std::env::args().collect();
    if !(args.len() == 7 || args.len() == 8) || !["run", "resume"].contains(&args[1].as_str()) {
        return Err("usage: run|resume INPUT_VIEWS CHECKPOINT_DIR|FILE OUTPUT_ROWS REPORT DELTA_TICKS [STOP_LINE]".into());
    }
    let delta = args[6].parse::<i64>().map_err(|_| "invalid delta")?;
    if delta <= 0 {
        return Err("delta must be positive".into());
    }
    let stop = args
        .get(7)
        .map(|s| s.parse::<usize>().map_err(|_| "invalid stop".to_string()))
        .transpose()?;
    let input = Path::new(&args[2]);
    if fs::metadata(input).map_err(|e| e.to_string())?.len() > 100 * 1024 * 1024 {
        return Err("input exceeds budget".into());
    }
    let source = hash(&fs::read(input).map_err(|e| e.to_string())?);
    let exe = hash(
        &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    let fresh = args[1] == "run";
    let (mut saved, mut chain, mut engines) = if fresh {
        fs::create_dir(&args[3]).map_err(|e| e.to_string())?;
        (
            Saved::default(),
            hash(format!("{PROFILE}:{delta}").as_bytes()),
            None,
        )
    } else {
        let cp: Checkpoint =
            serde_json::from_slice(&fs::read(&args[3]).map_err(|e| e.to_string())?)
                .map_err(|_| "invalid checkpoint")?;
        if cp.profile != PROFILE || cp.source_sha256 != source || cp.executable_sha256 != exe {
            return Err("checkpoint source/executable/profile mismatch".into());
        }
        if cp.delta != delta {
            return Err("checkpoint delta mismatch".into());
        }
        if hash(&serde_json::to_vec(&cp.saved).map_err(|e| e.to_string())?) != cp.payload_sha256 {
            return Err("checkpoint payload mismatch".into());
        }
        let en = rebuild(&cp.saved, delta)?;
        if engine_hash(&en) != cp.engine_sha256 {
            return Err("replayed engine differs from checkpoint".into());
        }
        (cp.saved, cp.output_chain_sha256, en)
    };
    let restored = saved.rows_seen;
    if stop.is_some_and(|s| s < restored) {
        return Err("stop before restored position".into());
    }
    let mut rows = BufWriter::new(create(Path::new(&args[4]))?);
    if fresh {
        checkpoint(
            &Path::new(&args[3]).join("empty.json"),
            &source,
            &exe,
            delta,
            &chain,
            &saved,
            &engines,
        )?;
    }
    let mut input_rows = BufReader::new(fs::File::open(input).map_err(|e| e.to_string())?).lines();
    for _ in 0..restored {
        input_rows
            .next()
            .ok_or("checkpoint past input")?
            .map_err(|e| e.to_string())?;
    }
    while stop.is_none_or(|s| saved.rows_seen < s) {
        let Some(line) = input_rows.next() else { break };
        let view: View = serde_json::from_str(&line.map_err(|e| e.to_string())?)
            .map_err(|_| "invalid view JSON")?;
        if view.schema != "coinbase-causal-view/1"
            || view.product_id != "BTC-USD"
            || view.line != saved.rows_seen + 1
        {
            return Err("view identity or continuity mismatch".into());
        }
        if ![
            "ready",
            "awaiting_snapshot",
            "sequence_gap",
            "missing_quote",
            "crossed_or_locked_quote",
            "disconnected",
        ]
        .contains(&view.status.as_str())
        {
            return Err("unknown view status".into());
        }
        if let Some(t) = &view.available_at_capture {
            let key = capture_key(t)?;
            if saved.last_capture.is_some_and(|old| key < old) {
                return Err("capture clock decreased".into());
            }
            saved.last_capture = Some(key);
        }
        saved.rows_seen = view.line;
        let ready = view.status == "ready";
        if ready != (view.quote.is_some()) {
            return Err("view status/quote conflict".into());
        }
        let restart = ["snapshot_batch", "catchup_batch"].contains(&view.transition.as_str());
        let mut ended = Value::Null;
        if saved.epoch.is_some() && (!ready || restart) {
            let e = saved.epoch.as_ref().unwrap();
            let (w, m) = engines.as_ref().unwrap();
            ended = json!({"epoch":e.id,"reason":if ready {view.transition.as_str()}else{view.status.as_str()},
                "tail_status":"unfinished_on_domain_exit","weighted_active":active(e,w),"midpoint_active":active(e,m)});
            saved.epoch = None;
            engines = None;
            saved.ended_epochs += 1;
        }
        let mut observation = Value::Null;
        let mut new_w = Vec::new();
        let mut new_m = Vec::new();
        if ready {
            let quote = view.quote.clone().ok_or("missing ready quote")?;
            let capture = view
                .available_at_capture
                .clone()
                .ok_or("ready state lacks capture time")?;
            let frontier = view.frontier.clone().ok_or("ready state lacks frontier")?;
            frontier.parse::<u64>().map_err(|_| "invalid frontier")?;
            let (wv, mv) = quote.project()?;
            if saved.epoch.is_none() {
                let id = saved.next_epoch;
                saved.next_epoch += 1;
                saved.epoch = Some(Epoch {
                    id,
                    observations: Vec::new(),
                });
                engines = Some((dc::Constructor::new(delta), dc::Constructor::new(delta)));
            }
            let e = saved.epoch.as_mut().unwrap();
            let changed = e.observations.last().is_none_or(|last| last.quote != quote);
            if changed {
                if e.observations.len() >= 5000 {
                    return Err("epoch observation budget reached".into());
                }
                let kind = match e.observations.last() {
                    None => "initial",
                    Some(last)
                        if last.quote.bid_price == quote.bid_price
                            && last.quote.ask_price == quote.ask_price =>
                    {
                        "quantity_only"
                    }
                    Some(_) => "price_change",
                };
                let o = Observation {
                    index: e.observations.len(),
                    source_line: view.line,
                    capture,
                    frontier,
                    quote,
                    weighted: wv.to_string(),
                    midpoint: mv.to_string(),
                    kind: kind.into(),
                    native_type: view.native_type.clone(),
                };
                observation = serde_json::to_value(&o).map_err(|e| e.to_string())?;
                e.observations.push(o);
                let (w, m) = engines.as_mut().unwrap();
                let wi = w.units.len();
                let mi = m.units.len();
                w.push(wv);
                m.push(mv);
                reference_check(w)?;
                reference_check(m)?;
                saved.reference_checks += 1;
                saved.observations_total += 1;
                for i in wi..w.units.len() {
                    new_w.push(unit(e, w, i));
                }
                for i in mi..m.units.len() {
                    new_m.push(unit(e, m, i));
                }
                saved.weighted_total += new_w.len();
                saved.midpoint_total += new_m.len();
                let key = format!("{kind}:{}", view.native_type);
                if !new_w.is_empty() {
                    *saved
                        .weighted_confirmation_kinds
                        .entry(key.clone())
                        .or_default() += new_w.len();
                }
                if !new_m.is_empty() {
                    *saved.midpoint_confirmation_kinds.entry(key).or_default() += new_m.len();
                }
            }
        }
        let (epoch_id, weighted_active, midpoint_active) =
            if let (Some(e), Some((w, m))) = (&saved.epoch, &engines) {
                (Some(e.id), active(e, w), active(e, m))
            } else {
                (None, Value::Null, Value::Null)
            };
        let out = json!({"profile":PROFILE,"source_line":view.line,"capture_time_lower_bound":view.available_at_capture,
            "status":view.status,"epoch":epoch_id,"delta_ticks":delta.to_string(),"scale":SCALE,
            "value_kind":"untradable_quote_observations","new_observation":observation,
            "weighted_confirmed":new_w,"midpoint_confirmed":new_m,"weighted_active":weighted_active,
            "midpoint_active":midpoint_active,"ended_scope":ended});
        let mut bytes = serde_json::to_vec(&out).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let mut link = chain.as_bytes().to_vec();
        link.extend_from_slice(&bytes);
        chain = hash(&link);
        rows.write_all(&bytes).map_err(|e| e.to_string())?;
        if fresh && [339, 340, 1000, 10000].contains(&view.line) {
            checkpoint(
                &Path::new(&args[3]).join(format!("line-{}.json", view.line)),
                &source,
                &exe,
                delta,
                &chain,
                &saved,
                &engines,
            )?;
        }
    }
    rows.flush().map_err(|e| e.to_string())?;
    drop(rows);
    let fingerprint = engine_hash(&engines);
    let state_hash = hash(&serde_json::to_vec(&(&saved, &fingerprint)).map_err(|e| e.to_string())?);
    let active_end = if let (Some(e), Some((w, m))) = (&saved.epoch, &engines) {
        json!({"epoch":e.id,"weighted":active(e,w),"midpoint":active(e,m)})
    } else {
        Value::Null
    };
    let report = json!({"profile":PROFILE,"source_sha256":source,"executable_sha256":exe,"delta_ticks":delta.to_string(),"scale":SCALE,
        "state_sha256":state_hash,"engine_sha256":fingerprint,"output_chain_sha256":chain,
        "output_file_sha256":hash(&fs::read(&args[4]).map_err(|e|e.to_string())?),"restored_from_line":restored,
        "rows_seen":saved.rows_seen,"rows_emitted":saved.rows_seen-restored,"epochs_started":saved.next_epoch,"ended_epochs":saved.ended_epochs,
        "observations":saved.observations_total,"weighted_completed":saved.weighted_total,"midpoint_completed":saved.midpoint_total,
        "reference_prefix_checks":saved.reference_checks,"weighted_confirmation_kinds":saved.weighted_confirmation_kinds,
        "midpoint_confirmation_kinds":saved.midpoint_confirmation_kinds,"active_at_end":active_end,
        "eof_does_not_confirm_tail":true,"scope":"one observed-state clock; midpoint is not trade baseline A; no F2 qualification or value claim"});
    write_json(Path::new(&args[5]), &report)?;
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
