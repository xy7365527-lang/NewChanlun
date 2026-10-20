//! 已核 Q 的关系索引。仅 owner 可从同快照 cold 结果建立，固定 effects 驱动更新。
use super::super::{
    audit_retention::{sum, Retained},
    business_json as bj,
    request_audit::{ExactRows, SqlCell},
};
use super::effects::{integer, optional_integer, text, Change, Effects, Operation, Table};
use super::native::{Budget, Failure, JsonShape};
use super::*;
use crate::v3::audit_retention::{tree_slots, vec_bytes};
fn invalid(e: impl std::fmt::Display) -> Failure {
    Failure::Invalid(super::invalid(e))
}
use newchan_rust::sqlite_checked::CheckedStatement;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Default)]
struct RawCharges {
    manual: usize,
    native: usize,
}
pub(super) struct Facts {
    clock: native::Reserved<v2::ClockPlan>,
    sid: native::Reserved<String>,
    inc: native::Reserved<String>,
    raw_tails: native::Reserved<HashMap<native::Reserved<String>, (i64, i64)>>,
    coordinates: native::Reserved<HashMap<i64, native::Reserved<String>>>,
    pending: native::Tree<(i64, i64)>, // event -> (rowid, attempt_count)
    frontiers: native::Reserved<Vec<(i64, i64)>>,
    next: i64,
    max_ns: i64,
    epoch_count: i64,
    epoch_to: Option<i64>,
    epoch_generation: i64,
    retained: usize,
    raw_capacity: Option<native::Permit>,
    raw_fees: native::Reserved<Vec<native::Permit>>,
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
            vec_bytes::<native::Permit>(self.raw_fees.capacity()),
        ])
    }
}
impl Facts {
    /// #1488 受控 Recover 迁移：已核 writer_epoch_history 最后一次换代的 to（只读）。
    pub(super) fn last_epoch_to(&self) -> Option<i64> {
        self.epoch_to
    }
    pub(super) fn cold(
        tx: &Transaction<'_>,
        originals: &[ExactRows],
        state: &state::AuditedState,
        budget: &Budget,
    ) -> Result<Self, Failure> {
        let protocol = &effects::table(originals, Table::Protocol)?.rows[0];
        let clock_json = text(&protocol[5])?;
        let shape = JsonShape::scan(clock_json.as_bytes());
        let clock_permit = budget.reserve(sum([
            shape.parse_peak(),
            shape.canonical_peak(),
            clock_json.len(),
            text(&protocol[4])?.len(),
            64,
        ]))?;
        let clock = v2::stored_clock(tx)?;
        let clock = clock_permit
            .finish(sum([clock.value.heap_bytes(), clock.hash.heap_bytes()]))
            .hold(clock);
        let meta = effects::table(originals, Table::Meta)?;
        let sid = meta
            .rows
            .iter()
            .find(|r| text(&r[1]).ok() == Some("session_id"))
            .ok_or_else(|| invalid("缺session"))?;
        let mut facts = Self {
            clock,
            sid: budget.string(text(&sid[2])?)?,
            inc: budget.string(text(&protocol[3])?)?,
            raw_tails: budget.empty()?,
            coordinates: budget.empty()?,
            pending: native::Tree::new(budget)?,
            frontiers: budget.empty()?,
            next: integer(&protocol[6])?,
            max_ns: parse_canonical_i64(text(&protocol[7])?, "phase frontier")?,
            epoch_count: 0,
            epoch_to: None,
            epoch_generation: 0,
            retained: 0,
            raw_capacity: None,
            raw_fees: budget.empty()?,
        };
        budget.vec_grow(&mut facts.frontiers, state.cuts.len())?;
        facts.frontiers.extend(
            state
                .cuts
                .iter()
                .map(|c| (c.metadata.generation, c.metadata.input_frontier)),
        );
        for r in state.raw.view().iter() {
            let id = bj::text(r, "identity_key")?;
            let coord = bj::coord(r)?;
            facts.put_raw(
                id,
                coord,
                bj::field(r, "revision")
                    .as_i64()
                    .ok_or_else(|| invalid("revision"))?,
                budget,
            )?;
        }
        for m in &effects::table(originals, Table::Message)?.rows {
            if text(&m[8])? == "AcceptedPending" {
                budget.tree_insert(
                    &mut facts.pending,
                    budget.string(text(&m[10])?)?,
                    (integer(&m[0])?, integer(&m[11])?),
                )?;
            }
        }
        if let Some(e) = effects::table(originals, Table::Epoch)?.rows.last() {
            facts.epoch_count = integer(&e[1])?;
            facts.epoch_to = Some(parse_writer_epoch(text(&e[3])?, "epoch.to")?);
            facts.epoch_generation = parse_canonical_i64(text(&e[4])?, "epoch.generation")?;
        }
        facts.retained = facts.heap_bytes();
        Ok(facts)
    }
    /// effects 已回读并保留 old/new；先闭合所有关系，随后才允许新 cut 消费。
    pub(super) fn advance(
        &mut self,
        tx: &Transaction<'_>,
        effects: &Effects,
        audited: &mut state::AuditedState,
        budget: &Budget,
    ) -> Result<usize, Failure> {
        let before = budget.live();
        let raw_added = self.advance_inner(tx, effects, audited, budget)?;
        self.retained = self
            .retained
            .checked_add(budget.live())
            .ok_or(native::BudgetExceeded)?
            .checked_sub(before)
            .and_then(|n| n.checked_sub(raw_added.manual))
            .and_then(|n| n.checked_sub(raw_added.native))
            .ok_or_else(|| invalid("Q增量归属账不闭合"))?;
        Ok(raw_added.manual)
    }
    pub(super) fn retained_bytes(&self) -> usize {
        self.retained
    }
    fn advance_inner(
        &mut self,
        tx: &Transaction<'_>,
        effects: &Effects,
        audited: &mut state::AuditedState,
        budget: &Budget,
    ) -> Result<RawCharges, Failure> {
        let old_g = audited.cuts.last().map_or(0, |c| c.metadata.generation);
        let root_g = meta_number(tx, "generation")?;
        let publish = effects.operation == Operation::Publish;
        if root_g != old_g + i64::from(publish) {
            return Err(invalid("固定写口改变了未声明的根代际"));
        }
        for change in &effects.changes {
            self.preservation(change, effects.operation, root_g)?;
        }
        let raw_added = self.raw(tx, effects, audited, budget)?;
        let mut protocol = CheckedStatement::prepare(tx, c"SELECT next_attempt_ordinal,logical_phase_frontier FROM s_protocol_meta WHERE singleton=1").map_err(invalid)?;
        let mut protocol_row = protocol
            .next()
            .map_err(invalid)?
            .ok_or_else(|| invalid("缺protocol"))?;
        let next = protocol_row.integer(0).map_err(invalid)?;
        let frontier = parse_canonical_i64(
            protocol_row.text(1).map_err(invalid)?,
            "logical_phase_frontier",
        )?;
        drop(protocol_row);
        protocol.finalize().map_err(invalid)?;
        if next != self.next + i64::from(effects.operation == Operation::Begin) {
            return Err(invalid("phase frontier/attempt序列断裂"));
        }
        if publish {
            let d = new(exactly_one(
                effects.rows(Table::Delta),
                "逐代 Delta 未闭合",
            )?)?;
            let p = new(exactly_one(
                effects.rows(Table::Publication),
                "逐代 publication 未闭合",
            )?)?;
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
            budget.vec_grow(&mut self.frontiers, 1)?;
            self.frontiers.push((root_g, f));
            self.delivery(effects, root_g, budget)?;
        }
        // 同一effects活期的事件借用；pending键仅在消息处理后更新。
        let affected_bound = effects
            .rows(Table::Message)
            .count()
            .saturating_add(if publish { self.pending.len() } else { 0 });
        let _affected_permit = budget.reserve(tree_slots::<(&str, i64), ()>(affected_bound))?;
        let mut affected = BTreeSet::new();
        for c in effects.rows(Table::Message) {
            affected.insert((text(&new(c)?[10])?, c.rowid));
        }
        // 包含没有被 UPDATE 命中的旧 pending attempt；根前移同样改变其 Begin 目标。
        if publish {
            for (event, (rowid, count)) in self.pending.iter() {
                if *count > 0 {
                    affected.insert((event.as_str(), *rowid));
                }
            }
        }
        let raw_insertions = effects.rows(Table::Raw).count();
        let _covered_permit = budget.reserve(tree_slots::<i64, ()>(raw_insertions))?;
        let mut covered_new = BTreeSet::new();
        let _updates_permit =
            budget.reserve(vec_bytes::<(native::Reserved<String>, i64, i64, bool)>(
                affected.len(),
            ))?;
        let mut updates = Vec::new();
        updates
            .try_reserve_exact(affected.len())
            .map_err(|_| native::BudgetExceeded)?;
        for (_, rowid) in &affected {
            let m = read_message(tx, *rowid, budget)?;
            let _validation_permit = message_work(tx, &m, budget)?;
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
            self.attempts(tx, &m, key.as_str(), root_g, next, budget)?;
            updates.push((
                budget.string(m.event.as_str())?,
                *rowid,
                m.count,
                m.published.is_none(),
            ));
        }
        if covered_new.len() != raw_insertions {
            return Err(invalid("v2原始接纳序缺持久消息承接"));
        }
        let mut begins = 0;
        let mut commits = 0;
        let recover_count = effects
            .rows(Table::Phase)
            .filter(|c| {
                c.new
                    .as_ref()
                    .is_some_and(|r| text(&r[2]).ok() == Some("recover"))
            })
            .count();
        let _recover_permit = budget.reserve(vec_bytes::<(&str, i64)>(recover_count))?;
        let mut recovers = Vec::new();
        recovers
            .try_reserve_exact(recover_count)
            .map_err(|_| native::BudgetExceeded)?;
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
                recovers.push((ns, g));
            } else if !affected.iter().any(|(e, _)| *e == id) {
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
            || frontier != self.max_ns
        {
            return Err(invalid("phase frontier/attempt序列断裂"));
        }
        let _epochs_permit =
            budget.reserve(vec_bytes::<(&str, i64)>(effects.rows(Table::Epoch).count()))?;
        let mut epochs = Vec::new();
        epochs
            .try_reserve_exact(effects.rows(Table::Epoch).count())
            .map_err(|_| native::BudgetExceeded)?;
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
            epochs.push((ns, at));
        }
        if recovers != epochs
            || recovers.len() != usize::from(effects.operation == Operation::Recover)
        {
            return Err(invalid("recover phase与writer换代时间/代际不闭合"));
        }
        let writer = meta_number(tx, "writer_epoch")?;
        if writer < 1 || self.epoch_to.is_some_and(|e| e != writer) {
            return Err(invalid("writer_epoch与最后换代结果不一致"));
        }
        drop(affected);
        for (event, rowid, count, pending) in updates {
            if pending {
                budget.tree_insert(&mut self.pending, event, (rowid, count))?;
            } else {
                remove_pending(&mut self.pending, &event, budget);
                drop(event);
            }
        }
        self.next = next;
        Ok(raw_added)
    }
    fn raw(
        &mut self,
        tx: &Transaction<'_>,
        effects: &Effects,
        audited: &mut state::AuditedState,
        budget: &Budget,
    ) -> Result<RawCharges, Failure> {
        let pairs = effects.rows(Table::Ohlc).count();
        let _paired_permit = budget.reserve(tree_slots::<i64, ()>(pairs))?;
        let mut paired = BTreeSet::new();
        let raw = &mut audited.raw;
        #[cfg(feature = "s_allocation_json_testonly")]
        if matches!(raw, bj::RawEvents::Owned(_)) {
            return Err(invalid("增量 Q 不接受 Owned 原始账"));
        }
        let mut raw_added = RawCharges::default();
        for change in effects.rows(Table::Raw) {
            let r = new(change)?;
            let seq = integer(&r[6])?;
            if seq != raw.len() as i64 {
                return Err(invalid("raw seq 不是已核连续尾"));
            }
            let pair = effects
                .rows(Table::Ohlc)
                .find(|p| p.new.as_ref().is_some_and(|p| p[1] == r[1] && p[2] == r[2]))
                .ok_or_else(|| invalid("raw/OHLC 配对键缺失"))?;
            if !paired.insert(pair.rowid) {
                return Err(invalid("raw/OHLC 配对重复"));
            }
            if matches!(raw, bj::RawEvents::Native(_)) {
                let added = budget
                    .codec_work()?
                    .run(|work| -> Result<usize, Failure> {
                        let value = read_raw_event_at_admitted(tx, seq, Some(work))?;
                        let view = value.view();
                        let id = bj::text(view, "identity_key")?;
                        let coord = bj::coord(view)?;
                        let previous = self.raw_tails.get(id);
                        bj::validate_raw_coordinate(
                            id,
                            coord,
                            self.coordinates.get(&coord).map(|value| value.as_str()),
                            previous.map(|(c, _)| *c),
                        )?;
                        bj::validate_raw_row_admitted(
                            view,
                            raw.len(),
                            previous.map(|(_, r)| *r),
                            Some(work),
                        )?;
                        self.put_raw(
                            id,
                            coord,
                            bj::field(view, "revision").as_i64().unwrap(),
                            budget,
                        )?;
                        let before = budget.live();
                        raw.push_native(view, work)?;
                        budget
                            .live()
                            .checked_sub(before)
                            .ok_or_else(|| invalid("Native原始账追加费用倒退"))
                    })
                    .map_err(Failure::from)?;
                raw_added.native = raw_added
                    .native
                    .checked_add(added)
                    .ok_or(native::BudgetExceeded)?;
                continue;
            }
            let bj::RawEvents::Plain(raw) = raw else {
                return Err(invalid("原始账存储类型不支持增量追加"));
            };
            // decode_raw_row 17字段 + enrich_raw_row 5字段；SQL逐列复制一次，JSON键各复制一次。
            let paired_row = new(pair)?;
            let cell_text = |v: &SqlCell| match v {
                SqlCell::Text(v) => v.len(),
                _ => 0,
            };
            let raw_text = sum(r.iter().map(cell_text));
            let paired_text = sum(paired_row.iter().map(cell_text));
            // json!逐字段to_value期间最多多活一份raw列文本；enrich的五列tuple与输出克隆共存。
            let text_bytes = sum([
                raw_text,
                r.iter().map(cell_text).max().unwrap_or(0),
                paired_text.saturating_mul(2),
            ]);
            const KEYS: [&str; 22] = [
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
                "schema_revision",
                "open",
                "high",
                "low",
                "close",
            ];
            let raw_permit = budget.reserve(sum([
                text_bytes,
                KEYS.iter().map(|k| k.len()).sum(),
                tree_slots::<String, Value>(KEYS.len()),
            ]))?;
            let value = read_raw_event_at(tx, seq)?;
            raw_added.manual = raw_added.manual.saturating_add(value.heap_bytes());
            let raw_fee = raw_permit.finish(value.heap_bytes());
            budget.vec_grow(&mut self.raw_fees, 1)?;
            self.raw_fees.push(raw_fee);
            let view = (&value).into();
            let id = bj::text(view, "identity_key")?;
            let coord = bj::coord(view)?;
            let previous = self.raw_tails.get(id);
            bj::validate_raw_coordinate(
                id,
                coord,
                self.coordinates.get(&coord).map(|value| value.as_str()),
                previous.map(|(c, _)| *c),
            )?;
            {
                let shape = JsonShape::of_value(&value);
                let _validation_permit =
                    budget.reserve(sum([shape.parse_peak(), shape.canonical_peak()]))?;
                bj::validate_raw_row(view, raw.len(), previous.map(|(_, r)| *r), None)?;
            }
            self.put_raw(
                id,
                coord,
                bj::field(view, "revision").as_i64().unwrap(),
                budget,
            )?;
            let old = vec_bytes::<Value>(raw.capacity());
            // #1488 U2：不追认冷构造遗留的 Plain raw 容量。首次增量 Q 先从本父取同容量新块许可、
            // 再分配，载荷移入后旧块析构；许可随 raw_capacity 持久，后续 grow_vec 直接使用。
            if self.raw_capacity.is_none() {
                self.raw_capacity = Some(budget.rebirth_vec(raw)?);
            }
            budget.grow_vec(raw, self.raw_capacity.as_mut().unwrap(), 1)?;
            raw_added.manual = raw_added
                .manual
                .saturating_add(vec_bytes::<Value>(raw.capacity()) - old);
            raw.push(value);
        }
        if paired.len() != pairs {
            return Err(invalid("OHLC/raw 键集未闭合"));
        }
        Ok(raw_added)
    }
    fn attempts(
        &self,
        tx: &Transaction<'_>,
        m: &v2::AuditMessage,
        key: &str,
        root: i64,
        next: i64,
        budget: &Budget,
    ) -> Result<(), Failure> {
        let _attempt_permit = budget.reserve(sum([
            tree_slots::<i64, (Option<i64>, i64)>(
                usize::try_from(m.count)
                    .unwrap_or(usize::MAX)
                    .saturating_add(1)
                    .saturating_mul(2),
            ),
            vec_bytes::<i64>(usize::try_from(m.count).unwrap_or(usize::MAX)),
        ]))?;
        let mut stmt = CheckedStatement::prepare(tx,c"SELECT phase,attempt_index,semantic_ns,message_key,attempt_ordinal,generation FROM s_clock_events WHERE clock_event_id=?1 ORDER BY phase,attempt_index").map_err(invalid)?;
        stmt.bind_text(1, m.event.as_str()).map_err(invalid)?;
        let mut accept = false;
        let mut begins = BTreeMap::new();
        let mut commits = BTreeMap::new();
        while let Some(mut r) = stmt.next().map_err(invalid)? {
            let index = r.integer(1).map_err(invalid)?;
            let ord = r.optional_integer(4).map_err(invalid)?;
            let g = r.integer(5).map_err(invalid)?;
            // CheckedRow 的文本借用限到下一次列取值，故先按实际文本长度预留三份临时复制。
            let bytes = sum([
                r.text(0).map_err(invalid)?.len(),
                r.text(2).map_err(invalid)?.len(),
                r.text(3).map_err(invalid)?.len(),
            ]);
            let _text_permit = budget.reserve(bytes)?;
            let phase = copy_text(r.text(0).map_err(invalid)?)?;
            let ns = copy_text(r.text(2).map_err(invalid)?)?;
            let actual_key = copy_text(r.text(3).map_err(invalid)?)?;
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
        stmt.finalize().map_err(invalid)?;
        let mut indexes = Vec::new();
        indexes
            .try_reserve_exact(begins.len())
            .map_err(|_| native::BudgetExceeded)?;
        indexes.extend(begins.keys().copied());
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
    fn delivery(&self, effects: &Effects, g: i64, budget: &Budget) -> Result<(), Failure> {
        let policy = exactly_one(effects.rows(Table::Policy), "缺 delivery policy 效果")?;
        let old = old(policy)?;
        let new = new(policy)?;
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
        let count = effects.rows(Table::Delivery).count();
        let _delivery_permit = budget.reserve(vec_bytes::<i64>(count.saturating_mul(2)))?;
        let mut removed = Vec::new();
        let mut added = Vec::new();
        removed
            .try_reserve_exact(count)
            .map_err(|_| native::BudgetExceeded)?;
        added
            .try_reserve_exact(count)
            .map_err(|_| native::BudgetExceeded)?;
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
    fn preservation(&self, c: &Change, op: Operation, g: i64) -> Result<(), Failure> {
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
fn read_message(
    tx: &Transaction<'_>,
    rowid: i64,
    budget: &Budget,
) -> Result<native::Reserved<v2::AuditMessage>, Failure> {
    let mut s=CheckedStatement::prepare(tx,c"SELECT source_namespace,source_epoch,message_id,payload_hash,canonical_envelope,receipt_id,accepted_seq,status,first_published_generation,clock_event_id,attempt_count FROM s_input_messages WHERE rowid=?1").map_err(invalid)?;
    s.bind_integer(1, rowid).map_err(invalid)?;
    let mut row = s
        .next()
        .map_err(invalid)?
        .ok_or_else(|| invalid("缺实际 message"))?;
    let mut bytes = 0usize;
    for i in [0, 1, 2, 3, 4, 5, 7, 9] {
        bytes = bytes.saturating_add(row.text(i).map_err(invalid)?.len());
    }
    let permit = budget.reserve(bytes)?;
    let result = v2::read_audit_message(&mut row, None)?;
    drop(row);
    s.finalize().map_err(invalid)?;
    Ok(permit.hold(result))
}
fn copy_text(s: &str) -> Result<String, Failure> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| native::BudgetExceeded)?;
    out.push_str(s);
    Ok(out)
}
fn meta_number(tx: &Transaction<'_>, key: &str) -> Result<i64, Failure> {
    let mut stmt =
        CheckedStatement::prepare(tx, c"SELECT value FROM meta WHERE key=?1").map_err(invalid)?;
    stmt.bind_text(1, key).map_err(invalid)?;
    let mut row = stmt
        .next()
        .map_err(invalid)?
        .ok_or_else(|| invalid("缺meta整数"))?;
    let value = parse_canonical_i64(row.text(0).map_err(invalid)?, key)?;
    drop(row);
    stmt.finalize().map_err(invalid)?;
    Ok(value)
}
fn exactly_one<'a>(
    mut rows: impl Iterator<Item = &'a Change>,
    what: &str,
) -> Result<&'a Change, Failure> {
    let row = rows.next().ok_or_else(|| invalid(what))?;
    if rows.next().is_some() {
        return Err(invalid(what));
    }
    Ok(row)
}
fn message_work(
    tx: &Transaction<'_>,
    m: &v2::AuditMessage,
    budget: &Budget,
) -> Result<native::Permit, Failure> {
    let shape = JsonShape::scan(m.raw.as_bytes());
    // 共享谓词额外回读 raw receipt/hash/identity 三列；先借用计字节，不能用SQL字节代JSON heap。
    let mut stmt = CheckedStatement::prepare(
        tx,
        c"SELECT receipt_id,payload_hash,identity_key FROM raw_events WHERE seq=?1",
    )
    .map_err(invalid)?;
    stmt.bind_integer(1, m.seq).map_err(invalid)?;
    let mut row = stmt
        .next()
        .map_err(invalid)?
        .ok_or_else(|| invalid("缺消息raw原件"))?;
    let mut sql_bytes = 0usize;
    for i in 0..3 {
        sql_bytes = sql_bytes.saturating_add(row.text(i).map_err(invalid)?.len());
    }
    drop(row);
    stmt.finalize().map_err(invalid)?;
    // identity 的4段拼接 <= 全部解码文本+3分隔符，Vec增长新旧块<=6倍；key JSON最多6字节转义。
    // envelope/hash/clock 的共享 JsonView 序列化工作向量按每个value两槽 + 字段排序槽覆盖。
    Ok(budget.reserve(sum([
        shape.parse_peak(),
        shape.canonical_peak(),
        sql_bytes,
        shape.text.saturating_add(3).saturating_mul(6),
        sum([m.ns.len(), m.se.len(), m.id.len()])
            .saturating_mul(6)
            .saturating_add(10)
            .saturating_mul(3),
        vec_bytes::<newchan_rust::s_content::capture_allocation::audit_json::JsonView<'_>>(
            shape.values.saturating_mul(2),
        ),
    ]))?)
}
impl Facts {
    fn put_raw(
        &mut self,
        id: &str,
        coord: i64,
        revision: i64,
        budget: &Budget,
    ) -> Result<(), Failure> {
        if let Some(tail) = self.raw_tails.get_mut(id) {
            *tail = (coord, revision);
        } else {
            let key = budget.string(id)?;
            budget.hash_room(&mut self.raw_tails, &key)?;
            self.raw_tails.insert(key, (coord, revision));
        }
        if !self.coordinates.contains_key(&coord) {
            budget.hash_room(&mut self.coordinates, &coord)?;
            self.coordinates.insert(coord, budget.string(id)?);
        }
        Ok(())
    }
}

fn new(c: &Change) -> Result<&[SqlCell], Failure> {
    c.new
        .as_ref()
        .map(|row| row.as_ref())
        .ok_or_else(|| invalid("缺实际新行"))
}
fn old(c: &Change) -> Result<&[SqlCell], Failure> {
    c.old
        .as_ref()
        .map(|row| row.as_ref())
        .ok_or_else(|| invalid("缺绑定旧行"))
}

/// #1394 最后一项删除后 std BTreeMap 仍可能持有空叶根；归还树槽费前真正释放它。
fn remove_pending(pending: &mut native::Tree<(i64, i64)>, event: &str, _budget: &Budget) {
    pending.remove(event);
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn pending_normal_last_remove_releases_root_before_charge() {
    let budget = Budget::new(1_048_576, 0).unwrap();
    let initial_charge = budget.live();
    let mut pending = native::Tree::new(&budget).unwrap();
    let window = super::allocation_probe::Window::start();
    for index in 0..24 {
        let event = format!("pending-{index:02}");
        let key = budget.string(&event).unwrap();
        budget.tree_insert(&mut pending, key, (index, 1)).unwrap();
    }
    for index in 0..24 {
        let event = format!("pending-{index:02}");
        remove_pending(&mut pending, &event, &budget);
    }
    let observed = window.finish();
    assert!(pending.is_empty());
    assert_eq!(budget.live(), initial_charge);
    assert_eq!(
        observed["live_after"], observed["live_before"],
        "归还全部树槽费时，正常插入再移除的树根和载荷必须已真实释放"
    );
    assert_eq!(observed["allocated_bytes"], observed["released_bytes"]);
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
#[test]
fn actual_nonempty_native_q_accept_preserves_full_raw_without_manual_double_charge() {
    let mut fixture = prepared::tests::Fixture::new();
    let (event, token) = fixture.accept_begin(1, ("q-native-first", 0, 6, 12, 3, 7), 1);
    fixture.publish(&event, &token).unwrap();
    let owner = AuditConnection::open(&fixture.db_path()).unwrap();
    let mut context = request_audit::RequestAuditContext::with_source_keeper(
        cross_request_source_cache::Keeper::new(536_870_912),
    );
    context
        .freeze_manifest_workspace_budget(536_870_912, 0, 0)
        .unwrap();
    let cold = owner.begin(TransactionBehavior::Deferred).unwrap();
    cold.audit_admission(
        &json!({"payload":{"op":"ready"}}),
        &s_allocation::Mode::CapturedNative,
        &mut context,
    )
    .unwrap();
    let ended = cold.read_end().unwrap();
    owner.continue_with(ended).unwrap();
    let (old_raw, old_historical) = {
        let complete = owner.complete.borrow();
        let c = complete.as_ref().unwrap();
        assert!(matches!(&c.audited.raw, bj::RawEvents::Native(_)));
        assert_eq!(c.audited.raw.len(), 1);
        // #1488 wide16-0058：冷候选仍持slot双份历史费；任何固定操作入口先detach_slot（#1394 c45d39be52起）
        // 真实释放slot份。基线取detach后的唯一归属份，只观察Accept本身不给Native raw记手工费。
        (
            serde_json::to_value(&c.audited.raw).unwrap(),
            c.charge.historical_and_raw_capacity - c.charge.slot_result_capacity,
        )
    };
    let raw = crate::tests::located_ohlc_input(&[("q-native-next", 10, 8, 16, 6, 12)]);
    let payload = json!({"op":"ingest","target_session_id":"s-session-testonly-001",
        "target_session_generation":"1","writer_epoch":"1","clock_event_id":"input-2","raw_input":raw});
    let envelope = json!({"schema_revision":"s-session/3","session_id":"s-session-testonly-001",
        "session_generation":"1","source_namespace":"testonly.ohlc","source_epoch":"1",
        "message_id":"message-2","producer_id":"real-content-test","producer_epoch":"1",
        "payload_hash":sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
    let clock = v2::stored_clock(&owner.connection).unwrap();
    let (admission, ended) = owner
        .admit(
            envelope,
            "1",
            &clock,
            tb02a::PROFILE,
            Some("1"),
            &s_allocation::Mode::CapturedNative,
            &mut context,
        )
        .unwrap();
    assert!(matches!(
        admission,
        Admission::Advance { accepted: true, .. }
    ));
    owner.continue_with(ended).unwrap();
    owner.assert_complete_matches_cold();
    let complete = owner.complete.borrow();
    let c = complete.as_ref().unwrap();
    let bj::RawEvents::Native(raw_owner) = &c.audited.raw else {
        panic!("真实Accept不得降为无scope原始账");
    };
    let actual = serde_json::to_value(&c.audited.raw).unwrap();
    assert_eq!(actual.as_array().unwrap().len(), 2);
    assert_eq!(actual[0], old_raw[0], "旧版本完整内容逐字保留");
    assert_eq!(actual[1]["seq"], 1);
    assert_eq!(actual[1]["revision"], 1);
    assert_eq!(actual[1]["schema_revision"], tb02a::RAW_SCHEMA);
    assert_eq!(
        actual[1],
        read_raw_event_at(&owner.connection, 1).unwrap(),
        "身份、版本、raw_text与OHLC完整字段由同核SQL原件比对"
    );
    assert_eq!(c.audited.raw.heap_bytes(), 0, "Native不再进入旧手工退款账");
    assert_eq!(c.charge.historical_and_raw_capacity, old_historical);
    assert!(raw_owner.retained_heap_bytes() > 0);
    assert!(c.resources.live() >= raw_owner.retained_heap_bytes());
    assert_eq!(c.charge.q_capacity, c.q.retained_bytes());
}
