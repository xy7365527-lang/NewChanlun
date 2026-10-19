//! #1394 C §5.6.1：真实 Unix I/O 的 job 计费与绝对期限（薄适配片）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md` §5.6.1。
//!
//! 本片把「一次真实的 reader/peer 尝试」接到同一个 [`Job`] 上：
//! - [`WireAttempt::begin`] 记一次 request（含错误与重试）并通过
//!   [`Job::first_read_deadline`] 取**同一个 job** 的唯一绝对期限；同一 attempt 内的
//!   read/write 不重复记 request，新重试必须新开 attempt 再记一次。
//! - 读写都走 `session_transport` 的唯一读循环与分块写循环：每次底层成功读写得到 `n>0` 后按
//!   **实际 n 字节**累计到 `max_total_wire_bytes`（LF、坏 JSON、validate 拒绝、超单帧上限、
//!   超时或 EOF 前的前缀、部分写后失败都保留已发生的字节；不按序列化预计长度充作已发长度）。
//! - 每次真正 I/O 之前先查停止状态与绝对期限；越限或过期先记下刚发生的物理字节，再停止并报错。
//!
//! 边界（照实）：
//! - 本片**只**计调用者交进来的那条 Unix 连接上的读写字节。HTTP 头、子进程 stderr、文件、
//!   数据库与本地缓存字节不在本片内，也不因本片存在而自动计入。
//! - **外层必须在 connect/spawn 之前开始计费**（先 [`WireAttempt::begin`] 或显式记 request），
//!   否则排队与连接等待不落入本 job；连接、spawn、HTTP、stderr 等阻塞步骤仍须各自带期限，
//!   本片不代替它们。
//! - 一个 [`Job`] 可以被多次 attempt 复用；本片不新建 job，也不把任何一次 attempt 的成功
//!   当作 job 的完成判据。

use super::job::Job;
use super::CodecError;
use crate::session_transport::{
    read_frame_observed, write_frame_observed, EnvelopeValidator, FrameObserver, FrameSink,
};
use serde_json::Value;
use std::fmt;
use std::os::unix::net::UnixStream;
use std::time::Instant;

/// 一次真实 reader/peer 尝试。
///
/// 生命周期即一次 attempt：`begin` 记一次 request 并取 job 的绝对期限；attempt 内可任意次
/// read/write（不重复记 request）；重试要新开 attempt（再记一次 request），但**复用同一个
/// [`Job`]**——余额与绝对期限都不因新 attempt 重置。
pub struct WireAttempt<'a> {
    job: &'a mut Job,
    deadline: Instant,
}

impl<'a> WireAttempt<'a> {
    /// 开始一次真实尝试：记一次 request（含错误与重试）并起/取 job 的唯一绝对期限。
    ///
    /// request 已在 [`Job::charge_requests`] 里计入；耗尽时本口在**任何 I/O 之前**拒绝，
    /// 不会建立 attempt。
    pub fn begin(job: &'a mut Job) -> Result<WireAttempt<'a>, CodecError> {
        job.charge_requests(1)?;
        let deadline = job.first_read_deadline()?;
        Ok(WireAttempt { job, deadline })
    }

    /// 本 job 的绝对期限（首次读取起算；逐次 attempt 不续期）。
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// 旧每操作期限与 job 绝对期限取更早者：两个限制同时生效，谁都不能延长另一个。
    pub fn effective_deadline(&self, operation_deadline: Instant) -> Instant {
        operation_deadline.min(self.deadline)
    }

    /// 本 attempt 所挂的 job（只读，供诊断与取快照）。
    pub fn job(&self) -> &Job {
        self.job
    }

    /// 一次真实读帧：复用 [`crate::session_transport::read_frame`] 的唯一读循环，
    /// 按每次实际读到的字节记账，并把 job 绝对期限与 `operation_deadline` 的较早者作为读期限。
    pub fn read_frame(
        &mut self,
        stream: &mut UnixStream,
        max: usize,
        operation_deadline: Instant,
        validate: EnvelopeValidator,
    ) -> Result<Value, String> {
        let deadline = self.effective_deadline(operation_deadline);
        let mut observer = JobWireObserver { job: self.job };
        let outcome = read_frame_observed(stream, max, deadline, validate, &mut observer);
        settle(self.job, outcome)
    }

    /// 一次真实分块写：每次真正写成功 `n` 字节后按实际 `n` 记账（不是按序列化预计长度）。
    ///
    /// `sink` 传调用者自己的连接（[`UnixStream`] 已实现 [`FrameSink`]）；测试也可注入可确定
    /// 复现的部分写与失败目标。
    pub fn write_frame(
        &mut self,
        sink: &mut dyn FrameSink,
        body: &[u8],
        operation_deadline: Instant,
    ) -> Result<(), String> {
        let deadline = self.effective_deadline(operation_deadline);
        let mut observer = JobWireObserver { job: self.job };
        let outcome = write_frame_observed(sink, body, deadline, &mut observer);
        settle(self.job, outcome)
    }
}

impl fmt::Debug for WireAttempt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let snapshot = self.job.snapshot();
        f.debug_struct("WireAttempt")
            .field("requests", &snapshot.requests)
            .field("wire_bytes", &snapshot.wire_bytes)
            .field("deadline", &self.deadline)
            .finish()
    }
}

/// 一次尝试结束时的落定：若 job 的绝对期限在本次 I/O 期间走完，就地把 job 停在 `job_deadline`
/// 上（含成功校验或零字节写返回），并以此错误返回；同时保留底层已有错误诊断。
fn settle<T>(job: &mut Job, outcome: Result<T, String>) -> Result<T, String> {
    match job.pre_io_check() {
        Ok(()) => outcome,
        Err(stop) => match outcome {
            Err(err) if !err.starts_with(stop.code()) => Err(format!("{stop}；原始错误：{err}")),
            Err(err) => Err(err),
            Ok(_) => Err(stop.to_string()),
        },
    }
}

/// 观测口实现：真正 I/O 前查停止/期限；每次成功 `n` 字节后按实际 `n` 累计 wire。
struct JobWireObserver<'a> {
    job: &'a mut Job,
}

impl FrameObserver for JobWireObserver<'_> {
    fn before_io(&mut self) -> Result<(), String> {
        self.job.pre_io_check().map_err(|e| e.to_string())
    }

    fn after_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        // 底层单次读/写的上界是 4096 字节（session_transport 的固定块），u64 转换不可能截断；
        // 物理字节已经发生：先照实入账，越限/过期由 charge_wire_bytes 记录后再停。
        let actual = u64::try_from(bytes.len()).map_err(|e| format!("job_wire：{e}"))?;
        self.job
            .charge_wire_bytes(actual)
            .map_err(|e| e.to_string())
    }
}
