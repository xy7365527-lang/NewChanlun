//! #1527：mixed 终端槽在父账构造前计入固定 Layout；停止后只写预置内存。
use super::*;
use std::borrow::Cow;
use std::fmt::Write as _;

const DETAIL_BYTES: usize = 512;
const PAYLOAD_BYTES: usize = 32768;
#[derive(Clone, Copy)]
pub(crate) struct FixedFailure {
    pub(crate) code: &'static str,
    pub(crate) phase: &'static str,
    detail: [u8; DETAIL_BYTES],
    len: usize,
    truncated: bool,
}
impl FixedFailure {
    pub(crate) fn new(code: &'static str, phase: &'static str, detail: &str) -> Self {
        let mut len = detail.len().min(DETAIL_BYTES);
        while !detail.is_char_boundary(len) {
            len -= 1;
        }
        let mut out = Self {
            code,
            phase,
            detail: [0; DETAIL_BYTES],
            len,
            truncated: len < detail.len(),
        };
        out.detail[..len].copy_from_slice(&detail.as_bytes()[..len]);
        out
    }
    pub(crate) fn formatted(
        code: &'static str,
        phase: &'static str,
        detail: fmt::Arguments<'_>,
    ) -> Self {
        let mut buffer = FixedBuffer::<DETAIL_BYTES>::new();
        let truncated = buffer.write_fmt(detail).is_err();
        Self {
            code,
            phase,
            detail: buffer.bytes,
            len: buffer.len,
            truncated,
        }
    }
    pub(crate) fn detail(&self) -> &str {
        std::str::from_utf8(&self.detail[..self.len]).expect("bounded UTF-8 copy")
    }
}
pub(super) struct MixedTerminal {
    first: Option<FixedFailure>,
    later: [Option<FixedFailure>; 7],
    unaccepted_read_wire: u128,
    unaccepted_read_overflow: bool,
    payload: FixedBuffer<PAYLOAD_BYTES>,
}
struct FixedBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> FixedBuffer<N> {
    fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
    fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}
impl<const N: usize> fmt::Write for FixedBuffer<N> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self
            .len
            .checked_add(text.len())
            .filter(|end| *end <= N)
            .ok_or(fmt::Error)?;
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn json_string(out: &mut impl fmt::Write, text: &str) -> fmt::Result {
    out.write_char('"')?;
    for ch in text.chars() {
        match ch {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\u{0}'..='\u{1f}' => write!(out, "\\u{:04x}", ch as u32)?,
            _ => out.write_char(ch)?,
        }
    }
    out.write_char('"')
}
impl MixedTerminal {
    pub(super) fn new() -> Self {
        Self {
            first: None,
            later: [None; 7],
            unaccepted_read_wire: 0,
            unaccepted_read_overflow: false,
            payload: FixedBuffer::new(),
        }
    }
    pub(super) fn note(&mut self, failure: FixedFailure) {
        if self.first.is_none() {
            self.first = Some(failure);
        }
    }
    fn payload(
        &mut self,
        fallback: &str,
        snapshot: Option<&admission_job::JobSnapshot>,
        ram: Option<&ram::RamSnapshot>,
        job_native: Option<usize>,
        retained: Option<usize>,
    ) -> fmt::Result {
        let failure = self
            .first
            .unwrap_or_else(|| FixedFailure::new("mixed_admission", "terminal", fallback));
        let out = &mut self.payload;
        out.write_str("{\"ok\":false,\"error\":")?;
        json_string(out, failure.detail())?;
        out.write_str(",\"code\":")?;
        json_string(out, failure.code)?;
        out.write_str(",\"phase\":")?;
        json_string(out, failure.phase)?;
        out.write_str(",\"later\":[")?;
        let mut separator = false;
        for later in self.later.iter().flatten() {
            if separator {
                out.write_char(',')?;
            }
            separator = true;
            out.write_str("{\"code\":")?;
            json_string(out, later.code)?;
            out.write_str(",\"phase\":")?;
            json_string(out, later.phase)?;
            out.write_str(",\"detail\":")?;
            json_string(out, later.detail())?;
            write!(out, ",\"detail_truncated\":{}}}", later.truncated)?;
        }
        write!(
            out,
            "],\"unaccepted_read_wire\":{},\"unaccepted_read_overflow\":{}",
            self.unaccepted_read_wire, self.unaccepted_read_overflow
        )?;
        write!(
            out,
            ",\"detail_truncated\":{},\"admission_accounting\":{{\"job_present\":{}",
            failure.truncated,
            snapshot.is_some()
        )?;
        if let Some(s) = snapshot {
            write!(out, ",\"before_stderr\":{{\"wire_bytes\":{},\"node_bytes\":{},\"nodes\":{},\"records\":{},\"requests\":{},\"open_reservations\":{},\"stopped\":",
                s.wire_bytes, s.node_bytes, s.nodes, s.records, s.requests, s.open_reservations)?;
            match s.stopped {
                Some(code) => json_string(out, code)?,
                None => out.write_str("null")?,
            }
            out.write_char('}')?;
        }
        if let Some(ram) = ram {
            write!(
                out,
                ",\"ram_before_stderr\":{{\"final_live\":{},\"retained_live\":",
                ram.job_live
            )?;
            match retained {
                Some(bytes) => write!(out, "{bytes}")?,
                None => out.write_str("null")?,
            }
            write!(
                out,
                ",\"fixed_metadata_bytes\":{},\"terminal_job_native_bytes\":",
                ram.fixed_metadata
            )?;
            match job_native {
                Some(bytes) => write!(out, "{bytes}")?,
                None => out.write_str("null")?,
            }
            out.write_str(",\"retention_error\":")?;
            if retained.is_some() {
                out.write_str("null")?;
            } else {
                json_string(
                    out,
                    if ram.active {
                        "MixedRamOperationOpen"
                    } else {
                        "MixedRamBalanceInvalid"
                    },
                )?;
            }
            write!(out, ",\"parts\":{{\"metadata_bytes\":{},\"layout_bytes\":{},\"native_bytes\":{}}},\"job_peak\":{},\"operation_live\":{},\"operation_peak\":{},\"operation_closed\":{},\"failed\":{}}}",
                ram.parts[0], ram.parts[1], ram.parts[2], ram.job_peak,
                ram.operation_live, ram.operation_peak, !ram.active, ram.failed.is_some())?;
        }
        out.write_str("}}\n")
    }
    pub(super) fn report(
        mut self,
        error: &str,
        sink: &mut impl io::Write,
        job: Option<&Rc<RefCell<admission_job::Job>>>,
        mut receipt: TerminalReceipt,
    ) -> TerminalReceipt {
        let before = job.map(|j| j.borrow().snapshot());
        let job_native = job.and_then(|j| j.borrow().mixed_tracking_native_bytes().ok());
        let retained = receipt.ram_before_stderr.as_ref().and_then(|ram| {
            if ram.active {
                return None;
            }
            ram.job_live
                .checked_sub(ram.fixed_metadata)?
                .checked_sub(job_native?)
        });
        if let Some(ram) = receipt.ram_before_stderr.as_ref() {
            if retained.is_none() {
                receipt
                    .accounting_error
                    .get_or_insert(Cow::Borrowed(if ram.active {
                        "MixedRamOperationOpen"
                    } else {
                        "MixedRamBalanceInvalid"
                    }));
            }
        }
        let payload_ok = self
            .payload(
                error,
                before.as_ref(),
                receipt.ram_before_stderr.as_ref(),
                job_native,
                retained,
            )
            .is_ok();
        // 固定容量内部不变量失败也只用静态字节；不能走 String/JSON fallback。
        let payload = if payload_ok {
            self.payload.bytes()
        } else {
            b"{\"ok\":false,\"code\":\"mixed_terminal_capacity\"}\n"
        };
        let mut offset = 0;
        while offset < payload.len() {
            match sink.write(&payload[offset..]) {
                Ok(n) => {
                    receipt.actual_stderr_bytes += n as u128;
                    if let Some(job) = job {
                        let mut job = job.borrow_mut();
                        let before = job.snapshot().wire_bytes;
                        if let Err(e) = job.charge_wire_bytes(n as u64) {
                            receipt
                                .accounting_error
                                .get_or_insert(Cow::Borrowed(e.code()));
                        }
                        let accepted = job.snapshot().wire_bytes - before;
                        receipt.unaccepted_stderr_bytes += (n as u128).saturating_sub(accepted);
                    } else {
                        receipt.unaccepted_stderr_bytes += n as u128;
                    }
                    if n == 0 {
                        receipt.write_error = Some(Cow::Borrowed("TerminalWriteZero"));
                        break;
                    }
                    offset += n;
                }
                Err(_) => {
                    receipt.write_error = Some(Cow::Borrowed("TerminalWriteFailed"));
                    break;
                }
            }
        }
        if let Some(job) = job {
            let mut job = job.borrow_mut();
            if let Err(e) = job.finish_check() {
                receipt
                    .accounting_error
                    .get_or_insert(Cow::Borrowed(e.code()));
            }
            receipt.job = Some(job.snapshot());
        }
        receipt
    }
}

/// 只锁存既有 mixed 命令的固定槽；调用方保留 Err，不另造 Job 或复活 operation。
pub(crate) fn note_failure(failure: FixedFailure) {
    TERMINAL.with(|slot| {
        if let Some(mixed) = slot.borrow_mut().as_mut().and_then(|s| s.mixed.as_mut()) {
            mixed.note(failure);
        }
    });
}

pub(crate) fn note_cleanup(later: [Option<FixedFailure>; 7], unaccepted: u128, overflow: bool) {
    TERMINAL.with(|slot| {
        if let Some(mixed) = slot.borrow_mut().as_mut().and_then(|s| s.mixed.as_mut()) {
            for (destination, value) in mixed.later.iter_mut().zip(later) {
                if destination.is_none() {
                    *destination = value;
                }
            }
            mixed.unaccepted_read_wire = unaccepted;
            mixed.unaccepted_read_overflow = overflow;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::job::{Existing, Job};
    struct ShortSink {
        bytes: [u8; 5],
        len: usize,
    }
    impl io::Write for ShortSink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.len == 5 {
                return Err(io::ErrorKind::Interrupted.into());
            }
            let n = (5 - self.len).min(bytes.len());
            self.bytes[self.len..self.len + n].copy_from_slice(&bytes[..n]);
            self.len += n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn mixed_terminal_preserves_real_short_write_and_stopped_job() {
        for stop_before in [false, true] {
            let command = CommandAccounting::begin();
            let limits = JobLimits::from_canonical_json(&serde_json::json!({
                "max_total_wire_bytes":"20","max_total_node_bytes":"1024",
                "max_total_nodes":"16","max_total_records":"16","max_total_requests":"1",
                "max_elapsed_ms":"60000","max_staged_bytes":"1024","max_store_bytes":"4096"
            }))
            .unwrap();
            let ram = ram::AdmissionRamLedger::new_with_fixed(
                resources::NativeRamLimits {
                    max_live_bytes: 1 << 20,
                    max_operation_bytes: 1 << 20,
                },
                mixed_fixed_metadata().unwrap(),
            )
            .unwrap();
            let first = Instant::now();
            let job = ram
                .codec_work()
                .unwrap()
                .run(|work| {
                    Job::new_started_mixed(
                        limits,
                        Existing {
                            store_bytes: 0,
                            staged_bytes: 0,
                        },
                        Box::new(Instant::now),
                        first,
                        work,
                    )
                })
                .unwrap();
            let job = Rc::new(RefCell::new(job));
            retain_terminal_job(&job, first, true);
            let initial = if stop_before { 21 } else { 17 };
            let result = job.borrow_mut().charge_wire_bytes(initial);
            assert_eq!(result.is_err(), stop_before);
            note_failure(FixedFailure::new("original", "business", "first"));
            note_failure(FixedFailure::new("later", "cleanup", "second"));
            ram.close().unwrap();
            let mut sink = ShortSink {
                bytes: [0; 5],
                len: 0,
            };
            let receipt = command.report("must not replace first", &mut sink);
            assert_eq!(receipt.actual_stderr_bytes, 5);
            assert_eq!(
                receipt.unaccepted_stderr_bytes,
                if stop_before { 5 } else { 0 }
            );
            assert_eq!(
                receipt.job.as_ref().unwrap().wire_bytes,
                if stop_before { 21 } else { 22 }
            );
            assert!(matches!(receipt.accounting_error, Some(Cow::Borrowed(_))));
            assert!(matches!(receipt.write_error, Some(Cow::Borrowed(_))));
        }
    }
    #[test]
    fn mixed_terminal_fixed_utf8_json_bound_and_first_failure() {
        let mut terminal = MixedTerminal::new();
        let detail = "界\n\"\\\u{0}".repeat(200);
        terminal.note(FixedFailure::new("original", "business", &detail));
        terminal.note(FixedFailure::new("later", "cleanup", "second"));
        terminal
            .payload("fallback", None, None, None, None)
            .unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(terminal.payload.bytes()).unwrap();
        assert_eq!(decoded["code"], "original");
        assert_eq!(decoded["phase"], "business");
        assert_eq!(decoded["detail_truncated"], true);
        assert!(detail.starts_with(decoded["error"].as_str().unwrap()));
    }
}
