//! 已核 Q 的关系索引。仅 owner 可从同快照 cold 结果建立，固定 effects 驱动更新。
use super::super::{
    audit_retention::{sum, Retained},
    business_json as bj,
    request_audit::{ExactRows, SqlCell},
};
use super::effects::{integer, optional_integer, text, Change, Effects, Operation, Table};
use super::*;
use newchan_rust::sqlite_checked::CheckedStatement;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) struct Facts {
    clock: v2::ClockPlan,
    sid: String,
    inc: String,
    raw_tails: HashMap<String, (i64, i64)>,
    coordinates: HashMap<i64, String>,
    pending: BTreeMap<String, (i64, i64)>, // event -> (rowid, attempt_count)
    frontiers: Vec<(i64, i64)>,
    next: i64,
    max_ns: i64,
    epoch_count: i64,
    epoch_to: Option<i64>,
    epoch_generation: i64,
}
impl Retained for Facts {
    fn heap_bytes(&self) -> usize {
        sum([
            self.clock.value.heap_bytes(),
            self.clock.hash.heap_bytes(),
            self.sid.heap_bytes(),
            self.inc.heap_bytes(),
            self.raw_tails.heap_bytes(),
            self.coordinates.heap_bytes(),
            self.pending.heap_bytes(),
            self.frontiers.heap_bytes(),
        ])
    }
}
impl Facts {
    pub(super) fn cold(
        tx: &Transaction<'_>,
        originals: &[ExactRows],
        state: &state::AuditedState,
    ) -> Result<Self, String> {
        let protocol = &effects::table(originals, Table::Protocol)?.rows[0];
        let mut facts = Self {
            clock: v2::stored_clock(tx)?,
            sid: meta_get_opt(tx, "session_id")?.ok_or_else(|| invalid("缺session"))?,
            inc: text(&protocol[3])?.into(),
            raw_tails: Default::default(),
            coordinates: Default::default(),
            pending: Default::default(),
            frontiers: state
                .cuts
                .iter()
                .map(|c| (c.metadata.generation, c.metadata.input_frontier))
                .collect(),
            next: integer(&protocol[6])?,
            max_ns: parse_canonical_i64(text(&protocol[7])?, "phase frontier")?,
            epoch_count: 0,
            epoch_to: None,
            epoch_generation: 0,
        };
        for r in state.raw.view().iter() {
            let id = bj::text(r, "identity_key")?;
            let coord = bj::coord(r)?;
            facts.raw_tails.insert(
                id.into(),
                (
                    coord,
                    bj::field(r, "revision")
                        .as_i64()
                        .ok_or_else(|| invalid("revision"))?,
                ),
            );
            facts.coordinates.insert(coord, id.into());
        }
        for m in &effects::table(originals, Table::Message)?.rows {
            if text(&m[8])? == "AcceptedPending" {
                facts
                    .pending
                    .insert(text(&m[10])?.into(), (integer(&m[0])?, integer(&m[11])?));
            }
        }
        if let Some(e) = effects::table(originals, Table::Epoch)?.rows.last() {
            facts.epoch_count = integer(&e[1])?;
            facts.epoch_to = Some(parse_writer_epoch(text(&e[3])?, "epoch.to")?);
            facts.epoch_generation = parse_canonical_i64(text(&e[4])?, "epoch.generation")?;
        }
        Ok(facts)
    }
    /// effects 已回读并保留 old/new；先闭合所有关系，随后才允许新 cut 消费。
    pub(super) fn advance(
        &mut self,
        tx: &Transaction<'_>,
        effects: &Effects,
        audited: &mut state::AuditedState,
    ) -> Result<(), String> {
        let old_g = audited.cuts.last().map_or(0, |c| c.metadata.generation);
        let root_g = meta_i64(tx, "generation")?;
        let publish = effects.operation == Operation::Publish;
        if root_g != old_g + i64::from(publish) {
            return Err(invalid("固定写口改变了未声明的根代际"));
        }
        for change in &effects.changes {
            self.preservation(change, effects.operation, root_g)?;
        }
        self.raw(tx, effects, audited)?;
        let (next,frontier):(i64,String) = tx.query_row("SELECT next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?))).map_err(invalid)?;
        if next != self.next + i64::from(effects.operation == Operation::Begin) {
            return Err(invalid("phase frontier/attempt序列断裂"));
        }
        if publish {
            let deltas: Vec<_> = effects.rows(Table::Delta).collect();
            let pubs: Vec<_> = effects.rows(Table::Publication).collect();
            if deltas.len() != 1 || pubs.len() != 1 {
                return Err(invalid("逐代 publication/Delta 未闭合"));
            }
            let d = new(deltas[0])?;
            let p = new(pubs[0])?;
            if integer(&d[1])? != root_g {
                return Err(invalid("Delta 不是连续新代"));
            }
            let lengths = [&p[3], &p[4], &p[5]].map(|c| {
                if let SqlCell::Blob(b) = c {
                    i64::try_from(b.len()).ok()
                } else {
                    None
                }
            });
            control_audit::validate_publication_pair(
                integer(&p[1])?,
                root_g,
                text(&p[2])?,
                Some(text(&d[7])?),
                lengths,
            )?;
            let f = integer(&d[8])?;
            if self.frontiers.last().is_some_and(|(_, old)| *old > f) {
                return Err(invalid("发布前沿倒退"));
            }
            self.frontiers.push((root_g, f));
            self.delivery(effects, root_g)?;
        }
        let mut affected = BTreeSet::new();
        for c in effects.rows(Table::Message) {
            affected.insert((text(&new(c)?[10])?.to_owned(), c.rowid));
        }
        // 包含没有被 UPDATE 命中的旧 pending attempt；根前移同样改变其 Begin 目标。
        if publish {
            for (event, (rowid, count)) in &self.pending {
                if *count > 0 {
                    affected.insert((event.clone(), *rowid));
                }
            }
        }
        let raw_insertions = effects.rows(Table::Raw).count();
        let mut covered_new = BTreeSet::new();
        for (event, rowid) in &affected {
            let m = read_message(tx, *rowid)?;
            let first = self
                .frontiers
                .get(self.frontiers.partition_point(|(_, f)| *f < m.seq))
                .map(|(g, _)| *g);
            let key = v2::validate_audit_message(
                tx,
                &m,
                PROTOCOL_REVISION,
                &self.sid,
                &self.inc,
                (&self.clock.value).into(),
                first,
                None,
            )?;
            let old_raw_len = audited.raw.len() - raw_insertions;
            if m.seq >= old_raw_len as i64 {
                covered_new.insert(m.seq);
            }
            self.attempts(tx, &m, key.as_str(), root_g, next)?;
            if m.published.is_none() {
                self.pending.insert(event.clone(), (*rowid, m.count));
            } else {
                self.pending.remove(event);
            }
        }
        if covered_new.len() != raw_insertions {
            return Err(invalid("v2原始接纳序缺持久消息承接"));
        }
        let mut begins = 0;
        let mut commits = 0;
        let mut recovers = Vec::new();
        for change in effects.rows(Table::Phase) {
            let row = new(change)?;
            let id = text(&row[1])?;
            let phase = text(&row[2])?;
            let index = integer(&row[3])?;
            let ns = text(&row[4])?;
            let key = text(&row[5])?;
            let ord = optional_integer(&row[6])?;
            let g = integer(&row[7])?;
            if phase == "recover" {
                v2::validate_audit_phase(
                    id,
                    phase,
                    index,
                    ns,
                    key,
                    ord,
                    g,
                    root_g,
                    next,
                    (&self.clock.value).into(),
                    None,
                )?;
                recovers.push((ns.to_owned(), g));
            } else if !affected.iter().any(|(e, _)| e == id) {
                return Err(invalid("新 phase 未进入实际消息影响集"));
            }
            if phase == "begin" {
                begins += 1;
                if ord != Some(self.next) {
                    return Err(invalid("全局 Begin ordinal 非连续尾"));
                }
            }
            if phase == "commit" {
                commits += 1;
                if g != root_g {
                    return Err(invalid("Commit 代际错误"));
                }
            }
            self.max_ns = self.max_ns.max(newchan_rust::session_protocol::nonnegative(
                ns,
                "semantic_ns",
            )?);
        }
        if begins != usize::from(effects.operation == Operation::Begin)
            || commits != usize::from(publish)
            || parse_canonical_i64(&frontier, "logical_phase_frontier")? != self.max_ns
        {
            return Err(invalid("phase frontier/attempt序列断裂"));
        }
        let mut epochs = Vec::new();
        for change in effects.rows(Table::Epoch) {
            let e = new(change)?;
            let ordinal = integer(&e[1])?;
            let from = parse_writer_epoch(text(&e[2])?, "epoch.from")?;
            let to = parse_writer_epoch(text(&e[3])?, "epoch.to")?;
            let at = newchan_rust::session_protocol::nonnegative(text(&e[4])?, "epoch.generation")?;
            let ns = text(&e[6])?;
            newchan_rust::session_protocol::nonnegative(ns, "epoch.transitioned_at")?;
            v2::validate_epoch_step(
                ordinal,
                self.epoch_count + 1,
                from,
                to,
                at,
                text(&e[5])?,
                self.epoch_to,
                self.epoch_generation,
                root_g,
                true,
            )?;
            self.epoch_count = ordinal;
            self.epoch_to = Some(to);
            self.epoch_generation = at;
            epochs.push((ns.to_owned(), at));
        }
        if recovers != epochs
            || recovers.len() != usize::from(effects.operation == Operation::Recover)
        {
            return Err(invalid("recover phase与writer换代时间/代际不闭合"));
        }
        let writer = parse_writer_epoch(
            &meta_get_opt(tx, "writer_epoch")?.ok_or_else(|| invalid("缺writer_epoch"))?,
            "writer_epoch",
        )?;
        if writer < 1 || self.epoch_to.is_some_and(|e| e != writer) {
            return Err(invalid("writer_epoch与最后换代结果不一致"));
        }
        self.next = next;
        Ok(())
    }
    fn raw(
        &mut self,
        tx: &Transaction<'_>,
        effects: &Effects,
        audited: &mut state::AuditedState,
    ) -> Result<(), String> {
        let pairs: Vec<_> = effects.rows(Table::Ohlc).collect();
        let mut paired = BTreeSet::new();
        let bj::RawEvents::Plain(raw) = &mut audited.raw else {
            return Err(invalid("增量 Q 不接受 Owned 原始账"));
        };
        for change in effects.rows(Table::Raw) {
            let r = new(change)?;
            let seq = integer(&r[6])?;
            if seq != raw.len() as i64 {
                return Err(invalid("raw seq 不是已核连续尾"));
            }
            let pair = pairs
                .iter()
                .find(|p| p.new.as_ref().is_some_and(|p| p[1] == r[1] && p[2] == r[2]))
                .ok_or_else(|| invalid("raw/OHLC 配对键缺失"))?;
            if !paired.insert(pair.rowid) {
                return Err(invalid("raw/OHLC 配对重复"));
            }
            let value = read_raw_event_at(tx, seq)?;
            let view = (&value).into();
            let id = bj::text(view, "identity_key")?;
            let coord = bj::coord(view)?;
            let previous = self.raw_tails.get(id);
            bj::validate_raw_coordinate(
                id,
                coord,
                self.coordinates.get(&coord).map(String::as_str),
                previous.map(|(c, _)| *c),
            )?;
            bj::validate_raw_row(view, raw.len(), previous.map(|(_, r)| *r), None)?;
            self.raw_tails.insert(
                id.into(),
                (coord, bj::field(view, "revision").as_i64().unwrap()),
            );
            self.coordinates.insert(coord, id.into());
            raw.push(value);
        }
        if paired.len() != pairs.len() {
            return Err(invalid("OHLC/raw 键集未闭合"));
        }
        Ok(())
    }
    fn attempts(
        &self,
        tx: &Transaction<'_>,
        m: &v2::AuditMessage,
        key: &str,
        root: i64,
        next: i64,
    ) -> Result<(), String> {
        let mut stmt=tx.prepare("SELECT phase,attempt_index,semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events WHERE clock_event_id=?1 ORDER BY phase,attempt_index").map_err(invalid)?;
        let mut rows = stmt.query([m.event.as_str()]).map_err(invalid)?;
        let mut accept = false;
        let mut begins = BTreeMap::new();
        let mut commits = BTreeMap::new();
        while let Some(r) = rows.next().map_err(invalid)? {
            let phase: String = r.get(0).map_err(invalid)?;
            let index: i64 = r.get(1).map_err(invalid)?;
            let ns: String = r.get(2).map_err(invalid)?;
            let actual_key: String = r.get(3).map_err(invalid)?;
            let ord: Option<i64> = r.get(4).map_err(invalid)?;
            let g: i64 = r.get(5).map_err(invalid)?;
            v2::validate_audit_phase(
                m.event.as_str(),
                &phase,
                index,
                &ns,
                &actual_key,
                ord,
                g,
                root,
                next,
                (&self.clock.value).into(),
                Some((key, m.count, m.published)),
            )?;
            match phase.as_str() {
                "accept" => accept = true,
                "begin" => {
                    begins.insert(index, (ord, g));
                }
                "commit" => {
                    commits.insert(index, (ord, g));
                }
                _ => {}
            }
        }
        let indexes: Vec<_> = begins.keys().copied().collect();
        v2::validate_attempt_summary(
            m.count,
            m.published,
            accept,
            &indexes,
            commits.iter().all(|(i, c)| begins.get(i) == Some(c)),
            commits.contains_key(&(m.count - 1)),
        )?;
        Ok(())
    }
    fn delivery(&self, effects: &Effects, g: i64) -> Result<(), String> {
        let policies: Vec<_> = effects.rows(Table::Policy).collect();
        if policies.len() != 1 {
            return Err(invalid("缺 delivery policy 效果"));
        }
        let old = old(policies[0])?;
        let new = new(policies[0])?;
        let keep = integer(&new[5])?;
        let first = 1.max(g - keep + 1);
        if new[1..6] != old[1..6] {
            return Err(invalid("投递静态策略改变"));
        }
        v2::validate_delivery_policy(
            text(&new[2])?,
            text(&new[3])?,
            text(&new[4])?,
            keep,
            integer(&new[6])?,
            integer(&new[7])?,
            &self.sid,
            &self.inc,
            g,
        )?;
        let mut removed = Vec::new();
        let mut added = Vec::new();
        for c in effects.rows(Table::Delivery) {
            match (&c.old, &c.new) {
                (None, Some(n)) => added.push(integer(&n[1])?),
                (Some(o), None) => removed.push(integer(&o[1])?),
                _ => return Err(invalid("delivery 非追加/删除效果")),
            }
        }
        if added != [g]
            || removed.len() as i64 != (first - integer(&old[6])?).max(0)
            || removed
                .iter()
                .enumerate()
                .any(|(i, n)| *n != integer(&old[6]).unwrap() + i as i64)
        {
            return Err(invalid("普通投递引用缺失/未来/越保留界"));
        }
        Ok(())
    }
    fn preservation(&self, c: &Change, op: Operation, g: i64) -> Result<(), String> {
        use Table::*;
        match c.table {
            Raw | Ohlc | Phase | Epoch | Delta | Publication | Node | Witness | Relation
            | Observation => {
                if c.old.is_some() || c.new.is_none() {
                    return Err(invalid("追加关系出现覆盖/删除"));
                }
            }
            Object => {
                let n = new(c)?;
                if let Some(o) = &c.old {
                    if o[..16] != n[..16]
                        || !matches!(o[16], SqlCell::Null)
                        || integer(&n[16])? != g
                    {
                        return Err(invalid("对象效果不保持旧身份/出生/原件"));
                    }
                } else if integer(&n[13])? != g || integer(&n[15])? != g {
                    return Err(invalid("对象新生非本代"));
                }
            }
            Message => {
                let n = new(c)?;
                if let Some(o) = &c.old {
                    match op {
                        Operation::Begin
                            if o[..11] == n[..11] && integer(&n[11])? == integer(&o[11])? + 1 => {}
                        Operation::Publish
                            if o[..8] == n[..8]
                                && o[10..] == n[10..]
                                && text(&o[8])? == "AcceptedPending"
                                && text(&n[8])? == "Committed"
                                && matches!(o[9], SqlCell::Null)
                                && integer(&n[9])? == g => {}
                        _ => return Err(invalid("消息效果改变旧 envelope/非声明列")),
                    }
                } else if op != Operation::Accept {
                    return Err(invalid("非 Accept 新消息"));
                }
            }
            Protocol => {
                let o = old(c)?;
                let n = new(c)?;
                if o[..6] != n[..6] {
                    return Err(invalid("固定写口改变协议/clock 上下文"));
                }
            }
            Meta => {
                let n = new(c)?;
                let key = text(&n[1])?;
                let allowed = match op {
                    Operation::Begin => [
                        "advance_state",
                        "begin_token",
                        "begin_generation",
                        "begin_input_frontier",
                    ]
                    .contains(&key),
                    Operation::Publish => [
                        "generation",
                        "structure_cut",
                        "index_frontier",
                        "last_advance_frontier",
                        "advance_state",
                    ]
                    .contains(&key),
                    Operation::Recover => [
                        "writer_epoch",
                        "advance_state",
                        "begin_token",
                        "last_recovered_begin",
                    ]
                    .contains(&key),
                    Operation::Accept => false,
                };
                if !allowed || c.old.as_ref().is_some_and(|o| o[..2] != n[..2]) {
                    return Err(invalid("固定写口改变共同上下文"));
                }
            }
            Catalog => {
                let o = old(c)?;
                let n = new(c)?;
                if o[..6] != n[..6] {
                    return Err(invalid("目录静态关系改变"));
                }
                state::validate_unrun_row(
                    text(&n[1])?,
                    text(&n[6])?,
                    text(&n[7])?,
                    text(&n[8])?,
                    text(&n[9])?,
                    false,
                )?;
            }
            Policy | Delivery => {}
        }
        if matches!(c.table, Witness | Relation | Observation) {
            let n = new(c)?;
            let column = match c.table {
                Witness => 7,
                Relation => 6,
                Observation => 7,
                _ => unreachable!(),
            };
            if integer(&n[column])? != g {
                return Err(invalid("辅助身份 first 非本代"));
            }
        }
        Ok(())
    }
}
fn read_message(tx: &Transaction<'_>, rowid: i64) -> Result<v2::AuditMessage, String> {
    let mut s=CheckedStatement::prepare(tx,c"SELECT source_namespace,source_epoch,message_id,payload_hash,canonical_envelope,receipt_id,accepted_seq,status,first_published_generation,clock_event_id,attempt_count FROM s_input_messages WHERE rowid=?1").map_err(invalid)?;
    s.bind_integer(1, rowid).map_err(invalid)?;
    let result = v2::read_audit_message(
        &mut s
            .next()
            .map_err(invalid)?
            .ok_or_else(|| invalid("缺实际 message"))?,
        None,
    )?;
    s.finalize().map_err(invalid)?;
    Ok(result)
}
fn new(c: &Change) -> Result<&[SqlCell], String> {
    c.new.as_deref().ok_or_else(|| invalid("缺实际新行"))
}
fn old(c: &Change) -> Result<&[SqlCell], String> {
    c.old.as_deref().ok_or_else(|| invalid("缺绑定旧行"))
}
