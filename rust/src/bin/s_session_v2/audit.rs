//! #1372：一次新鲜独立索引捕获，按代核出生/撤回与封存原件；旧前缀须精确重核依赖才复用。
//! 每批原件仍完整读取。累计 batch 自身的历史字节不会凭此变成 O(page)。
use super::*;
use std::collections::BTreeSet;

fn number(v: &Value, key: &str) -> Result<i64, String> {
    parse_canonical_i64(
        v[key].as_str().ok_or("StorageUnavailable：索引整数缺失")?,
        key,
    )
    .map_err(|e| format!("StorageUnavailable：{e}"))
}
fn identity(v: &Value, field: &str) -> Result<String, String> {
    if field == "relations" {
        return Ok(canonical_json(v));
    }
    let key = match field {
        "witnesses" => "witness_id",
        "observations" => "observation_id",
        _ => "object_id",
    };
    Ok(v[key]
        .as_str()
        .ok_or("StorageUnavailable：索引身份缺失")?
        .to_string())
}

/// 共享不可变投影的**唯一保留量**记账：同一份原件（同一个 `Arc`）只算一次规范源字节，
/// 不按持有视图的个数重复计费。
fn account_unique(seen: &mut BTreeSet<usize>, total: &mut usize, value: &std::sync::Arc<Value>) {
    if seen.insert(std::sync::Arc::as_ptr(value) as usize) {
        *total = total.saturating_add(canonical_json(value).len());
    }
}

struct Family {
    name: &'static str,
    /// 独立索引行的**不可变原件投影**：一份记录只保留一份，按唯一保留量记账。
    records: BTreeMap<String, (std::sync::Arc<Value>, String, i64)>,
    born: BTreeMap<i64, BTreeSet<String>>,
    seen: BTreeSet<String>,
}
impl Family {
    /// 从**独立索引行**直接读该族的完整记录与出生代。
    ///
    /// 不复用公开快照作为必经中间件：快照会把全库（含原始历史与目录）一并物化，而审计只需要
    /// 这三族索引行本身。记录形状与同 cut 快照逐字段一致（同一组列、同一投影规则）。
    fn capture(conn: &Connection, name: &'static str) -> Result<Self, String> {
        let (sql, project): (
            &str,
            fn(&rusqlite::Row<'_>) -> rusqlite::Result<(String, Option<i64>, Value)>,
        ) = match name {
            "witnesses" => (
                "SELECT witness_id,published_generation,object_id,slot,merged_source_index,
                        merged_high,merged_low,merged_open,merged_close,raw_json FROM witnesses",
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        Some(r.get::<_, i64>(1)?),
                        json!({
                            "witness_id": r.get::<_, String>(0)?,
                            "object_id": r.get::<_, String>(2)?,
                            "slot": r.get::<_, i64>(3)?.to_string(),
                            "merged_source_index": r.get::<_, i64>(4)?.to_string(),
                            "merged_high": r.get::<_, String>(5)?,
                            "merged_low": r.get::<_, String>(6)?,
                            "merged_open": r.get::<_, String>(7)?,
                            "merged_close": r.get::<_, String>(8)?,
                            "raw_bars": project_raw_bars(&json_column(r, 9, JsonShape::Array)?)
                                .map_err(|e| json_column_error(9, e))?,
                        }),
                    ))
                },
            ),
            "observations" => (
                "SELECT observation_id,published_generation,batch_id,kind,window_start,window_mid,
                        window_end,reason,detail_json FROM observations",
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        Some(r.get::<_, i64>(1)?),
                        json!({
                            "observation_id": r.get::<_, String>(0)?,
                            "batch_id": r.get::<_, String>(2)?,
                            "kind": r.get::<_, String>(3)?,
                            "window_start": r.get::<_, Option<i64>>(4)?,
                            "window_mid": r.get::<_, Option<i64>>(5)?,
                            "window_end": r.get::<_, Option<i64>>(6)?,
                            "reason": r.get::<_, String>(7)?,
                            "detail": json_column(r, 8, JsonShape::Object)?,
                        }),
                    ))
                },
            ),
            "relations" => (
                "SELECT subject,relation_type,object,published_generation FROM relations",
                |r| {
                    let subject: String = r.get(0)?;
                    let relation_type: String = r.get(1)?;
                    let object: String = r.get(2)?;
                    Ok((
                        canonical_json(&json!({
                            "subject": subject,
                            "relation_type": relation_type,
                            "object": object,
                        })),
                        Some(r.get::<_, i64>(3)?),
                        json!({
                            "subject": subject,
                            "relation_type": relation_type,
                            "object": object,
                        }),
                    ))
                },
            ),
            _ => return Err("StorageUnavailable：未知索引族".into()),
        };
        let mut result = Self {
            name,
            records: BTreeMap::new(),
            born: BTreeMap::new(),
            seen: BTreeSet::new(),
        };
        let mut q = conn.prepare(sql).map_err(|e| e.to_string())?;
        let mut rows = q.query([]).map_err(|e| e.to_string())?;
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            let (id, generation, record) = project(r).map_err(|e| e.to_string())?;
            let g = generation.ok_or("StorageUnavailable：索引缺出生代")?;
            if identity(&record, name)? != id {
                return Err(format!(
                    "StorageUnavailable：{name} 独立索引身份与其记录不一致"
                ));
            }
            if result.records.contains_key(&id) {
                return Err(format!("StorageUnavailable：{name} 重复身份"));
            }
            result.born.entry(g).or_default().insert(id.clone());
            // 记录只保留一份不可变投影（`Arc`），不再按视图重复深拷贝整条 payload。
            let canonical = canonical_json(&legacy_payload_projection(&record));
            result
                .records
                .insert(id, (std::sync::Arc::new(record), canonical, g));
        }
        Ok(result)
    }
    /// 本代 Delta 与该族独立索引对拍。
    ///
    /// - 内容必须逐字一致，且首次发布代不得晚于本代；
    /// - 未在本审计中出现过的成员，其持久发布代必须恰为本代；
    /// - 封存批次自称的成员必须有独立索引端点（不得只凭批次自称存在）；
    /// - `changes-only` 域另核「本代 Delta 精确等于本代首次发布集」：漏 birth 与伪旧 member 都拒绝。
    fn verify(
        &mut self,
        delta: &Value,
        g: i64,
        mode: delta_storage::Mode,
        batch_records: &[std::sync::Arc<Value>],
    ) -> Result<(), String> {
        let mut present = BTreeSet::new();
        for record in required_array(delta, self.name)? {
            let id = identity(record, self.name)?;
            if !present.insert(id.clone()) {
                return Err(format!("StorageUnavailable：{} Delta 重复身份", self.name));
            }
            let (_, canonical, born) = self
                .records
                .get(&id)
                .ok_or("StorageUnavailable：Delta 没有独立索引端点")?;
            if *canonical != canonical_json(&legacy_payload_projection(record))
                || *born > g
                || (!self.seen.contains(&id) && *born != g)
            {
                return Err(format!(
                    "StorageUnavailable：{} 内容/首次发布代不一致",
                    self.name
                ));
            }
            self.seen.insert(id);
        }
        let mut batch_ids = BTreeSet::new();
        for record in batch_records {
            let id = identity(record, self.name)?;
            if !batch_ids.insert(id.clone()) {
                return Err(format!(
                    "StorageUnavailable：{} 封存批次重复身份",
                    self.name
                ));
            }
            let (_, canonical, born) = self.records.get(&id).ok_or_else(|| {
                format!(
                    "StorageUnavailable：{} 封存批次成员没有独立索引端点",
                    self.name
                )
            })?;
            if *born > g || *canonical != canonical_json(&legacy_payload_projection(record)) {
                return Err(format!(
                    "StorageUnavailable：{} 封存批次内容/发布代与独立索引不一致",
                    self.name
                ));
            }
        }
        let born_g = self.born.get(&g).cloned().unwrap_or_default();
        if mode.changes_only() {
            if present != born_g {
                return Err(format!(
                    "StorageUnavailable：{} 本代 Delta 不等于本代首次发布集（漏 birth 或伪旧 member）",
                    self.name
                ));
            }
            if !born_g.is_subset(&batch_ids) {
                return Err(format!(
                    "StorageUnavailable：{} 封存批次缺本代首次成员",
                    self.name
                ));
            }
        } else if !born_g.is_subset(&present) {
            return Err(format!(
                "StorageUnavailable：{} 本代索引出生未出现在 Delta",
                self.name
            ));
        }
        Ok(())
    }
    fn finish(&self) -> Result<(), String> {
        if self.seen.len() != self.records.len() {
            return Err(format!(
                "StorageUnavailable：{} 存在不可达幽灵行",
                self.name
            ));
        }
        Ok(())
    }
}

pub(super) struct AuditWalk {
    /// 活动集 / 出生 / 撤回三视图共享同一份不可变对象投影（`Arc`），
    /// 一个对象在内存里只留一份；#1394 之前每个视图各持一份深拷贝。
    active: BTreeMap<String, std::sync::Arc<Value>>,
    births: BTreeMap<i64, Vec<std::sync::Arc<Value>>>,
    deaths: BTreeMap<i64, Vec<std::sync::Arc<Value>>>,
    witnesses: Family,
    relations: Family,
    observations: Family,
    original_observations: BTreeMap<String, std::sync::Arc<Value>>,
    raw: Vec<Value>,
    objects: BTreeMap<String, std::sync::Arc<Value>>,
}
impl AuditWalk {
    /// 一次新鲜捕获：对象与三族独立索引直接读各自的行，原始历史由调用方在同一次读集中读入。
    ///
    /// **不经 `read_snapshot_in_tx`**：公开快照会把全库（含 raw_history、catalog、发布头）整体
    /// 物化，而审计只需要对象/见证/关系/观察四族。这里也没有第二次复制：记录直接取自索引行。
    pub(super) fn capture(conn: &Connection, raw: &[Value]) -> Result<Self, String> {
        let (objects, withdrawn) = read_objects_view(conn, None)?;
        let mut result = Self {
            active: BTreeMap::new(),
            births: BTreeMap::new(),
            deaths: BTreeMap::new(),
            witnesses: Family::capture(conn, "witnesses")?,
            relations: Family::capture(conn, "relations")?,
            observations: Family::capture(conn, "observations")?,
            original_observations: BTreeMap::new(),
            raw: raw.to_vec(),
            objects: BTreeMap::new(),
        };
        let mut object_ids = BTreeSet::new();
        for o in objects.iter().chain(withdrawn.iter()) {
            if !object_ids.insert(identity(o, "objects")?) {
                return Err("StorageUnavailable：对象重复身份".into());
            }
            // 三个视图共享同一份不可变投影：每个对象只深拷贝一次。
            let shared = std::sync::Arc::new(o.clone());
            result
                .objects
                .insert(identity(o, "objects")?, shared.clone());
            result
                .births
                .entry(number(o, "first_known_generation")?)
                .or_default()
                .push(shared.clone());
            if !o["withdrawn_generation"].is_null() {
                result
                    .deaths
                    .entry(number(o, "withdrawn_generation")?)
                    .or_default()
                    .push(shared);
            }
        }
        Ok(result)
    }
    /// 以当前独立索引重建旧前缀的全部输入；只允许新增代的出生/撤回。
    /// 旧 raw（包括前次 pending）、三类独立内容与出生代、全部对象原字段都须相同。
    pub(super) fn resume_from(&mut self, old: &Self, generation: i64) -> bool {
        if self.raw.len() < old.raw.len() || self.raw[..old.raw.len()] != old.raw {
            return false;
        }
        for (current, previous) in [
            (&self.witnesses, &old.witnesses),
            (&self.relations, &old.relations),
            (&self.observations, &old.observations),
        ] {
            if previous
                .records
                .iter()
                .any(|(id, r)| current.records.get(id) != Some(r))
                || current
                    .records
                    .iter()
                    .any(|(id, (_, _, g))| *g <= generation && !previous.records.contains_key(id))
            {
                return false;
            }
        }
        let mut matched = 0;
        for (id, record) in &self.objects {
            let Ok(born) = number(record, "first_known_generation") else {
                return false;
            };
            if born > generation {
                continue;
            }
            // 未发生撤回的对象不做任何深拷贝：直接借共享投影比对。
            let at_previous: std::borrow::Cow<'_, Value> =
                if record["withdrawn_generation"].is_null() {
                    std::borrow::Cow::Borrowed(record.as_ref())
                } else {
                    let Ok(withdrawn) = number(record, "withdrawn_generation") else {
                        return false;
                    };
                    if withdrawn <= generation {
                        std::borrow::Cow::Borrowed(record.as_ref())
                    } else {
                        let mut projected = record.as_ref().clone();
                        for key in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                            projected[key] = Value::Null;
                        }
                        projected["lifecycle"] = json!("active");
                        std::borrow::Cow::Owned(projected)
                    }
                };
            let same = old
                .objects
                .get(id)
                .is_some_and(|prev| prev.as_ref() == at_previous.as_ref());
            if !same {
                return false;
            }
            matched += 1;
        }
        if matched != old.objects.len() {
            return false;
        }
        // 上述证明全部成功之后才移动游标；失败时 self 仍是可从代1完整核验的新鲜捕获。
        self.births.retain(|g, _| *g > generation);
        self.deaths.retain(|g, _| *g > generation);
        self.active = old.active.clone();
        self.witnesses.seen = old.witnesses.seen.clone();
        self.relations.seen = old.relations.seen.clone();
        self.observations.seen = old.observations.seen.clone();
        self.original_observations = old.original_observations.clone();
        true
    }
    pub(super) fn source_bytes(&self) -> usize {
        // 对每个持久缓存投影按规范源字节记账；**共享原件按唯一保留量记账**——同一份不可变投影
        // 被多个视图（objects / births / deaths / active / original_observations）引用时只算一次，
        // 不按持有处重复计费。这是记账口径，不是 RSS 估算。
        let mut n = 0usize;
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        for v in self
            .active
            .values()
            .chain(self.objects.values())
            .chain(self.original_observations.values())
            .chain(self.births.values().flatten())
            .chain(self.deaths.values().flatten())
        {
            account_unique(&mut seen, &mut n, v);
        }
        // 原始历史是逐条独立投影（不与其它视图共享），按条记账。
        for v in &self.raw {
            n = n.saturating_add(canonical_json(v).len());
        }
        for f in [&self.witnesses, &self.relations, &self.observations] {
            for (id, (v, c, _)) in &f.records {
                account_unique(&mut seen, &mut n, v);
                n = n.saturating_add(id.len() + c.len() + 8);
            }
            for id in &f.seen {
                n = n.saturating_add(id.len());
            }
            for id in f.born.values().flatten() {
                n = n.saturating_add(id.len() + 8);
            }
        }
        n
    }
    /// **三方强相等**：全量 batch 正规化（①）/ 新鲜独立索引（②）/ base active + Delta（③）
    /// 必须给出逐字段相同的完整 next active。旧实现假定「Delta upserts == 完整活动集」，
    /// changes-only 域下该假定不成立，但完整结果核验不得取消。
    ///
    /// 三个活动集都用同一 wire 投影比较，故撤回字段（`withdrawn_generation`/`withdrawal_reason`/
    /// `superseded_by`/`lifecycle`）与四个首次元数据（first_known_generation/first_known_cut/
    /// batch_id/published_generation）都在比较面内；本代死亡对象另按撤回历史单独核。
    pub(super) fn verify(
        &mut self,
        delta: &Value,
        g: i64,
        batch: &Value,
        mode: delta_storage::Mode,
    ) -> Result<(), String> {
        verify_delta_evidence(delta, batch)?;

        // ① 正规化 full batch：同 ID 沿用既有首次元数据（batch 自身封存的是本代元数据投影）。
        let mut batch_next: Vec<Value> = Vec::new();
        for record in required_array(batch, "objects")? {
            let mut record = record.clone();
            record["batch_id"] = delta["index_frontier"].clone();
            let mut wire = project_object_wire(&record)?;
            if let Some(old) = self.active.get(&identity(&wire, "objects")?) {
                for key in [
                    "batch_id",
                    "first_known_generation",
                    "first_known_cut",
                    "published_generation",
                ] {
                    wire[key] = old[key].clone();
                }
            }
            batch_next.push(wire);
        }

        // ② 新鲜独立索引的 born/death：完整 next active + 本代首次成员原记录。
        let mut index_next = self.active.clone();
        let mut index_born: Vec<std::sync::Arc<Value>> = Vec::new();
        if let Some(births) = self.births.remove(&g) {
            for mut o in births {
                // 本代投影的**独占副本**：`make_mut` 只在共享时深拷贝，
                // 绝不改动仍被 objects/births 视图持有的那份原件。
                {
                    let value = std::sync::Arc::make_mut(&mut o);
                    for key in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                        value[key] = Value::Null;
                    }
                    value["lifecycle"] = json!("active");
                }
                index_born.push(o.clone());
                if index_next.insert(identity(&o, "objects")?, o).is_some() {
                    return Err("StorageUnavailable：重复对象出生".into());
                }
            }
        }
        let mut withdrawals = Vec::new();
        let mut replaces = Vec::new();
        for o in self.deaths.remove(&g).unwrap_or_default() {
            let id = identity(&o, "objects")?;
            if index_next.remove(&id).is_none() {
                return Err("StorageUnavailable：撤回对象尚未出生".into());
            }
            // 撤回历史的完整记录必须在独立快照索引里闭合：撤回代即本代，首次获知不晚于撤回。
            if number(&o, "withdrawn_generation")? != g || number(&o, "first_known_generation")? > g
            {
                return Err("StorageUnavailable：撤回历史首次获知/撤回代不一致".into());
            }
            withdrawals.push(tb02a::withdrawal(
                &o,
                o["withdrawal_reason"]
                    .as_str()
                    .ok_or("StorageUnavailable：撤回原因缺失")?,
                o["superseded_by"].clone(),
            ));
            if !o["superseded_by"].is_null() {
                replaces.push(
                    json!({"old_object_id":o["object_id"],"new_object_id":o["superseded_by"]}),
                );
            }
        }

        // ③ base active 应用本代 Delta：完整 next active。判据侧对「同一身份重复 upsert」与
        //    「无活动前件的撤回」一律拒绝，即 Delta 只能应用一次；消费侧重放的同一性质由
        //    「按身份 upsert + 撤回移动」（幂等）保证，见 base Snapshot + Delta 靶向用例。
        let mut delta_next = self.active.clone();
        for o in required_array(delta, "upserts")? {
            let id = identity(o, "objects")?;
            match delta_next.get(&id) {
                None => {
                    delta_next.insert(id, std::sync::Arc::new(o.clone()));
                }
                Some(old) if !mode.changes_only() => {
                    // 旧 full 域：完整活动集重发，既有身份必须逐字段一致（同 ID 异内容不得吞掉）。
                    if canonical_json(&legacy_payload_projection(old))
                        != canonical_json(&legacy_payload_projection(o))
                    {
                        return Err(
                            "StorageUnavailable：Delta upsert 与既有活动身份内容不一致".into()
                        );
                    }
                }
                Some(_) => {
                    return Err(
                        "StorageUnavailable：Delta upsert 覆盖既有活动身份（同 ID 异内容不得吞掉）"
                            .into(),
                    )
                }
            }
        }
        for w in required_array(delta, "withdrawals")? {
            let id = w["object_id"]
                .as_str()
                .ok_or("StorageUnavailable：Delta 撤回缺 object_id")?
                .to_string();
            if delta_next.remove(&id).is_none() {
                return Err("StorageUnavailable：Delta 撤回无活动前件".into());
            }
        }

        // 三方逐字段相等（不能只核其中两方，也不能只核活动集大小）。
        let normalized_batch_next = {
            // 首次仍先完整规范化 actual，再规范化/查重 expected，保留原错误优先顺序。
            let actual = normalize_record_iter(
                index_next.values().map(|v| v.as_ref()),
                "objects/独立索引活动集",
            )?;
            let expected = normalize_record_iter(&batch_next, "objects/独立索引活动集")?;
            compare_normalized_records(&actual, &expected, "objects/独立索引活动集")?;
            expected
        };
        // 只在首次完整比较成功后复用规范字符串；不再需要这份派生 Value 集合。
        drop(batch_next);
        {
            let actual = normalize_record_iter(
                delta_next.values().map(|v| v.as_ref()),
                "objects/Delta活动集",
            )?;
            compare_normalized_records(&actual, &normalized_batch_next, "objects/Delta活动集")?;
        }
        same_records_against_iter(&delta["withdrawals"], &withdrawals, "withdrawals")?;
        same_records_against_iter(&delta["replaces"], &replaces, "replaces")?;
        if mode.changes_only() {
            // 本代新生/替代精确等于 Delta upserts：漏 birth 与伪旧 member 都拒绝。
            same_records_against_iter(
                &delta["upserts"],
                index_born.iter().map(|v| v.as_ref()),
                "upserts/本代首次",
            )?;
        } else {
            // 旧 full 域保持旧判据：Delta upserts 仍是完整活动集。
            let actual = normalize_record_array(&delta["upserts"], "upserts/batch")?;
            compare_normalized_records(&actual, &normalized_batch_next, "upserts/batch")?;
        }
        // 此完整规范化向量仅属于本次 verify，最后一次比较后立即释放，不跨代保留。
        drop(normalized_batch_next);

        // 三族：封存批次成员须有独立端点；changes-only 域另核 Delta 精确等于本代首次发布集。
        // 三族成员同样以共享不可变投影持有（一记录一份），不按视图重复深拷贝。
        let batch_witnesses = required_array(batch, "witnesses")?
            .iter()
            .map(project_witness_wire)
            .map(|r| r.map(std::sync::Arc::new))
            .collect::<Result<Vec<_>, _>>()?;
        let batch_relations = required_array(batch, "relations")?
            .iter()
            .cloned()
            .map(std::sync::Arc::new)
            .collect::<Vec<_>>();

        // 关系总量（batch 生成的关系 + 替代边 + 源修订 supersedes）与 Delta 对拍：旧 full 域逐字相同；
        // changes-only 域的关系增量由上面「精确等于首次发布集」独立核定。
        let mut relations = batch_relations;
        for r in &replaces {
            relations.push(std::sync::Arc::new(json!({"subject":r["new_object_id"],"relation_type":"replaces","object":r["old_object_id"]})));
        }
        let to = number(delta, "input_frontier")?;
        let end = usize::try_from(to.checked_add(1).ok_or("StorageUnavailable：前沿溢出")?)
            .map_err(|e| e.to_string())?;
        let raw = self
            .raw
            .get(..end)
            .ok_or("StorageUnavailable：批次前沿超过原始事实")?;
        let batch_raw = required_array(batch, "raw_events")?
            .iter()
            .map(project_raw_event_wire)
            .collect::<Result<Vec<_>, _>>()?;
        same_record_iters(raw, &batch_raw, "raw_history/batch")?;
        let from =
            usize::try_from(number(&delta["seq_range"], "from")?).map_err(|e| e.to_string())?;
        same_records_against_iter(
            &delta["raw_history_added"],
            raw.get(from..).ok_or("StorageUnavailable：raw区间反向")?,
            "raw_history_added",
        )?;
        for e in raw {
            if let Some(sup) = e["supersedes_revision"].as_str() {
                relations.push(std::sync::Arc::new(json!({"subject":format!("{}@{}",e["identity_key"].as_str().unwrap_or(""),e["revision"].as_str().unwrap_or("")),"relation_type":"supersedes","object":format!("{}@{sup}",e["identity_key"].as_str().unwrap_or(""))})));
            }
        }
        // changes-only 域的三族 Delta 由下面 Family::verify 的「精确等于本代首次发布集」独立核定，
        // 不在此重述一遍完整集比较（那正是旧实现被取消的假定）。
        if !mode.changes_only() {
            same_records_against_iter(
                &delta["witnesses"],
                batch_witnesses.iter().map(|v| v.as_ref()),
                "witnesses/batch",
            )?;
            same_records_against_iter(
                &delta["relations"],
                relations.iter().map(|v| v.as_ref()),
                "relations/batch",
            )?;
        }
        let mut observations = Vec::new();
        let mut first_observations = Vec::new();
        for ob in required_array(batch, "observations")? {
            let id = observation_id(ob);
            let (stored, _, born) = self
                .observations
                .records
                .get(&id)
                .ok_or("StorageUnavailable：观察没有独立端点")?;
            if !self.original_observations.contains_key(&id) {
                if *born != g || stored["batch_id"] != delta["index_frontier"] {
                    return Err("StorageUnavailable：观察首版批次/出生代不一致".into());
                }
                let mut first = ob.clone();
                first["observation_id"] = json!(id);
                first["batch_id"] = delta["index_frontier"].clone();
                let original = project_observation_wire(&first)?;
                // SQLite 观察列保存整数窗口；公开首版使用规范十进制文本。两侧先走同一
                // 投影再逐字段比较，不能把合法旧 tick domain 观察误报成封存漂移。
                let stored = project_observation_wire(stored)?;
                same_record_iters(
                    std::iter::once(&stored),
                    std::iter::once(&original),
                    "observation/原封存首版",
                )?;
                self.original_observations
                    .insert(id.clone(), std::sync::Arc::new(original));
            }
            let original = self.original_observations.get(&id).unwrap().clone();
            if *born == g {
                first_observations.push(original.clone());
            }
            observations.push(original);
        }
        if mode.changes_only() {
            same_records_against_iter(
                &delta["observations"],
                first_observations.iter().map(|v| v.as_ref()),
                "observations/本代首次",
            )?;
        } else {
            same_records_against_iter(
                &delta["observations"],
                observations.iter().map(|v| v.as_ref()),
                "observations/首版",
            )?;
        }
        self.witnesses.verify(delta, g, mode, &batch_witnesses)?;
        self.relations.verify(delta, g, mode, &relations)?;
        self.observations.verify(delta, g, mode, &observations)?;
        self.active = index_next;
        Ok(())
    }
    pub(super) fn finish(&self) -> Result<(), String> {
        if !self.births.is_empty() || !self.deaths.is_empty() {
            return Err("StorageUnavailable：对象生灭代不可达".into());
        }
        self.witnesses.finish()?;
        self.relations.finish()?;
        self.observations.finish()
    }
}
