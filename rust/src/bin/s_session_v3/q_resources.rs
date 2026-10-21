//! #1394 C §5.1/5.6.1：启动时冻结的 Q v3 资源合同，无部署默认值。
//! 文件最多64KiB；只接受新版 exact 字段和规范正整数文本，旧v2加载器不变。
use newchan_rust::session_protocol::{canonical_json, positive, sha256_hex, strict_json};
#[cfg(test)]
use serde_json::Value;
use std::{fs::File, io::Read, path::Path};

const FILE_BYTES: usize = 65536;
const REVISION: &str = "s-query-resources/3";
const KEYS: [&str; 21] = [
    "max_frame_bytes",
    "max_pending_connections",
    "socket_read_timeout_ms",
    "socket_write_timeout_ms",
    "max_capture_bytes",
    "capture_deadline_ms",
    "verify_deadline_ms",
    "max_query_workers",
    "max_query_queue",
    "max_reply_bytes",
    "max_cached_images",
    "max_cache_bytes",
    "poll_interval_ms",
    "max_client_leases",
    "client_lease_ms",
    "max_client_pending_batches",
    "max_page_size",
    "max_watch_batches",
    "max_atomic_delta_node_bytes",
    "max_atomic_delta_nodes",
    "max_atomic_delta_records",
];

#[derive(Clone, Copy)]
pub(super) enum Limit {
    FrameBytes,
    PendingConnections,
    SocketReadMs,
    SocketWriteMs,
    CaptureBytes,
    CaptureMs,
    VerifyMs,
    QueryWorkers,
    QueryQueue,
    ReplyBytes,
    CachedImages,
    CacheBytes,
    #[allow(
        dead_code,
        reason = "#1608：资源表槽位（values 按变体序号索引），当前无读取方；删除会移动后续槽位"
    )]
    PollMs,
    ClientLeases,
    ClientLeaseMs,
    ClientPendingBatches,
    PageSize,
    WatchBatches,
    DeltaNodeBytes,
    DeltaNodes,
    DeltaRecords,
}

/// 私有字段且没有修改接口；请求只能读取启动时已校验的同一份配置。
pub(super) struct QResources {
    values: [u64; 21],
    canonical: String,
    digest: String,
}
impl QResources {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > FILE_BYTES {
            return Err(err("资源文件超过64KiB"));
        }
        let wire = strict_json(bytes).map_err(err)?;
        if !wire.as_object().is_some_and(|o| {
            o.len() == 2 && o.contains_key("schema_revision") && o.contains_key("limits")
        }) || wire["schema_revision"] != REVISION
        {
            return Err(err("资源版本或exact外层字段不符"));
        }
        let limits = wire["limits"]
            .as_object()
            .ok_or_else(|| err("limits不是对象"))?;
        if limits.len() != KEYS.len() || limits.keys().any(|key| !KEYS.contains(&key.as_str())) {
            return Err(err(
                "limits必须是18个保留项与3个完整Delta限额，无缺省或扩展字段",
            ));
        }
        let mut values = [0; 21];
        for (i, key) in KEYS.iter().enumerate() {
            let raw = limits[*key]
                .as_str()
                .ok_or_else(|| err(format!("{key}必须为规范正整数文本")))?;
            values[i] = positive(raw, key).map_err(err)? as u64;
        }
        if values[Limit::CachedImages as usize] < values[Limit::QueryWorkers as usize] {
            return Err(err("max_cached_images不得小于max_query_workers"));
        }
        let canonical = canonical_json(&wire);
        let digest = sha256_hex(canonical.as_bytes());
        Ok(Self {
            values,
            canonical,
            digest,
        })
    }
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(err)?
            .take((FILE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        Self::parse(&bytes)
    }
    pub(super) fn limit(&self, key: Limit) -> u64 {
        self.values[key as usize]
    }
    pub(super) fn canonical(&self) -> &str {
        &self.canonical
    }
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }
    #[cfg(unix)]
    pub(super) fn capture_limits(&self) -> super::capture::Limits {
        super::capture::Limits {
            max_capture_bytes: self.limit(Limit::CaptureBytes),
            capture_deadline_ms: self.limit(Limit::CaptureMs),
            verify_deadline_ms: self.limit(Limit::VerifyMs),
        }
    }
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QResourcesInvalid：{e}")
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use serde_json::json;
    /// 仅规范测试输入；原18数值取旧夹具，三新值不是具名部署档。
    pub(in crate::v3) fn input() -> Value {
        let mut old: Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb01c/query-resources.json"
        ))
        .unwrap();
        let fields = old.as_object_mut().unwrap();
        fields.remove("max_atomic_batch_bytes");
        for value in fields.values_mut() {
            *value = json!(value.as_u64().unwrap().to_string());
        }
        fields.insert("max_atomic_delta_node_bytes".into(), json!("16777216"));
        fields.insert("max_atomic_delta_nodes".into(), json!("100000"));
        fields.insert("max_atomic_delta_records".into(), json!("10000"));
        json!({"schema_revision":"s-query-resources/3","limits":old})
    }
    pub(in crate::v3) fn resources() -> QResources {
        QResources::parse(&serde_json::to_vec(&input()).unwrap()).unwrap()
    }
    /// TestOnly，不是产品期限（#1394 B1 根裁定）：只给不测期限的功能测试用的时间类限额。
    /// 冻结的 TB-01-C 夹具在 debug 构建、并发测试线程下完整核验会超过 2 s/5 s，两次请求之间也会
    /// 超过 10 s 的客户租约，功能测试因此随 runner 速度红；这里把捕获/核验期限与客户租约时长放长，
    /// 让功能测试不依赖墙钟。产品期限、冻结夹具 `query-resources.json` 与上面的
    /// `input()`/`resources()` 一字不动；核冻结值的用例、测期限本身的用例（`capture.rs` 受控时钟
    /// 两条）继续用冻结资源，测租约过期的用例（`q_lease`、`q_watch` 租约轨迹）显式设租约时长。
    pub(in crate::v3) const FUNCTIONAL_TEST_ONLY_DEADLINE_MS: u64 = 600_000;
    /// 功能测试资源里换成 [`FUNCTIONAL_TEST_ONLY_DEADLINE_MS`] 的时间类限额；其余冻结值原样。
    pub(in crate::v3) const FUNCTIONAL_TEST_ONLY_KEYS: [&str; 3] = [
        "capture_deadline_ms",
        "verify_deadline_ms",
        "client_lease_ms",
    ];
    /// TestOnly：冻结资源原样，只把 [`FUNCTIONAL_TEST_ONLY_KEYS`] 换成
    /// [`FUNCTIONAL_TEST_ONLY_DEADLINE_MS`]。不是部署档，也不是产品期限。
    pub(in crate::v3) fn functional_input() -> Value {
        let mut value = input();
        for key in FUNCTIONAL_TEST_ONLY_KEYS {
            value["limits"][key] = json!(FUNCTIONAL_TEST_ONLY_DEADLINE_MS.to_string());
        }
        value
    }
    /// TestOnly：[`functional_input`] 的解析结果，供不测期限的功能测试使用。
    pub(in crate::v3) fn functional_resources() -> QResources {
        QResources::parse(&serde_json::to_vec(&functional_input()).unwrap()).unwrap()
    }
    #[test]
    fn functional_test_only_resources_differ_from_frozen_only_in_time_limits() {
        let frozen = input();
        let functional = functional_input();
        for key in KEYS {
            if FUNCTIONAL_TEST_ONLY_KEYS.contains(&key) {
                assert_eq!(functional["limits"][key], json!("600000"), "{key}");
                assert_ne!(functional["limits"][key], frozen["limits"][key], "{key}");
            } else {
                assert_eq!(functional["limits"][key], frozen["limits"][key], "{key}");
            }
        }
        assert_eq!(functional["schema_revision"], frozen["schema_revision"]);
        assert_ne!(functional_resources().digest(), resources().digest());
        #[cfg(unix)]
        {
            let frozen = resources().capture_limits();
            assert_eq!(
                (frozen.capture_deadline_ms, frozen.verify_deadline_ms),
                (2000, 5000)
            );
        }
        assert_eq!(resources().limit(Limit::ClientLeaseMs), 10000);
    }
    #[test]
    fn old_eighteen_values_survive_only_in_explicit_v3_schema() {
        let resource = resources();
        let wire: Value = strict_json(resource.canonical().as_bytes()).unwrap();
        let old: Value = serde_json::from_str(include_str!(
            "../../../../s_session/tests/fixtures/tb01c/query-resources.json"
        ))
        .unwrap();
        for (key, value) in old.as_object().unwrap() {
            if key != "max_atomic_batch_bytes" {
                assert_eq!(
                    wire["limits"][key],
                    json!(value.as_u64().unwrap().to_string()),
                    "{key}"
                );
            }
        }
        assert_eq!(wire["limits"].as_object().unwrap().len(), 21);
        assert!(QResources::parse(&serde_json::to_vec(&old).unwrap()).is_err());
        assert_eq!(resource.limit(Limit::FrameBytes), 1048576);
        #[cfg(unix)]
        assert_eq!(resource.capture_limits().verify_deadline_ms, 5000);
    }
    #[test]
    fn exact_schema_duplicate_keys_and_positive_text_domain_are_enforced() {
        let input = input();
        let parse = |v: &Value| QResources::parse(&serde_json::to_vec(v).unwrap());
        for key in KEYS {
            let mut missing = input.clone();
            missing["limits"].as_object_mut().unwrap().remove(key);
            assert!(parse(&missing).is_err(), "missing {key}");
            for bad in [
                json!(null),
                json!(true),
                json!(1),
                json!(1.0),
                json!(""),
                json!("0"),
                json!("-1"),
                json!("01"),
                json!("+1"),
                json!(" 1"),
                json!("1.0"),
                json!("1e3"),
                json!("１"),
                json!("9223372036854775808"),
            ] {
                let mut value = input.clone();
                value["limits"][key] = bad.clone();
                assert!(parse(&value).is_err(), "{key}={bad}");
            }
        }
        for key in ["extra", "max_atomic_batch_bytes"] {
            let mut value = input.clone();
            value["limits"][key] = json!("1");
            assert!(parse(&value).is_err());
        }
        let mut value = input.clone();
        value["extra"] = json!("1");
        assert!(parse(&value).is_err());
        value = input.clone();
        value["schema_revision"] = json!("s-query-resources/2");
        assert!(parse(&value).is_err());
        value = input.clone();
        value["limits"]["max_cached_images"] = json!("1");
        assert!(parse(&value).is_err());
        value = input.clone();
        for key in KEYS {
            value["limits"][key] = json!(i64::MAX.to_string());
        }
        assert!(parse(&value).is_ok());
        let raw = canonical_json(&input);
        for duplicate in [
            raw.replacen("{", "{\"schema_revision\":\"s-query-resources/3\",", 1),
            raw.replace("\"limits\":{", "\"limits\":{\"max_page_size\":\"1\","),
        ] {
            assert!(QResources::parse(duplicate.as_bytes()).is_err());
        }
        assert!(QResources::parse(format!("{raw}null").as_bytes()).is_err());
        assert!(QResources::parse(&[0xff]).is_err());
    }
    #[test]
    fn bounded_file_is_frozen_and_digest_matches_python_canonical_oracle() {
        use std::{
            fs,
            process::Command,
            time::{SystemTime, UNIX_EPOCH},
        };
        let wire = input();
        let raw = serde_json::to_vec_pretty(&wire).unwrap();
        let parsed = QResources::parse(&raw).unwrap();
        let oracle = Command::new("python3").args(["-c",
            "import json,hashlib,sys;print(hashlib.sha256(json.dumps(json.loads(sys.argv[1]),sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest())",
            std::str::from_utf8(&raw).unwrap()]).output().unwrap();
        assert!(oracle.status.success());
        assert_eq!(
            parsed.digest(),
            String::from_utf8(oracle.stdout).unwrap().trim()
        );
        assert_eq!(
            parsed.digest(),
            QResources::parse(parsed.canonical().as_bytes())
                .unwrap()
                .digest()
        );
        let path = std::env::temp_dir().join(format!(
            "q3-resources-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Remove(std::path::PathBuf);
        impl Drop for Remove {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let _remove = Remove(path.clone());
        assert!(QResources::load(&path).is_err());
        let mut padded = raw;
        padded.resize(FILE_BYTES, b' ');
        fs::write(&path, &padded).unwrap();
        let frozen = QResources::load(&path).unwrap();
        assert_eq!(frozen.digest(), parsed.digest());
        padded.push(b' ');
        fs::write(&path, &padded).unwrap();
        assert!(QResources::load(&path).is_err());
        let mut changed = wire;
        changed["limits"]["max_page_size"] = json!("99");
        fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(frozen.limit(Limit::PageSize), 31);
        assert_eq!(QResources::load(&path).unwrap().limit(Limit::PageSize), 99);
        assert_ne!(QResources::load(&path).unwrap().digest(), frozen.digest());
    }
}
