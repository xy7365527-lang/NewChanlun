//! #1394：Q v3共用的完整请求校验与完整回复编码；不执行HTTP或虚构wire账。
use super::q_resources::{Limit, QResources};
use newchan_rust::{
    s_content::job::Job,
    session_protocol::{positive, reply_envelope, strict_json, validate_envelope},
};
use serde_json::Value;
use std::io::{self, Write};

pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Value, String> {
    if bytes.len() as u128 > u128::from(resources.limit(Limit::FrameBytes)) {
        return Err(err("Incomplete：完整请求超过max_frame_bytes"));
    }
    validate_envelope(strict_json(bytes)?, "s-session/3")
}
/// payload必须由已核切面生成；源namespace/消息ID/因果回引沿共同Q配方。
pub(super) fn reply(
    request: &Value,
    payload: Value,
    session: &str,
    incarnation: &str,
    epoch: &str,
    resources: &QResources,
    job: &mut Job,
) -> Result<Vec<u8>, String> {
    try_reply(
        request,
        payload,
        session,
        incarnation,
        epoch,
        resources,
        job,
    )?
    .ok_or_else(|| err("Incomplete：完整Q消息体超过max_reply_bytes"))
}
/// None只表示完整消息字节装不下；期限/身份/编码等错误不得借此降级为截短成功。
pub(super) fn try_reply(
    request: &Value,
    payload: Value,
    session: &str,
    incarnation: &str,
    epoch: &str,
    resources: &QResources,
    job: &mut Job,
) -> Result<Option<Vec<u8>>, String> {
    positive(epoch, "Q producer_epoch")?;
    if encode(&payload, resources.limit(Limit::ReplyBytes), job)?.is_none() {
        return Ok(None);
    }
    let envelope = reply_envelope(
        request,
        payload,
        "s-session/3",
        session,
        incarnation,
        "s-observe/replies",
        &format!("Q:{session}"),
        epoch,
    );
    encode(&envelope, resources.limit(Limit::ReplyBytes), job)
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QReplyInvalid：{e}")
}

fn encode(value: &Value, limit: u64, job: &mut Job) -> Result<Option<Vec<u8>>, String> {
    struct Buffer<'a> {
        bytes: Vec<u8>,
        limit: u64,
        job: &'a mut Job,
        exceeded: bool,
    }
    impl Write for Buffer<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.job.pre_io_check().map_err(io::Error::other)?;
            if self.bytes.len() as u128 + bytes.len() as u128 > u128::from(self.limit) {
                self.exceeded = true;
                return Err(io::Error::other(
                    "Incomplete：完整Q消息体超过max_reply_bytes",
                ));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    // 与共同canonical_json相同的排序和标量编码，逐段写入显式上界，避免先展开整份输出。
    fn write(value: &Value, out: &mut impl Write) -> io::Result<()> {
        match value {
            Value::Object(map) => {
                out.write_all(b"{")?;
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort();
                for (i, key) in keys.into_iter().enumerate() {
                    if i > 0 {
                        out.write_all(b",")?;
                    }
                    serde_json::to_writer(&mut *out, key).map_err(io::Error::other)?;
                    out.write_all(b":")?;
                    write(&map[key], out)?;
                }
                out.write_all(b"}")
            }
            Value::Array(values) => {
                out.write_all(b"[")?;
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        out.write_all(b",")?;
                    }
                    write(value, out)?;
                }
                out.write_all(b"]")
            }
            _ => serde_json::to_writer(out, value).map_err(io::Error::other),
        }
    }
    let mut out = Buffer {
        bytes: Vec::new(),
        limit,
        job,
        exceeded: false,
    };
    let result = write(value, &mut out);
    out.job.pre_io_check().map_err(err)?;
    if out.exceeded {
        return Ok(None);
    }
    result.map_err(err)?;
    Ok(Some(out.bytes))
}
