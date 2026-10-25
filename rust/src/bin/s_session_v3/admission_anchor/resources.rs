//! #1527：具名 mixed 合同解析；不签发运行资格，不提供 RAM 默认值。
use newchan_rust::s_content::{job::JobLimits, CodecError};
use serde_json::Value;

pub(super) const MIXED_REVISION: &str = "s-admission-mixed-resources/1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NativeRamLimits {
    pub(super) max_live_bytes: u64,
    pub(super) max_operation_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdmissionResources {
    Ordinary(JobLimits),
    Mixed {
        job_limits: JobLimits,
        native_ram: NativeRamLimits,
    },
}

impl AdmissionResources {
    pub(super) fn parse(node: &Value) -> Result<Self, CodecError> {
        let Some(revision) = node.get("schema_revision") else {
            return JobLimits::from_canonical_json(node).map(Self::Ordinary);
        };
        if revision.as_str() != Some(MIXED_REVISION) {
            return Err(invalid("不支持的准入资源 schema_revision"));
        }
        let object = node
            .as_object()
            .ok_or_else(|| invalid("资源合同必须是对象"))?;
        if object.len() != 4
            || ![
                "schema_revision",
                "measurement_domains",
                "job_limits",
                "native_ram",
            ]
            .iter()
            .all(|key| object.contains_key(*key))
        {
            return Err(invalid("mixed 资源合同字段不 exact"));
        }
        let domains = node["measurement_domains"]
            .as_array()
            .ok_or_else(|| invalid("measurement_domains 必须是有序数组"))?;
        if domains.len() != 2
            || domains[0].as_str() != Some("s-content/1")
            || domains[1].as_str() != Some("legacy-evidence/1")
        {
            return Err(invalid(
                "measurement_domains 必须依序为 s-content/1、legacy-evidence/1",
            ));
        }
        let job_limits = JobLimits::from_canonical_json(&node["job_limits"])?;
        let ram = node["native_ram"]
            .as_object()
            .ok_or_else(|| invalid("native_ram 必须是对象"))?;
        if ram.len() != 2
            || !ram.contains_key("max_live_bytes")
            || !ram.contains_key("max_operation_bytes")
        {
            return Err(invalid("native_ram 字段不 exact"));
        }
        let max_live_bytes = nonnegative(&ram["max_live_bytes"])?;
        let max_operation_bytes = nonnegative(&ram["max_operation_bytes"])?;
        if max_operation_bytes > max_live_bytes {
            return Err(invalid("max_operation_bytes 不得大于 max_live_bytes"));
        }
        Ok(Self::Mixed {
            job_limits,
            native_ram: NativeRamLimits {
                max_live_bytes,
                max_operation_bytes,
            },
        })
    }

    pub(super) fn job_limits(self) -> JobLimits {
        match self {
            Self::Ordinary(limits)
            | Self::Mixed {
                job_limits: limits, ..
            } => limits,
        }
    }

    pub(super) fn is_mixed(self) -> bool {
        matches!(self, Self::Mixed { .. })
    }
}

fn nonnegative(value: &Value) -> Result<u64, CodecError> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid("RAM 字节数必须是规范非负十进制字符串"))?;
    if text.is_empty()
        || !text.bytes().all(|b| b.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(invalid("RAM 字节数必须是规范非负十进制字符串"));
    }
    text.parse().map_err(|_| invalid("RAM 字节数超出 u64"))
}

fn invalid(message: &str) -> CodecError {
    CodecError::new("admission_resource_contract", message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(super) fn ordinary() -> Value {
        json!({
            "max_total_wire_bytes":"1048576", "max_total_node_bytes":"1",
            "max_total_nodes":"1", "max_total_records":"1", "max_total_requests":"1",
            "max_elapsed_ms":"60000", "max_staged_bytes":"1", "max_store_bytes":"1"
        })
    }
    fn mixed() -> Value {
        json!({"schema_revision":MIXED_REVISION,
            "measurement_domains":["s-content/1","legacy-evidence/1"],
            "job_limits":ordinary(),
            "native_ram":{"max_live_bytes":"4096","max_operation_bytes":"1024"}})
    }
    #[test]
    fn mixed_resources_keeps_original_job_limits_and_explicit_ram() {
        assert_eq!(
            AdmissionResources::parse(&ordinary()).unwrap(),
            AdmissionResources::Ordinary(JobLimits::from_canonical_json(&ordinary()).unwrap())
        );
        let contract = AdmissionResources::parse(&mixed()).unwrap();
        assert_eq!(
            contract.job_limits(),
            JobLimits::from_canonical_json(&ordinary()).unwrap()
        );
        assert!(matches!(
            contract,
            AdmissionResources::Mixed {
                native_ram: NativeRamLimits {
                    max_live_bytes: 4096,
                    max_operation_bytes: 1024
                },
                ..
            }
        ));
        let mut zero = mixed();
        zero["native_ram"] = json!({"max_live_bytes":"0","max_operation_bytes":"0"});
        assert!(AdmissionResources::parse(&zero).is_ok());
    }
    #[test]
    fn mixed_resources_rejects_noncanonical_ram_and_unknown_contracts() {
        for value in [
            json!(0),
            json!("-1"),
            json!("+1"),
            json!("01"),
            json!(" 1"),
            json!("1.0"),
            json!(""),
            json!("18446744073709551616"),
        ] {
            let mut node = mixed();
            node["native_ram"]["max_live_bytes"] = value;
            assert!(AdmissionResources::parse(&node).is_err());
        }
        for domains in [
            json!(["legacy-evidence/1", "s-content/1"]),
            json!(["s-content/1"]),
            json!(["s-content/1", "legacy-evidence/1", "other"]),
            json!(["s-content/1", "s-content/1"]),
        ] {
            let mut node = mixed();
            node["measurement_domains"] = domains;
            assert!(AdmissionResources::parse(&node).is_err());
        }
        let mut node = mixed();
        node["native_ram"]["max_operation_bytes"] = json!("4097");
        assert!(AdmissionResources::parse(&node).is_err());
        let mut node = mixed();
        node["schema_revision"] = json!("s-admission-mixed-resources/2");
        assert!(AdmissionResources::parse(&node).is_err());
    }
    #[test]
    fn mixed_resources_has_no_missing_or_extra_field_defaults() {
        for key in [
            "measurement_domains",
            "job_limits",
            "native_ram",
            "schema_revision",
        ] {
            let mut node = mixed();
            node.as_object_mut().unwrap().remove(key);
            assert!(AdmissionResources::parse(&node).is_err());
        }
        for key in ["max_live_bytes", "max_operation_bytes"] {
            let mut node = mixed();
            node["native_ram"].as_object_mut().unwrap().remove(key);
            assert!(AdmissionResources::parse(&node).is_err());
        }
        for path in ["native_ram", "job_limits"] {
            let mut node = mixed();
            node[path]["extra"] = json!("1");
            assert!(AdmissionResources::parse(&node).is_err());
        }
        let mut node = mixed();
        node["extra"] = json!("1");
        assert!(AdmissionResources::parse(&node).is_err());
        let mut node = mixed();
        node["job_limits"]["max_total_records"] = json!("0");
        assert!(AdmissionResources::parse(&node).is_err());
    }
}
