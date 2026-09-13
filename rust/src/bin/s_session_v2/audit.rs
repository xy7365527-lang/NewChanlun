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
    fn verify(&mut self, delta: &Value, g: i64) -> Result<(), String> {
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
        if self
            .born
            .get(&g)
            .is_some_and(|ids| !ids.is_subset(&present))
        {
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
    pub(super) fn verify(&mut self, delta: &Value, g: i64, batch: &Value) -> Result<(), String> {
        verify_delta_evidence(delta, batch)?;
        let mut upserts = Vec::new();
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
            upserts.push(wire);
        }
        same_records(&delta["upserts"], &json!(upserts), "upserts/batch")?;
        if let Some(births) = self.births.remove(&g) {
            for mut o in births {
                for key in ["withdrawn_generation", "withdrawal_reason", "superseded_by"] {
                    o[key] = Value::Null;
                }
                o["lifecycle"] = json!("active");
                if self.active.insert(identity(&o, "objects")?, o).is_some() {
                    return Err("StorageUnavailable：重复对象出生".into());
                }
            }
        }
        let mut withdrawals = Vec::new();
        let mut replaces = Vec::new();
        if let Some(deaths) = self.deaths.remove(&g) {
            for o in deaths {
                if self.active.remove(&identity(&o, "objects")?).is_none() {
                    return Err("StorageUnavailable：撤回对象尚未出生".into());
                }
                withdrawals.push(json!({"object_id":o["object_id"],"window_start":o["window_start"],"window_mid":o["window_mid"],"window_end":o["window_end"],"reason":o["withdrawal_reason"],"superseded_by":o["superseded_by"]}));
                if !o["superseded_by"].is_null() {
                    replaces.push(
                        json!({"old_object_id":o["object_id"],"new_object_id":o["superseded_by"]}),
                    );
                }
            }
        }
        same_records(
            &json!(self.active.values().collect::<Vec<_>>()),
            &json!(upserts),
            "objects/独立索引活动集",
        )?;
        same_records(&delta["withdrawals"], &json!(withdrawals), "withdrawals")?;
        same_records(&delta["replaces"], &json!(replaces), "replaces")?;
        let witnesses = required_array(batch, "witnesses")?
            .iter()
            .map(project_witness_wire)
            .collect::<Result<Vec<_>, _>>()?;
        same_records(&delta["witnesses"], &json!(witnesses), "witnesses/batch")?;
        let mut relations = required_array(batch, "relations")?.clone();
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
        same_records(&delta["relations"], &json!(relations), "relations/batch")?;
        let mut observations = Vec::new();
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
            observations.push(self.original_observations.get(&id).unwrap().clone());
        }
        same_records(
            &delta["observations"],
            &json!(observations),
            "observations/首版",
        )?;
        self.witnesses.verify(delta, g)?;
        self.relations.verify(delta, g)?;
        self.observations.verify(delta, g)?;
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
