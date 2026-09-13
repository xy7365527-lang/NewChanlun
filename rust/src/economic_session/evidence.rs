//! #1374：固定 TestOnly 定义证据权威。请求只给引用；独评收据与完整原件均由 manifest 固定。
//! 此适配器不生产 γ，不调用旧 π，也不把 Closed/Strict 作为定义 γ 的前置。
#[cfg(test)]
use super::book::hash;
use super::store::Config;
use super::voice::{digest, exact};
use crate::session_protocol::{nonnegative, positive, required, sha256_hex, strict_json};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;
const LIMIT: u64 = 64 * 1024 * 1024;

pub(super) struct VerifiedGammaEvidence {
    binding: Value,
    reference: Value,
    witness: Value,
    observation_ns: i64,
}
impl VerifiedGammaEvidence {
    pub(super) fn observation_ns(&self) -> i64 {
        self.observation_ns
    }
    pub(super) fn binding(&self) -> &Value {
        &self.binding
    }
    pub(super) fn reference(&self) -> Value {
        self.reference.clone()
    }
    pub(super) fn witness(&self) -> Value {
        self.witness.clone()
    }
}
pub(super) fn enabled(config: &Config) -> bool {
    config.domain.starts_with("B:")
        && config
            .manifest
            .get("voice_binding")
            .is_some_and(|v| !v.is_null())
}
pub(super) fn validate_config(manifest: &Value) -> Result<(), String> {
    let Some(v) = manifest.get("voice_binding").filter(|v| !v.is_null()) else {
        return Ok(());
    };
    exact(v, &["schema_revision", "authorities"])?;
    if v["schema_revision"] != "voice-binding/1" || manifest["clock"]["kind"] != "TestOnly" {
        return Err("SchemaUnsupported：Voice 证据仅显式 TestOnly voice-binding/1".into());
    }
    let authorities = v["authorities"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("InvalidDomain：Voice 缺固定权威列表")?;
    let mut ids = std::collections::BTreeSet::new();
    for a in authorities {
        exact(
            a,
            &[
                "authority_id",
                "bundle_path",
                "bundle_sha256",
                "verification_path",
                "verification_sha256",
                "max_bytes",
            ],
        )?;
        if !ids.insert(required(a, "authority_id")?) {
            return Err("IdentityConflict：重复 Voice 权威".into());
        }
        for k in ["bundle_sha256", "verification_sha256"] {
            digest(required(a, k)?)?;
        }
        for k in ["bundle_path", "verification_path"] {
            if !Path::new(required(a, k)?).is_absolute() {
                return Err("InvalidDomain：Voice 固定原件路径须绝对路径".into());
            }
        }
        let max = positive(required(a, "max_bytes")?, "evidence max_bytes")? as u64;
        if max > LIMIT {
            return Err("InvalidDomain：Voice 证据上限超过64MiB".into());
        }
    }
    Ok(())
}
fn authority<'a>(config: &'a Config, id: &str) -> Result<&'a Value, String> {
    if !enabled(config) {
        return Err("MissingDependency：未配置定义 γ 工作例权威".into());
    }
    validate_config(&config.manifest)?;
    config.manifest["voice_binding"]["authorities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["authority_id"] == id)
        .ok_or_else(|| "MissingDependency：未配置该 γ 权威".into())
}
fn read_pinned(path: &str, expected: &str, max: u64) -> Result<String, String> {
    if !Path::new(path).is_absolute() {
        return Err("InvalidDomain：证据原件须固定绝对路径".into());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let f = options
        .open(path)
        .map_err(|e| format!("MissingDependency：固定证据原件不可读：{e}"))?;
    if !f
        .metadata()
        .map_err(|e| format!("StorageUnavailable：证据文件状态：{e}"))?
        .is_file()
    {
        return Err("InvalidDomain：证据原件须普通文件".into());
    }
    let mut bytes = Vec::new();
    f.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("StorageUnavailable：读取证据：{e}"))?;
    if bytes.len() as u64 > max {
        return Err("InvalidDomain：证据原件超过明确上限".into());
    }
    if sha256_hex(&bytes) != expected {
        return Err("IdentityConflict：证据原始字节摘要与固定权威不同".into());
    }
    String::from_utf8(bytes).map_err(|_| "InvalidDomain：证据原件非 UTF8".into())
}

/// 单次读取并保留原字节。运行时只在此处读外部文件，审计重放使用已提交 witness。
pub(super) fn load(
    config: &Config,
    evidence_id: &str,
    authority_id: &str,
) -> Result<VerifiedGammaEvidence, String> {
    if !enabled(config) {
        return Err("MissingDependency：未配置定义 γ 工作例权威".into());
    }
    validate_config(&config.manifest)?;
    let a = authority(config, authority_id)?;
    let max = positive(required(a, "max_bytes")?, "max_bytes")? as u64;
    let raw = read_pinned(
        required(a, "bundle_path")?,
        required(a, "bundle_sha256")?,
        max,
    )?;
    let bundle = strict_json(raw.as_bytes())?;
    if !bundle["bindings"]
        .as_array()
        .ok_or("SchemaUnsupported：权威 bindings 须数组")?
        .iter()
        .any(|b| b["evidence_id"] == evidence_id)
    {
        return Err("MissingDependency：固定权威不存在该 evidence_id".into());
    }
    let verification = read_pinned(
        required(a, "verification_path")?,
        required(a, "verification_sha256")?,
        max,
    )?;
    let witness = json!({"authority_id":a["authority_id"],"evidence_id":evidence_id,"bundle_raw_utf8":raw,"verification_raw_utf8":verification});
    // 先核受信清单/独评绑定，再读其中的固定路径；没有全局扫描无关 authority。
    let verified = verify_witness(config, &witness)?;
    let artifacts = bundle["artifacts"].as_array().unwrap();
    for artifact in artifacts {
        let n = positive(required(artifact, "bytes")?, "artifact bytes")? as u64;
        let actual = read_pinned(
            required(artifact, "path")?,
            required(artifact, "sha256")?,
            n,
        )?;
        if actual.len() as u64 != n {
            return Err("IdentityConflict：原件精确长度与清单不同".into());
        }
    }
    Ok(verified)
}

/// 独评结果必须由 manifest 事先固定，且覆盖完整 bundle 与其中每一原件；绝不相信请求的布尔值。
pub(super) fn verify_witness(
    config: &Config,
    witness: &Value,
) -> Result<VerifiedGammaEvidence, String> {
    exact(
        witness,
        &[
            "authority_id",
            "evidence_id",
            "bundle_raw_utf8",
            "verification_raw_utf8",
        ],
    )?;
    let a = authority(config, required(witness, "authority_id")?)?;
    let raw = required(witness, "bundle_raw_utf8")?;
    let review_raw = required(witness, "verification_raw_utf8")?;
    let max = positive(required(a, "max_bytes")?, "max_bytes")? as usize;
    if raw.len() > max
        || review_raw.len() > max
        || sha256_hex(raw.as_bytes()) != a["bundle_sha256"]
        || sha256_hex(review_raw.as_bytes()) != a["verification_sha256"]
    {
        return Err("IdentityConflict：持久 γ 证据不等于固定原始字节".into());
    }
    let bundle = strict_json(raw.as_bytes())?;
    let review = strict_json(review_raw.as_bytes())?;
    exact(
        &bundle,
        &["schema", "authority_id", "source", "artifacts", "bindings"],
    )?;
    if bundle["schema"] != "voice-binding-authority/1"
        || bundle["authority_id"] != a["authority_id"]
        || review["schema"] != "voice-binding-independent-verification/1"
        || review["verdict"] != "APPROVE_CONTROLLED_VOICE_BINDING"
        || review["authority_id"] != a["authority_id"]
        || review["bundle_sha256"] != a["bundle_sha256"]
        || review["artifacts"] != bundle["artifacts"]
    {
        return Err("IdentityConflict：定义 γ 独立核验未绑定完整原件".into());
    }
    let artifacts = bundle["artifacts"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("MissingDependency：γ 缺独立原件")?;
    let mut names = std::collections::BTreeSet::new();
    let mut total = 0u64;
    for f in artifacts {
        exact(f, &["name", "path", "sha256", "bytes"])?;
        if !names.insert(required(f, "name")?) {
            return Err("IdentityConflict：γ 原件名重复".into());
        }
        digest(required(f, "sha256")?)?;
        if !Path::new(required(f, "path")?).is_absolute() {
            return Err("InvalidDomain：证据原件须固定绝对路径".into());
        }
        let n = positive(required(f, "bytes")?, "artifact bytes")? as u64;
        total = total.checked_add(n).ok_or("InvalidDomain：原件总长溢出")?;
        if n > LIMIT || total > 2 * LIMIT {
            return Err("InvalidDomain：权威原件集合超过明确上限".into());
        }
    }
    for name in [
        "input",
        "controlled_gammas",
        "carrier_objects",
        "instrument_declaration",
        "opening_raw",
    ] {
        if !names.contains(name) {
            return Err(format!("MissingDependency：γ 绑定缺原件 {name}"));
        }
    }
    let bindings = bundle["bindings"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("MissingDependency：γ 权威绑定集合为空")?;
    let ids = bindings
        .iter()
        .map(|b| required(b, "evidence_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != ids.len() || review["verified_evidence_ids"] != json!(ids) {
        return Err("IdentityConflict：独评未逐一覆盖唯一绑定全集".into());
    }
    let b = bindings
        .iter()
        .find(|b| b["evidence_id"] == witness["evidence_id"])
        .ok_or("MissingDependency：权威证据 ID 缺失")?;
    exact(
        b,
        &[
            "evidence_id",
            "instrument",
            "chong_id",
            "operation_level",
            "carrier_ref",
            "gamma_ref",
            "structure_view",
            "sigma",
            "n",
            "parent_voice_id",
            "opening_source",
        ],
    )?;
    if b["carrier_ref"]["authority_id"] != a["authority_id"]
        || b["gamma_ref"]["authority_id"] != a["authority_id"]
        || b["instrument"] != bundle["source"]["instrument"]
    {
        return Err("IdentityConflict：γ/载体/标的不属于同一固定权威".into());
    }
    let input = artifacts.iter().find(|f| f["name"] == "input").unwrap();
    if bundle["source"]["input_sha256"] != input["sha256"] {
        return Err("IdentityConflict：instrument 声明未绑定实际输入".into());
    }
    exact(
        &b["opening_source"],
        &[
            "fact_key",
            "fact_hash",
            "allocation_id",
            "raw_sha256",
            "json_pointer",
            "quantity",
            "units",
        ],
    )?;
    let opening = artifacts
        .iter()
        .find(|f| f["name"] == "opening_raw")
        .unwrap();
    if b["opening_source"]["raw_sha256"] != opening["sha256"] {
        return Err("IdentityConflict：Voice 开局来源未绑定独立原件".into());
    }
    let source = &bundle["source"];
    if source["profile_id"] != b["structure_view"]["profile_id"] {
        return Err("IdentityConflict：Voice 操作/γ profile 未绑定来源".into());
    }
    let mapping = &source["clock_mapping"];
    exact(
        mapping,
        &[
            "kind",
            "source_origin",
            "nanosecond_origin",
            "nanoseconds_per_ordinal",
        ],
    )?;
    if mapping["kind"] != "TestOnlyAffineOrdinalToNs" {
        return Err("SchemaUnsupported：缺显式具名 source ordinal 到纳秒映射".into());
    }
    let source_ordinal = nonnegative(
        required(source, "first_confirming_observation_source")?,
        "γ observation ordinal",
    )?;
    let source_origin = nonnegative(required(mapping, "source_origin")?, "clock source_origin")?;
    let ns_origin = nonnegative(
        required(mapping, "nanosecond_origin")?,
        "clock nanosecond_origin",
    )?;
    let step = positive(required(mapping, "nanoseconds_per_ordinal")?, "clock step")?;
    let delta = source_ordinal
        .checked_sub(source_origin)
        .filter(|n| *n >= 0)
        .ok_or("InvalidDomain：γ 观察序号早于映射原点")?;
    let observation_ns = delta
        .checked_mul(step)
        .and_then(|n| n.checked_add(ns_origin))
        .ok_or("InvalidDomain：γ 观察时钟映射溢出")?;
    Ok(VerifiedGammaEvidence {
        binding: b.clone(),
        observation_ns,
        reference: json!({"authority_id":a["authority_id"],"bundle_sha256":a["bundle_sha256"],"verification_sha256":a["verification_sha256"]}),
        witness: witness.clone(),
    })
}

#[cfg(test)]
pub(super) fn unit_verified(binding: Value) -> VerifiedGammaEvidence {
    // 仅内部纯算子单位测试；不是生产 adapter 的认证分支，不用于正式成功证据。
    VerifiedGammaEvidence {
        binding,
        observation_ns: 0,
        reference: json!({"authority_id":"unit","bundle_sha256":hash(&json!("unit bundle")),"verification_sha256":hash(&json!("unit verifier"))}),
        witness: Value::Null,
    }
}
