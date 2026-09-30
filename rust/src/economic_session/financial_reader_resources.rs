//! #1394 §5.6.1：新财务 reader 的独立冻结资源合同。
//! 本模块仅解析合同；具体数值须由受测域另行具名冻结。
use crate::s_content::job::JobLimits;
use serde_json::Value;

const REVISION: &str = "s-financial-reader-resources/1";
const FIELDS: [&str; 6] = [
    "schema_revision",
    "max_request_bytes",
    "max_reply_bytes",
    "max_stderr_bytes",
    "request_timeout_ms",
    "job_limits",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FinancialReaderResources {
    pub max_request_bytes: u64,
    pub max_reply_bytes: u64,
    pub max_stderr_bytes: u64,
    pub request_timeout_ms: u64,
    pub job_limits: JobLimits,
}

impl FinancialReaderResources {
    pub(super) fn from_json(value: &Value) -> Result<Self, String> {
        let fields = value
            .as_object()
            .filter(|fields| {
                fields.len() == FIELDS.len()
                    && fields.keys().all(|key| FIELDS.contains(&key.as_str()))
            })
            .ok_or("SchemaUnsupported：financial reader_resources 必须 exact 六字段")?;
        if fields["schema_revision"] != REVISION {
            return Err("SchemaUnsupported：financial reader_resources 版本不符".into());
        }
        let result = Self {
            max_request_bytes: positive_text(&fields["max_request_bytes"], "max_request_bytes")?,
            max_reply_bytes: positive_text(&fields["max_reply_bytes"], "max_reply_bytes")?,
            max_stderr_bytes: positive_text(&fields["max_stderr_bytes"], "max_stderr_bytes")?,
            request_timeout_ms: positive_text(&fields["request_timeout_ms"], "request_timeout_ms")?,
            job_limits: JobLimits::from_canonical_json(&fields["job_limits"])
                .map_err(|e| format!("SchemaUnsupported：financial job_limits：{e}"))?,
        };
        result.validate()?;
        Ok(result)
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("max_request_bytes", self.max_request_bytes),
            ("max_reply_bytes", self.max_reply_bytes),
            ("max_stderr_bytes", self.max_stderr_bytes),
            ("request_timeout_ms", self.request_timeout_ms),
        ] {
            if value == 0 || value > i64::MAX as u64 {
                return Err(format!("InvalidDomain：{name} 必须在 1..=2^63-1"));
            }
        }
        self.job_limits.validate().map_err(|e| e.to_string())
    }
}

fn positive_text(value: &Value, name: &str) -> Result<u64, String> {
    let text = value
        .as_str()
        .filter(|text| !text.is_empty() && (text == &"0" || !text.starts_with('0')))
        .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| format!("SchemaUnsupported：{name} 必须为规范正整数文本"))?;
    let number = text
        .parse::<u64>()
        .map_err(|_| format!("InvalidDomain：{name} 超出整数域"))?;
    if number == 0 || number > i64::MAX as u64 {
        return Err(format!("InvalidDomain：{name} 必须在 1..=2^63-1"));
    }
    Ok(number)
}
