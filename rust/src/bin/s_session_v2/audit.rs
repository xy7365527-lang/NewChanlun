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

struct Family {
    name: &'static str,
    records: BTreeMap<String, (Value, String, i64)>,
    born: BTreeMap<i64, BTreeSet<String>>,
    seen: BTreeSet<String>,
}
impl Family {
    fn capture(conn: &Connection, snapshot: &Value, name: &'static str) -> Result<Self, String> {
        let sql = match name {
            "witnesses" => "SELECT witness_id,published_generation FROM witnesses",
            "observations" => "SELECT observation_id,published_generation FROM observations",
            "relations" => {
                "SELECT subject,relation_type,object,published_generation FROM relations"
            }
            _ => return Err("StorageUnavailable：未知索引族".into()),
        };
        let mut generations = BTreeMap::new();
        let mut q = conn.prepare(sql).map_err(|e| e.to_string())?;
        let mut rows = q.query([]).map_err(|e| e.to_string())?;
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            let (id, g): (String, i64) = if name == "relations" {
                (
                    canonical_json(
                        &json!({"subject":r.get::<_,String>(0).map_err(|e|e.to_string())?,
                    "relation_type":r.get::<_,String>(1).map_err(|e|e.to_string())?,
                    "object":r.get::<_,String>(2).map_err(|e|e.to_string())?}),
                    ),
                    r.get(3).map_err(|e| e.to_string())?,
                )
            } else {
                (
                    r.get(0).map_err(|e| e.to_string())?,
                    r.get(1).map_err(|e| e.to_string())?,
                )
            };
            if generations.insert(id, g).is_some() {
                return Err(format!("StorageUnavailable：{name} 重复身份"));
            }
        }
        let mut result = Self {
            name,
            records: BTreeMap::new(),
            born: BTreeMap::new(),
            seen: BTreeSet::new(),
        };
        for record in required_array(snapshot, name)? {
            let id = identity(record, name)?;
            let g = generations
                .remove(&id)
                .ok_or("StorageUnavailable：索引缺出生代")?;
            result.born.entry(g).or_default().insert(id.clone());
            result.records.insert(
                id,
                (
                    record.clone(),
                    canonical_json(&legacy_payload_projection(record)),
                    g,
                ),
            );
        }
        if !generations.is_empty() {
            return Err("StorageUnavailable：索引捕获成员不完整".into());
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
        batch_records: &[Value],
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
    active: BTreeMap<String, Value>,
    births: BTreeMap<i64, Vec<Value>>,
    deaths: BTreeMap<i64, Vec<Value>>,
    witnesses: Family,
    relations: Family,
    observations: Family,
    original_observations: BTreeMap<String, Value>,
    raw: Vec<Value>,
    objects: BTreeMap<String, Value>,
}
impl AuditWalk {
    pub(super) fn capture(conn: &Connection) -> Result<Self, String> {
        let snapshot = read_snapshot_in_tx(conn, None)?;
        let mut result = Self {
            active: BTreeMap::new(),
            births: BTreeMap::new(),
            deaths: BTreeMap::new(),
            witnesses: Family::capture(conn, &snapshot, "witnesses")?,
            relations: Family::capture(conn, &snapshot, "relations")?,
            observations: Family::capture(conn, &snapshot, "observations")?,
            original_observations: BTreeMap::new(),
            raw: required_array(&snapshot, "raw_history")?.clone(),
            objects: BTreeMap::new(),
        };
        let mut object_ids = BTreeSet::new();
        for o in required_array(&snapshot, "objects")?
            .iter()
            .chain(required_array(&snapshot, "withdrawn_objects")?)
        {
            if !object_ids.insert(identity(o, "objects")?) {
                return Err("StorageUnavailable：对象重复身份".into());
            }
            result.objects.insert(identity(o, "objects")?, o.clone());
            result
                .births
                .entry(number(o, "first_known_generation")?)
                .or_default()
                .push(o.clone());
            if !o["withdrawn_generation"].is_null() {
                result
                    .deaths
                    .entry(number(o, "withdrawn_generation")?)
                    .or_default()
                    .push(o.clone());
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
            let mut at_previous = record.clone();
            if !record["withdrawn_generation"].is_null() {
                let Ok(withdrawn) = number(record, "withdrawn_generation") else {
                    return false;
                };
                if withdrawn > generation {
                    for key in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                        at_previous[key] = Value::Null;
                    }
                    at_previous["lifecycle"] = json!("active");
                }
            }
            if old.objects.get(id) != Some(&at_previous) {
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
        // 对每个持久缓存投影按规范源字节记账（含重复持有的投影）；不是 RSS 估算。
        let mut n = 0usize;
        for v in self
            .active
            .values()
            .chain(self.objects.values())
            .chain(self.original_observations.values())
            .chain(self.raw.iter())
            .chain(self.births.values().flatten())
            .chain(self.deaths.values().flatten())
        {
            n = n.saturating_add(canonical_json(v).len());
        }
        for f in [&self.witnesses, &self.relations, &self.observations] {
            for (id, (v, c, _)) in &f.records {
                n = n.saturating_add(id.len() + canonical_json(v).len() + c.len() + 8);
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
        let mut index_born: Vec<Value> = Vec::new();
        if let Some(births) = self.births.remove(&g) {
            for mut o in births {
                for key in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                    o[key] = Value::Null;
                }
                o["lifecycle"] = json!("active");
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
                    delta_next.insert(id, o.clone());
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
        same_records(
            &json!(index_next.values().collect::<Vec<_>>()),
            &json!(batch_next),
            "objects/独立索引活动集",
        )?;
        same_records(
            &json!(delta_next.values().collect::<Vec<_>>()),
            &json!(batch_next),
            "objects/Delta活动集",
        )?;
        same_records(&delta["withdrawals"], &json!(withdrawals), "withdrawals")?;
        same_records(&delta["replaces"], &json!(replaces), "replaces")?;
        if mode.changes_only() {
            // 本代新生/替代精确等于 Delta upserts：漏 birth 与伪旧 member 都拒绝。
            same_records(&delta["upserts"], &json!(index_born), "upserts/本代首次")?;
        } else {
            // 旧 full 域保持旧判据：Delta upserts 仍是完整活动集。
            same_records(&delta["upserts"], &json!(batch_next), "upserts/batch")?;
        }

        // 三族：封存批次成员须有独立端点；changes-only 域另核 Delta 精确等于本代首次发布集。
        let batch_witnesses = required_array(batch, "witnesses")?
            .iter()
            .map(project_witness_wire)
            .collect::<Result<Vec<_>, _>>()?;
        let batch_relations = required_array(batch, "relations")?.clone();

        // 关系总量（batch 生成的关系 + 替代边 + 源修订 supersedes）与 Delta 对拍：旧 full 域逐字相同；
        // changes-only 域的关系增量由上面「精确等于首次发布集」独立核定。
        let mut relations = batch_relations;
        for r in &replaces {
            relations.push(json!({"subject":r["new_object_id"],"relation_type":"replaces","object":r["old_object_id"]}));
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
        same_records(&json!(raw), &json!(batch_raw), "raw_history/batch")?;
        let from =
            usize::try_from(number(&delta["seq_range"], "from")?).map_err(|e| e.to_string())?;
        same_records(
            &delta["raw_history_added"],
            &json!(raw.get(from..).ok_or("StorageUnavailable：raw区间反向")?),
            "raw_history_added",
        )?;
        for e in raw {
            if let Some(sup) = e["supersedes_revision"].as_str() {
                relations.push(json!({"subject":format!("{}@{}",e["identity_key"].as_str().unwrap_or(""),e["revision"].as_str().unwrap_or("")),"relation_type":"supersedes","object":format!("{}@{sup}",e["identity_key"].as_str().unwrap_or(""))}));
            }
        }
        // changes-only 域的三族 Delta 由下面 Family::verify 的「精确等于本代首次发布集」独立核定，
        // 不在此重述一遍完整集比较（那正是旧实现被取消的假定）。
        if !mode.changes_only() {
            same_records(
                &delta["witnesses"],
                &json!(batch_witnesses),
                "witnesses/batch",
            )?;
            same_records(&delta["relations"], &json!(relations), "relations/batch")?;
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
                same_records(
                    &json!([stored]),
                    &json!([original]),
                    "observation/原封存首版",
                )?;
                self.original_observations.insert(id.clone(), original);
            }
            let original = self.original_observations.get(&id).unwrap().clone();
            if *born == g {
                first_observations.push(original.clone());
            }
            observations.push(original);
        }
        if mode.changes_only() {
            same_records(
                &delta["observations"],
                &json!(first_observations),
                "observations/本代首次",
            )?;
        } else {
            same_records(
                &delta["observations"],
                &json!(observations),
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
