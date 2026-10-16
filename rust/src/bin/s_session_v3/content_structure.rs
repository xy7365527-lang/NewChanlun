//! #1419/#1424 v3 C/D/E/F 内容结构层（PriorState 授权）
//!
//! 从同次 parser产物+独立中心扫描派生 TB-02-C/D/E/F typed objects。只作构造，
//! 不操纵六数组或生成态头。按 ROOT-PRIORSTATE-RULING.md 的授权范围访问
//! object_index/prepared已审接口，不越界编辑 StageOwner P 独占区。

use crate::content_producer::TypedObject;
use crate::s_fact_cache;
use newchan_rust::s_content::{self as sc, Built, NodeStore};
use newchan_rust::theta_v0::classifier::center_scan;
use newchan_rust::theta_v0::parser::segment::SegmentFacts;
use newchan_rust::theta_v0::parser::stroke::StrokeFacts;
use newchan_rust::theta_v0::types::Segment;
use serde_json::Value;
use std::collections::BTreeMap;

/// C/D/E/F 的构造输入借用；只取同次 parser产物与PriorState授权根
pub(super) struct StructureInput<'a> {
    pub segments: &'a [Segment],
    pub segment_facts: &'a SegmentFacts,
    pub stroke_facts: &'a StrokeFacts,
    pub all_events: &'a [Value],
    pub raw: &'a BTreeMap<i64, Value>,
    pub prior_structure: &'a [Built],
    pub generation: i64,
    pub cut: &'a str,
    pub frontier: i64,
    pub semantic_commit_ns: Option<&'a str>,
}

/// 构造 C/D E/F typed objects; 不含B; relations 直接返回 Built
pub(super) struct StructureOutput {
    pub objects: Vec<TypedObject>,
    pub relations: Vec<Built>,
}

/// 主入口：消费 StructureInput，返回 C/D/E/F对象+关系
pub(super) fn build_structure(
    store: &mut dyn NodeStore,
    input: &StructureInput<'_>,
    _lane: &s_fact_cache::Lane,
) -> Result<StructureOutput, String> {
    // C/D: 使用已有 crate::tb02c::build 接口（基线已审，此处只接线不重写）
    let second_store = crate::tb02c::round_budget();
    crate::tb02c::restore_cursors(
        &input
            .prior_structure
            .iter()
            .filter_map(|b| decode_object(store, b).ok())
            .collect::<Vec<_>>(),
        &mut second_store.clone(),
    );
    let (cd_objects_json, cd_relations_json) = crate::tb02c::build(
        input.segment_facts,
        input.stroke_facts,
        input.raw,
        input.all_events,
        &input
            .prior_structure
            .iter()
            .filter_map(|b| decode_object(store, b).ok())
            .collect::<Vec<_>>(),
        input.generation,
        input.cut,
        input.frontier,
        input.semantic_commit_ns.map(ToString::to_string),
        &second_store,
    )?;

    // E/F：复用生产塔同一份 center_scan，由 tb02e/tb02f 纯投影事实 wire。
    let units = center_scan::l0_units(input.segments);
    let completed = input
        .segments
        .iter()
        .enumerate()
        .map(|(i, segment)| crate::tb02e::SegUnit {
            unit_id: format!("segment:{}:{}", input.generation, segment.start_index),
            object_id: format!("obj-segment-{}", i),
            range: units.get(i).copied(),
            sources: vec![i],
        })
        .collect::<Vec<_>>();
    let inputs = crate::tb02e::UnitInputs {
        completed,
        in_progress: None,
    };
    let previous = input
        .prior_structure
        .iter()
        .filter_map(|b| decode_object(store, b).ok())
        .collect::<Vec<_>>();
    let (ef_objects, ef_relations, center_build) = crate::tb02e::build(
        &inputs,
        input.raw,
        input.all_events,
        &previous,
        input.generation,
        input.cut,
        input.frontier,
        input.semantic_commit_ns.map(ToString::to_string),
    )?;
    let (pair_objects, pair_relations) = crate::tb02f::build(
        &inputs,
        &center_build,
        input.raw,
        &previous,
        input.generation,
        input.cut,
        input.frontier,
    );

    // 合并
    let mut objects = Vec::new();
    for o in cd_objects_json
        .iter()
        .chain(ef_objects.iter())
        .chain(pair_objects.iter())
    {
        objects.push(typed_from_json(store, o)?);
    }
    let mut relations = Vec::new();
    for r in cd_relations_json
        .into_iter()
        .chain(ef_relations)
        .chain(pair_relations)
    {
        relations.push(json_to_built(store, &r)?);
    }
    Ok(StructureOutput { objects, relations })
}

/// 将 tb02c JSON object 转为 TypedObject
fn typed_from_json(store: &mut dyn NodeStore, o: &Value) -> Result<TypedObject, String> {
    let kind = o["kind"].as_str().ok_or("kind missing")?.to_string();
    let fact_key = o["fact_key"]
        .as_str()
        .ok_or("fact_key missing")?
        .to_string();
    let object_id = o["object_id"]
        .as_str()
        .ok_or("object_id missing")?
        .to_string();
    let payload_json = o["payload_json"].as_str().ok_or("payload_json missing")?;
    let payload_value: Value =
        serde_json::from_str(payload_json).map_err(|e| format!("parse payload: {e}"))?;
    let payload_cvalue = json_to_cvalue(&payload_value)?;
    let record =
        sc::write_value(store, &payload_cvalue).map_err(|e| format!("write payload: {e}"))?;
    Ok(TypedObject {
        kind,
        fact_key,
        object_id,
        record,
        request: None,
    })
}

fn decode_object(store: &mut dyn NodeStore, built: &Built) -> Result<Value, String> {
    let reference = built.content_ref();
    sc::render(store, &reference)
        .and_then(|v| match v {
            sc::Value::Object(m) => Ok(serde_json::to_value(&m).unwrap()),
            _ => Err(sc::CodecError::DataUnavailable),
        })
        .map_err(|e| format!("decode: {e}"))
}

fn json_to_cvalue(v: &Value) -> Result<sc::Value, String> {
    match v {
        Value::Null => Ok(sc::Value::Null),
        Value::Bool(b) => Ok(sc::Value::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(sc::Value::Number(sc::NumberToken::SignedInteger(i)))
            } else {
                Err("unsupported number".into())
            }
        }
        Value::String(s) => Ok(sc::Value::String(s.clone())),
        Value::Array(a) => {
            let mut out = Vec::new();
            for item in a {
                out.push(json_to_cvalue(item)?);
            }
            Ok(sc::Value::Array(out))
        }
        Value::Object(m) => {
            let mut out = BTreeMap::new();
            for (k, val) in m {
                out.insert(k.clone(), json_to_cvalue(val)?);
            }
            Ok(sc::Value::Object(out))
        }
    }
}

fn json_to_built(store: &mut dyn NodeStore, v: &Value) -> Result<Built, String> {
    let cvalue = json_to_cvalue(v)?;
    sc::write_value(store, &cvalue).map_err(|e| format!("write relation: {e}"))
}
