//! #1527 B2：源码锁，NOT_RUN；覆盖实际短写/错误返回，而不是日志长度。
use super::*;
use newchan_rust::s_content::job::{Existing, Job};

struct ShortSink(Vec<u8>);
impl io::Write for ShortSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len() == 5 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let n = (5 - self.0.len()).min(bytes.len());
        self.0.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn retain_test_job(limit: u64) -> Rc<RefCell<Job>> {
    let limits = JobLimits::from_canonical_json(&serde_json::json!({
        "max_total_wire_bytes":limit.to_string(),"max_total_node_bytes":"1024",
        "max_total_nodes":"16","max_total_records":"16","max_total_requests":"1",
        "max_elapsed_ms":"60000","max_staged_bytes":"1024","max_store_bytes":"4096"
    }))
    .unwrap();
    let first = Instant::now();
    let job = Rc::new(RefCell::new(
        Job::new_started(
            limits,
            Existing {
                store_bytes: 17,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
            first,
        )
        .unwrap(),
    ));
    retain_terminal_job(&job, first);
    job
}
#[test]
fn original_job_outlives_startup_owner_until_partial_terminal_write_returns() {
    let command = CommandAccounting::begin();
    let job = retain_test_job(10000);
    job.borrow_mut().charge_wire_bytes(17).unwrap();
    let weak = Rc::downgrade(&job);
    drop(job);
    assert!(weak.upgrade().is_some());
    let mut sink = ShortSink(Vec::new());
    let receipt = command.report("原业务失败", &mut sink);
    assert_eq!(sink.0.len(), 5);
    assert_eq!(receipt.actual_stderr_bytes, 5);
    assert_eq!(receipt.unaccepted_stderr_bytes, 0);
    assert_eq!(receipt.job.unwrap().wire_bytes, 22);
    assert!(receipt.write_error.is_some());
    assert!(weak.upgrade().is_none());
}
#[test]
fn stopped_job_keeps_first_error_and_separately_observes_real_diagnostic_write() {
    let command = CommandAccounting::begin();
    let job = retain_test_job(16);
    job.borrow_mut().charge_wire_bytes(17).unwrap_err();
    let before = job.borrow().snapshot();
    drop(job);
    let receipt = command.report("原业务失败", &mut ShortSink(Vec::new()));
    let after = receipt.job.unwrap();
    assert_eq!(after.wire_bytes, before.wire_bytes);
    assert_eq!(after.stopped, before.stopped);
    assert_eq!(receipt.actual_stderr_bytes, 5);
    assert_eq!(receipt.unaccepted_stderr_bytes, 5);
    assert!(receipt.accounting_error.is_some());
}
#[test]
fn terminal_limit_failure_counts_the_write_before_stopping() {
    let command = CommandAccounting::begin();
    let job = retain_test_job(20);
    job.borrow_mut().charge_wire_bytes(17).unwrap();
    drop(job);
    let receipt = command.report("原业务失败", &mut ShortSink(Vec::new()));
    assert_eq!(receipt.job.unwrap().wire_bytes, 22);
    assert_eq!(receipt.actual_stderr_bytes, 5);
    assert_eq!(receipt.unaccepted_stderr_bytes, 0);
    assert!(receipt.accounting_error.is_some());
}
#[test]
fn untrusted_prelude_remains_no_job_and_preserves_partial_read_and_stage() {
    let command = CommandAccounting::begin();
    let mut prelude = ControlPrelude::new();
    prelude.first_read_at = Some(Instant::now());
    read_counted(&mut io::Cursor::new(b"bad"), &mut prelude, Stage::E1).unwrap();
    let error = prelude
        .error(ErrorKind::AnchorInvalid, Stage::E1)
        .to_string();
    drop(prelude);
    let receipt = command.report(&error, &mut ShortSink(Vec::new()));
    let p = receipt.prelude.unwrap();
    assert_eq!(p.wire_bytes, 3);
    assert_eq!(p.stage, Stage::E1);
    assert!(!p.budget_trusted);
    assert!(receipt.job.is_none());
    assert_eq!(receipt.actual_stderr_bytes, 5);
}
