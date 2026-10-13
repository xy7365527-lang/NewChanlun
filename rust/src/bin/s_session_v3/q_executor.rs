//! #1394：固定worker、有界Q队列，caller budget真实持有调度owner。
//! None/Some沿同核；先清reader/释放active+queued，再发布终态；Drop排空已接纳任务。
//! 外部QResources Arc、std线程启动packet/name、OS栈、caller传入的PathBuf、
//! std String诊断与第三方io错误未资格化；此模块不授权任何部署默认额度。
use super::{
    capture::AuditReader,
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
};
use newchan_rust::s_content::{
    capture_allocation::{
        dispatch::{DispatchArc, DispatchOwner, DispatchPath, DispatchQueue, DispatchTask},
        OwnedBudget,
    },
    CodecVec,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

struct ResultState<R> {
    result: Option<Result<R, String>>,
    published: bool,
    disconnected: bool,
}
struct ResultCell<R> {
    state: Mutex<ResultState<R>>,
    wake: Condvar,
}
impl<R> ResultCell<R> {
    fn new() -> Self {
        Self {
            state: Mutex::new(ResultState {
                result: None,
                published: false,
                disconnected: false,
            }),
            wake: Condvar::new(),
        }
    }
}
/// Cell的实际refcount/header与R存储同budget持有，直到listener消费/丢弃Pending。
pub(super) struct Pending<R> {
    cell: DispatchArc<ResultCell<R>>,
}
impl<R> Pending<R> {
    pub(super) fn try_result(&mut self) -> Option<Result<R, String>> {
        let mut s = self.cell.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(r) = s.result.take() {
            return Some(r);
        }
        if s.disconnected || s.published {
            return Some(Err("Incomplete：Q executor worker 在返回结果前退出".into()));
        }
        None
    }
    pub(super) fn recv(self) -> Result<R, String> {
        let mut s = self.cell.state.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if let Some(r) = s.result.take() {
                return r;
            }
            if s.disconnected || s.published {
                return Err("Incomplete：Q executor worker 在返回结果前退出".into());
            }
            s = self.cell.wake.wait(s).unwrap_or_else(|p| p.into_inner());
        }
    }
}
struct Job<T, R, F> {
    input: Option<T>,
    task: Option<F>,
    cell: DispatchArc<ResultCell<R>>,
    staged: Option<Result<R, String>>,
    published: bool,
}
impl<T, R, F> DispatchTask<AuditReader, QResources, ClientLeases> for Job<T, R, F>
where
    T: Send + 'static,
    R: Send + 'static,
    F: FnOnce(T, &mut AuditReader, &QResources, &ClientLeases, Instant) -> Result<R, String>
        + Send
        + 'static,
{
    fn run(
        &mut self,
        reader: &mut AuditReader,
        resources: &QResources,
        leases: &ClientLeases,
        at: Instant,
    ) -> bool {
        let input = self.input.take().expect("accepted task owns input");
        let task = self.task.take().expect("task runs once");
        match catch_unwind(AssertUnwindSafe(|| {
            task(input, reader, resources, leases, at)
        })) {
            Ok(result) => {
                self.staged = Some(result);
                false
            }
            Err(_) => {
                self.staged = Some(Err(
                    "Incomplete：Q executor task panic；已清理本 worker reader".into(),
                ));
                true
            }
        }
    }
    fn reject(&mut self, reason: String) -> bool {
        // 各自take后析构，即使input/drop panic也继续清掉task捕获，避免残留析构二次panic。
        let input = self.input.take();
        let task = self.task.take();
        let input_panic = catch_unwind(AssertUnwindSafe(|| drop(input))).is_err();
        let task_panic = catch_unwind(AssertUnwindSafe(|| drop(task))).is_err();
        let reset = input_panic || task_panic;
        self.staged = Some(Err(if reset {
            "Incomplete：Q executor task panic；已清理本 worker reader".into()
        } else {
            reason
        }));
        reset
    }
    fn complete(&mut self) {
        let mut s = self.cell.state.lock().unwrap_or_else(|p| p.into_inner());
        s.result = self.staged.take();
        s.published = true;
        self.published = true;
        drop(s);
        self.cell.wake.notify_all();
    }
}
impl<T, R, F> Drop for Job<T, R, F> {
    fn drop(&mut self) {
        if !self.published {
            let mut s = self.cell.state.lock().unwrap_or_else(|p| p.into_inner());
            s.disconnected = true;
            drop(s);
            self.cell.wake.notify_all();
        }
    }
}
type TaskOwner = DispatchOwner<AuditReader, QResources, ClientLeases>;
struct QueuedTask {
    queued_at: Instant,
    owner: TaskOwner,
}
struct QueueState {
    closed: bool,
    active_queued: u64,
    queue: DispatchQueue<QueuedTask>,
}
struct Shared {
    state: Mutex<QueueState>,
    wake: Condvar,
    capacity: u64,
    queue_timeout: Duration,
}
pub(super) struct Executor {
    shared: DispatchArc<Shared>,
    resources: Arc<QResources>,
    leases: DispatchArc<ClientLeases>,
    workers: CodecVec<JoinHandle<()>>,
    allocation: Option<super::q_allocation::Profile>,
}
impl Executor {
    pub(super) fn new(path: PathBuf, resources: Arc<QResources>) -> Result<Self, String> {
        Self::new_with_allocation(path, resources, None)
    }
    pub(crate) fn new_with_allocation(
        path: PathBuf,
        resources: Arc<QResources>,
        allocation: Option<super::q_allocation::Profile>,
    ) -> Result<Self, String> {
        Self::build(path, resources, allocation, None)
    }
    fn build(
        path: PathBuf,
        resources: Arc<QResources>,
        allocation: Option<super::q_allocation::Profile>,
        fail_spawn_at: Option<usize>,
    ) -> Result<Self, String> {
        if let Some(p) = &allocation {
            p.validate()?;
        }
        let budget = allocation.as_ref().map(|p| &p.budget);
        let worker_count = resources.limit(Limit::QueryWorkers);
        let count = usize::try_from(worker_count)
            .map_err(|_| "Incomplete：max_query_workers 超出本机线程索引范围".to_string())?;
        let capacity = resources.limit(Limit::PendingConnections).min(
            worker_count
                .checked_add(resources.limit(Limit::QueryQueue))
                .ok_or_else(|| "Incomplete：Q executor active+queued 容量溢出".to_string())?,
        );
        if capacity == 0 || count == 0 {
            return Err("Incomplete：Q executor worker/active+queued 容量必须为正".into());
        }
        let shared = DispatchArc::try_new(
            Shared {
                state: Mutex::new(QueueState {
                    closed: false,
                    active_queued: 0,
                    queue: DispatchQueue::new(budget),
                }),
                wake: Condvar::new(),
                capacity,
                queue_timeout: Duration::from_millis(resources.limit(Limit::SocketReadMs)),
            },
            budget,
        )
        .map_err(|(e, _)| e.to_string())?;
        let leases = DispatchArc::try_new(
            ClientLeases::with_allocation(resources.as_ref(), budget)?,
            budget,
        )
        .map_err(|(e, _)| e.to_string())?;
        let path = DispatchPath::try_from_path(path, budget).map_err(|(e, _)| e.to_string())?;
        let path = DispatchArc::try_new(path, budget).map_err(|(e, _)| e.to_string())?;
        let mut workers: CodecVec<JoinHandle<()>> = CodecVec::new_with_allocation(budget);
        for index in 0..count {
            // Handles在spawn之前reserve；失败不能遗失一个已经运行的worker。
            if let Err(error) = workers.try_reserve(1) {
                close_shared(&shared);
                for handle in workers {
                    let _ = handle.join();
                }
                return Err(error.to_string());
            }
            let worker_shared = shared.clone();
            let worker_resources = Arc::clone(&resources);
            let worker_leases = leases.clone();
            let worker_path = path.clone();
            let worker_allocation = allocation.clone();
            let name = format!("s-query-{index}");
            let spawned = if fail_spawn_at == Some(index) {
                Err(std::io::Error::other("injected worker spawn failure"))
            } else {
                thread::Builder::new().name(name).spawn(move || {
                    worker_loop(
                        worker_path,
                        worker_resources,
                        worker_leases,
                        worker_shared,
                        worker_allocation,
                    )
                })
            };
            match spawned {
                Ok(handle) => {
                    // reserve成功之后push不会再分配，也不会detach句柄。
                    if workers.try_push(handle).is_err() {
                        unreachable!("pre-reserved worker handle slot");
                    }
                }
                Err(error) => {
                    close_shared(&shared);
                    for handle in workers {
                        let _ = handle.join();
                    }
                    return Err(format!("Incomplete：Q executor worker 创建失败: {error}"));
                }
            }
        }
        Ok(Self {
            shared,
            resources,
            leases,
            workers,
            allocation,
        })
    }
    pub(super) fn leases(&self) -> &ClientLeases {
        &self.leases
    }
    pub(super) fn resources(&self) -> &QResources {
        &self.resources
    }
    pub(crate) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        self.allocation.as_ref().map(|p| &p.budget)
    }
    pub(super) fn try_submit<R, F>(&self, task: F) -> Result<Pending<R>, String>
    where
        R: Send + 'static,
        F: FnOnce(&mut AuditReader, &QResources, &ClientLeases, Instant) -> Result<R, String>
            + Send
            + 'static,
    {
        self.try_submit_at(Instant::now(), task)
    }
    fn try_submit_at<R, F>(&self, queued_at: Instant, task: F) -> Result<Pending<R>, String>
    where
        R: Send + 'static,
        F: FnOnce(&mut AuditReader, &QResources, &ClientLeases, Instant) -> Result<R, String>
            + Send
            + 'static,
    {
        self.try_submit_owned_at(queued_at, (), move |(), reader, resources, leases, at| {
            task(reader, resources, leases, at)
        })
        .map_err(|(e, ())| e)
    }
    pub(super) fn try_submit_owned<T, R, F>(
        &self,
        input: T,
        task: F,
    ) -> Result<Pending<R>, (String, T)>
    where
        T: Send + 'static,
        R: Send + 'static,
        F: FnOnce(T, &mut AuditReader, &QResources, &ClientLeases, Instant) -> Result<R, String>
            + Send
            + 'static,
    {
        self.try_submit_owned_at(Instant::now(), input, task)
    }
    fn try_submit_owned_at<T, R, F>(
        &self,
        queued_at: Instant,
        input: T,
        task: F,
    ) -> Result<Pending<R>, (String, T)>
    where
        T: Send + 'static,
        R: Send + 'static,
        F: FnOnce(T, &mut AuditReader, &QResources, &ClientLeases, Instant) -> Result<R, String>
            + Send
            + 'static,
    {
        let mut state = match self.shared.state.lock() {
            Ok(s) => s,
            Err(_) => return Err(("Incomplete：Q executor 队列锁已中毒".into(), input)),
        };
        if state.closed {
            return Err(("Incomplete：Q executor 已关闭".into(), input));
        }
        if state.active_queued >= self.shared.capacity {
            return Err((
                "Incomplete：Q executor active+queued 达到冻结上限".into(),
                input,
            ));
        }
        let budget = self.allocation_budget();
        if let Err(e) = state.queue.reserve_one() {
            return Err((e.to_string(), input));
        }
        let cell = match DispatchArc::try_new(ResultCell::new(), budget) {
            Ok(c) => c,
            Err((e, _)) => return Err((e.to_string(), input)),
        };
        let job = Job {
            input: Some(input),
            task: Some(task),
            cell: cell.clone(),
            staged: None,
            published: false,
        };
        let owner = match DispatchOwner::try_new(job, budget) {
            Ok(owner) => owner,
            Err((e, mut job)) => {
                let input = job.input.take().expect("unaccepted input retained");
                // 未接纳闭包的捕获值是用户代码：先解锁，再析构；可重入而不自锁。
                // 清理panic不能毒化队列锁，也不能替换原Layout拒绝原因/原input。
                drop(state);
                let reason = e.to_string();
                let _ = catch_unwind(AssertUnwindSafe(|| drop(job)));
                return Err((reason, input));
            }
        };
        state.active_queued += 1;
        state.queue.push_reserved(QueuedTask { queued_at, owner });
        drop(state);
        self.shared.wake.notify_one();
        Ok(Pending { cell })
    }
}
struct ActiveSlot<'a>(&'a Shared);
impl Drop for ActiveSlot<'_> {
    fn drop(&mut self) {
        release_slot(self.0);
    }
}
fn reset_reader(
    reader: &mut AuditReader,
    path: &DispatchArc<DispatchPath>,
    resources: &QResources,
    allocation: &Option<super::q_allocation::Profile>,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| reader.close()));
    *reader = AuditReader::new_with_dispatch_path(path.clone(), resources, allocation.clone());
}
fn worker_loop(
    path: DispatchArc<DispatchPath>,
    resources: Arc<QResources>,
    leases: DispatchArc<ClientLeases>,
    shared: DispatchArc<Shared>,
    allocation: Option<super::q_allocation::Profile>,
) {
    let reader_path = path.clone();
    let mut reader =
        AuditReader::new_with_dispatch_path(path, resources.as_ref(), allocation.clone());
    loop {
        let mut task = match take_task(&shared) {
            Some(task) => task,
            None => break,
        };
        // Any unexpected unwind still releases the one accepted active+queued slot.
        let slot = ActiveSlot(&shared);
        let outcome = if task.queued_at.elapsed() > shared.queue_timeout {
            catch_unwind(AssertUnwindSafe(|| {
                task.owner
                    .reject("Incomplete：Q executor 请求排队超过 socket_read_timeout".into())
            }))
        } else {
            catch_unwind(AssertUnwindSafe(|| {
                task.owner
                    .run(&mut reader, resources.as_ref(), &leases, task.queued_at)
            }))
        };
        let publish = outcome.is_ok();
        if !matches!(outcome, Ok(false)) {
            reset_reader(&mut reader, &reader_path, resources.as_ref(), &allocation);
        }
        drop(slot); // reader cleanup and active-slot release both precede publication
        if publish && catch_unwind(AssertUnwindSafe(|| task.owner.complete())).is_err() {
            reset_reader(&mut reader, &reader_path, resources.as_ref(), &allocation);
        }
        // A dropped Pending may make final result payload destruction occur on this worker.
        // Catch user payload Drop panic after the local actual-allocation unwind guard runs.
        if catch_unwind(AssertUnwindSafe(|| drop(task))).is_err() {
            reset_reader(&mut reader, &reader_path, resources.as_ref(), &allocation);
        }
    }
    let _ = catch_unwind(AssertUnwindSafe(|| reader.close()));
}
fn take_task(shared: &Shared) -> Option<QueuedTask> {
    let mut state = shared.state.lock().unwrap_or_else(|p| p.into_inner());
    loop {
        if let Some(t) = state.queue.pop_front() {
            return Some(t);
        }
        if state.closed {
            return None;
        }
        state = shared.wake.wait(state).unwrap_or_else(|p| p.into_inner());
    }
}
fn release_slot(shared: &Shared) {
    let mut state = shared.state.lock().unwrap_or_else(|p| p.into_inner());
    state.active_queued = state.active_queued.saturating_sub(1);
    drop(state);
    shared.wake.notify_one();
}
fn close_shared(shared: &Shared) {
    let mut state = shared.state.lock().unwrap_or_else(|p| p.into_inner());
    state.closed = true;
    drop(state);
    shared.wake.notify_all();
}
impl Drop for Executor {
    fn drop(&mut self) {
        close_shared(&self.shared);
        while !self.workers.is_empty() {
            let _ = self.workers.remove(0).join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        sync::{
            atomic::{AtomicU64, Ordering},
            mpsc, Arc, Barrier,
        },
        thread,
    };

    fn resources(workers: u64, queue: u64, pending: u64, read_ms: u64) -> Arc<QResources> {
        let mut input = super::super::q_resources::tests::input();
        input["limits"]["max_query_workers"] = json!(workers.to_string());
        input["limits"]["max_query_queue"] = json!(queue.to_string());
        input["limits"]["max_pending_connections"] = json!(pending.to_string());
        input["limits"]["socket_read_timeout_ms"] = json!(read_ms.to_string());
        Arc::new(QResources::parse(&serde_json::to_vec(&input).unwrap()).unwrap())
    }

    #[test]
    fn queue_capacity_rejects_after_active_plus_queued_limit() {
        let resources = resources(1, 1, 2, 1000);
        let executor = Executor::new(PathBuf::from("test-db"), resources).unwrap();
        let gate = Arc::new(Barrier::new(2));
        let (started_tx, started_rx) = mpsc::channel();
        let first_gate = Arc::clone(&gate);
        let first = executor
            .try_submit(move |_, _, _, _| {
                started_tx.send(()).unwrap();
                first_gate.wait();
                Ok::<_, String>(1)
            })
            .unwrap();
        started_rx.recv().unwrap();
        let second = executor
            .try_submit(|_, _, _, _| Ok::<_, String>(2))
            .unwrap();
        assert!(executor
            .try_submit(|_, _, _, _| Ok::<_, String>(3))
            .is_err());
        gate.wait();
        assert_eq!(first.recv().unwrap(), 1);
        assert_eq!(second.recv().unwrap(), 2);
    }

    #[test]
    fn failed_or_panicking_task_releases_slot() {
        let resources = resources(1, 1, 2, 1000);
        let executor = Executor::new(PathBuf::from("test-db"), resources).unwrap();
        assert!(executor
            .try_submit(|_, _, _, _| Err::<u8, _>("task failed".into()))
            .unwrap()
            .recv()
            .is_err());
        let panic_result = executor
            .try_submit(|_, _, _, _| -> Result<u8, String> { panic!("test panic") })
            .unwrap()
            .recv();
        assert!(panic_result.unwrap_err().contains("task panic"));
        assert_eq!(
            executor
                .try_submit(|_, _, _, _| Ok::<_, String>(7))
                .unwrap()
                .recv()
                .unwrap(),
            7
        );
    }

    #[test]
    fn drop_waits_for_inflight_task_and_releases_worker_reader() {
        let resources = resources(1, 1, 2, 1000);
        let executor = Executor::new(PathBuf::from("test-db"), resources).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let (queued_tx, queued_rx) = mpsc::channel();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let gate = Arc::new(Barrier::new(2));
        let task_gate = Arc::clone(&gate);
        let pending = executor
            .try_submit(move |_, _, _, _| {
                started_tx.send(thread::current().id()).unwrap();
                task_gate.wait();
                finished_tx.send(()).unwrap();
                Ok::<_, String>(())
            })
            .unwrap();
        let second = executor
            .try_submit(move |_, _, _, _| {
                queued_tx.send(thread::current().id()).unwrap();
                Ok::<_, String>(())
            })
            .unwrap();
        let worker_id = started_rx.recv().unwrap();
        let dropper = thread::spawn(move || {
            drop(executor);
            dropped_tx.send(()).unwrap();
        });
        let release = thread::spawn(move || {
            gate.wait();
        });
        release.join().unwrap();
        finished_rx.recv().unwrap();
        assert!(pending.recv().is_ok());
        assert_eq!(queued_rx.recv().unwrap(), worker_id);
        assert!(second.recv().is_ok());
        dropped_rx.recv().unwrap();
        dropper.join().unwrap();
        assert_ne!(worker_id, thread::current().id());
    }

    #[test]
    fn queued_task_can_be_rejected_by_injected_old_queued_at_without_running() {
        let resources = resources(1, 1, 2, 1000);
        let executor = Executor::new(PathBuf::from("test-db"), resources).unwrap();
        let gate = Arc::new(Barrier::new(2));
        let (started_tx, started_rx) = mpsc::channel();
        let ran = Arc::new(AtomicU64::new(0));
        let hold = Arc::clone(&gate);
        let first = executor
            .try_submit(move |_, _, _, _| {
                started_tx.send(()).unwrap();
                hold.wait();
                Ok::<_, String>(())
            })
            .unwrap();
        started_rx.recv().unwrap();
        let old = Instant::now() - Duration::from_secs(2);
        let ran_task = Arc::clone(&ran);
        let second = executor
            .try_submit_at(old, move |_, _, _, _| {
                ran_task.fetch_add(1, Ordering::SeqCst);
                Ok::<_, String>(9)
            })
            .unwrap();
        gate.wait();
        assert!(first.recv().is_ok());
        let result = second.recv();
        assert!(result.is_err());
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn owned_input_returns_on_capacity_and_closes_before_expiry_result() {
        struct Owned(Arc<AtomicU64>);
        impl Drop for Owned {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let executor =
            Executor::new(PathBuf::from("unopened-test-db"), resources(1, 1, 2, 1000)).unwrap();
        let (started_send, started) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let first = executor
            .try_submit(move |_, _, _, _| {
                started_send.send(()).unwrap();
                held.recv().unwrap();
                Ok(())
            })
            .unwrap();
        started.recv().unwrap();
        let expired_drops = Arc::new(AtomicU64::new(0));
        let queued = executor
            .try_submit_owned_at(
                Instant::now() - Duration::from_secs(2),
                Owned(expired_drops.clone()),
                |_, _, _, _, _| -> Result<(), String> {
                    panic!("过期连接不得被读取");
                },
            )
            .ok()
            .unwrap();
        let returned_drops = Arc::new(AtomicU64::new(0));
        let rejected =
            executor.try_submit_owned(Owned(returned_drops.clone()), |_, _, _, _, _| Ok(()));
        let (_, input) = rejected.err().expect("容量已满");
        assert_eq!(returned_drops.load(Ordering::SeqCst), 0);
        drop(input);
        assert_eq!(returned_drops.load(Ordering::SeqCst), 1);
        release.send(()).unwrap();
        first.recv().unwrap();
        assert!(queued.recv().unwrap_err().contains("排队"));
        assert_eq!(expired_drops.load(Ordering::SeqCst), 1);
        executor
            .try_submit(|_, _, _, _| Ok(()))
            .unwrap()
            .recv()
            .unwrap();
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    fn allocation(budget: &OwnedBudget) -> super::super::q_allocation::Profile {
        super::super::q_allocation::Profile {
            budget: budget.clone(),
            overlay_slots: 0,
            overlay_bytes: 0,
        }
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_none_owned_same_queue_expiry_panic_close_and_drop_drain() {
        let mut observations = Vec::new();
        for owned in [false, true] {
            let budget = OwnedBudget::new(512 * 1024);
            let executor = Executor::new_with_allocation(
                PathBuf::from("unopened-owner-db"),
                resources(1, 1, 2, 1000),
                owned.then(|| allocation(&budget)),
            )
            .unwrap();
            let (start_tx, start_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let first = executor
                .try_submit(move |_, _, _, _| {
                    start_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(1u8)
                })
                .unwrap();
            start_rx.recv().unwrap();
            let second = executor
                .try_submit_at(
                    Instant::now() - Duration::from_secs(2),
                    |_, _, _, _| -> Result<u8, String> { panic!("expired task must not run") },
                )
                .unwrap();
            let (error, original) = executor
                .try_submit_owned(83u64, |_, _, _, _, _| Ok(3u8))
                .err()
                .unwrap();
            assert_eq!(original, 83);
            assert!(error.contains("冻结上限"));
            release_tx.send(()).unwrap();
            assert_eq!(first.recv().unwrap(), 1);
            assert!(second.recv().unwrap_err().contains("排队"));
            assert_eq!(
                executor.shared.state.lock().unwrap().active_queued,
                0,
                "slot released before completion"
            );
            let panic_error = executor
                .try_submit(|_, _, _, _| -> Result<u8, String> { panic!("task reset") })
                .unwrap()
                .recv()
                .unwrap_err();
            assert!(panic_error.contains("task panic"));
            let next = executor
                .try_submit(|reader, _, _, _| Ok(reader.stats().captures))
                .unwrap()
                .recv()
                .unwrap();
            assert_eq!(next, 0);
            close_shared(&executor.shared);
            assert!(executor
                .try_submit_owned(91u64, |_, _, _, _, _| Ok(()))
                .err()
                .unwrap()
                .0
                .contains("已关闭"));
            observations.push((original, next, panic_error));
            drop(executor);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
        assert_eq!(observations[0], observations[1]);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_partial_spawn_failure_joins_and_releases_every_owner() {
        for owned in [false, true] {
            let budget = OwnedBudget::new(512 * 1024);
            let error = Executor::build(
                PathBuf::from("spawn-failure-db"),
                resources(2, 1, 3, 1000),
                owned.then(|| allocation(&budget)),
                Some(1),
            )
            .err()
            .unwrap();
            assert!(error.contains("worker 创建失败"));
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_quota_refusal_retains_input_for_queue_cell_and_job_boundaries() {
        // 先完成一次，固定ring实际capacity；随后用真实allocator填满剩余额度。
        for room_for_cell in [false, true] {
            let budget = OwnedBudget::new(512 * 1024);
            let executor = Executor::new_with_allocation(
                PathBuf::from("quota-owned-db"),
                resources(1, 1, 2, 1000),
                Some(allocation(&budget)),
            )
            .unwrap();
            executor
                .try_submit(|_, _, _, _| Ok(1u8))
                .unwrap()
                .recv()
                .unwrap();
            // 同worker下一任务已经开始 => 上一Job已完成析构；当前Job阻塞使live稳定。
            let (ack_tx, ack_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let held = executor
                .try_submit(move |_, _, _, _| {
                    ack_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
            ack_rx.recv().unwrap();
            // 测量真实cell block Layout；当前worker不再触账，快照差值没有并发污染。
            let before = budget.snapshot().live_requested_layout_bytes;
            let probe = DispatchArc::try_new(ResultCell::<u8>::new(), Some(&budget))
                .ok()
                .unwrap();
            let cell_bytes = budget.snapshot().live_requested_layout_bytes - before;
            drop(probe);
            let allowance = if room_for_cell { cell_bytes } else { 0 };
            let live = budget.snapshot().live_requested_layout_bytes;
            let mut filler = CodecVec::<u8>::new_with_allocation(Some(&budget));
            filler
                .try_reserve_exact(budget.snapshot().limit_bytes - live - allowance)
                .unwrap();
            let outcome = executor.try_submit_owned(131u64, |_, _, _, _, _| Ok(7u8));
            match outcome {
                Err((error, input)) => {
                    assert_eq!(input, 131);
                    assert!(error.contains("prototype_capture_owned_payload_limit"));
                }
                Ok(pending) => {
                    drop(filler);
                    release_tx.send(()).unwrap();
                    pending.recv().unwrap();
                    panic!("saturated actual budget must refuse");
                }
            }
            drop(filler);
            release_tx.send(()).unwrap();
            held.recv().unwrap();
            drop(executor);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
        // 新ring第一次增长本身必须先过同ledger：零余量不得接纳或移动input。
        let budget = OwnedBudget::new(512 * 1024);
        let executor = Executor::new_with_allocation(
            PathBuf::from("queue-growth-db"),
            resources(1, 1, 2, 1000),
            Some(allocation(&budget)),
        )
        .unwrap();
        let mut filler = CodecVec::<u8>::new_with_allocation(Some(&budget));
        filler
            .try_reserve_exact(
                budget.snapshot().limit_bytes - budget.snapshot().live_requested_layout_bytes,
            )
            .unwrap();
        let (error, original) = executor
            .try_submit_owned(193u64, |_, _, _, _, _| Ok(()))
            .err()
            .unwrap();
        assert_eq!(original, 193);
        assert!(error.contains("prototype_capture_owned_payload_limit"));
        assert_eq!(executor.shared.state.lock().unwrap().active_queued, 0);
        drop(filler);
        drop(executor);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        // 零额度构造在任何线程启动前拒绝，不安装allocator或丢入std路径。
        let budget = OwnedBudget::new(0);
        assert!(Executor::new_with_allocation(
            PathBuf::from("zero-budget-db"),
            resources(1, 1, 2, 1000),
            Some(allocation(&budget))
        )
        .is_err());
        assert_eq!(budget.snapshot().allocated_layouts, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_result_payload_and_cell_live_until_actual_pending_consumption() {
        let budget = OwnedBudget::new(512 * 1024);
        let executor = Executor::new_with_allocation(
            PathBuf::from("result-owner-db"),
            resources(1, 1, 2, 1000),
            Some(allocation(&budget)),
        )
        .unwrap();
        let task_budget = budget.clone();
        let pending = executor
            .try_submit(move |_, _, _, _| {
                let mut payload = CodecVec::<u8>::new_with_allocation(Some(&task_budget));
                payload
                    .try_extend_bytes(b"completion-owned-body")
                    .map_err(|e| e.to_string())?;
                Ok(payload)
            })
            .unwrap();
        drop(executor); // join establishes publication AND Job teardown before checking retained owner
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        let mut pending = pending;
        let body = pending.try_result().unwrap().unwrap();
        assert_eq!(&*body, b"completion-owned-body");
        assert!(body.allocation_budget().unwrap().same_ledger(&budget));
        let body_bytes = body.capacity();
        drop(pending);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, body_bytes);
        drop(body);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_panic_resets_warmed_reader_before_completion_none_and_owned() {
        use newchan_rust::s_content::{access::Access, job::Job};
        struct Server(Option<OwnedBudget>);
        impl Access for Server {
            fn owner_job(&mut self) -> Option<&mut Job> {
                None
            }
            fn allocation_budget(&self) -> Option<&OwnedBudget> {
                self.0.as_ref()
            }
        }
        let mut f = super::super::prepared::tests::Fixture::new();
        let (envelope, token) = f.accept_begin(1, ("dispatch-owner-raw", 10, 7, 13, 4, 8), 1);
        f.publish(&envelope, &token).unwrap();
        for owned in [false, true] {
            let budget = OwnedBudget::new(50 * 1024 * 1024);
            let mut profile = allocation(&budget);
            profile.overlay_slots = 32768;
            profile.overlay_bytes = 1_000_000;
            let executor = Executor::new_with_allocation(
                f.db_path(),
                resources(1, 1, 2, 20000),
                owned.then_some(profile),
            )
            .unwrap();
            let warm = executor
                .try_submit(|reader, resources, _, _| {
                    let mut server = Server(reader.allocation_profile().map(|p| p.budget.clone()));
                    reader.with_capture(resources, &mut server, |_, _| Ok(()))?;
                    Ok(reader.stats())
                })
                .unwrap()
                .recv()
                .unwrap();
            assert_eq!(warm.captures, 1);
            assert!(warm.cached_bytes > 0);
            let failure = executor
                .try_submit(|reader, _, _, _| -> Result<(), String> {
                    assert_eq!(reader.stats().captures, 1);
                    assert!(reader.stats().cached_bytes > 0);
                    panic!("warm reader reset fixture");
                })
                .unwrap()
                .recv()
                .unwrap_err();
            assert!(failure.contains("task panic"));
            let reset = executor
                .try_submit(|reader, resources, _, _| {
                    assert_eq!(reader.stats().captures, 0);
                    assert_eq!(reader.stats().cached_bytes, 0);
                    let mut server = Server(reader.allocation_profile().map(|p| p.budget.clone()));
                    reader.with_capture(resources, &mut server, |_, _| Ok(()))?;
                    Ok(reader.stats().captures)
                })
                .unwrap()
                .recv()
                .unwrap();
            assert_eq!(reset, 1);
            drop(executor);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_owned_drop_drains_accepted_jobs_in_fifo_order() {
        let budget = OwnedBudget::new(512 * 1024);
        let executor = Executor::new_with_allocation(
            PathBuf::from("drop-owner-db"),
            resources(1, 2, 3, 20000),
            Some(allocation(&budget)),
        )
        .unwrap();
        let (start_tx, start_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (order_tx, order_rx) = mpsc::channel();
        let order_first = order_tx.clone();
        let first = executor
            .try_submit(move |_, _, _, _| {
                start_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                order_first.send(1).unwrap();
                Ok(1)
            })
            .unwrap();
        start_rx.recv().unwrap();
        let order_second = order_tx.clone();
        let second = executor
            .try_submit(move |_, _, _, _| {
                order_second.send(2).unwrap();
                Ok(2)
            })
            .unwrap();
        let third = executor
            .try_submit(move |_, _, _, _| {
                order_tx.send(3).unwrap();
                Ok(3)
            })
            .unwrap();
        let dropper = thread::spawn(move || drop(executor));
        release_tx.send(()).unwrap();
        dropper.join().unwrap();
        assert_eq!(order_rx.into_iter().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(first.recv().unwrap(), 1);
        assert_eq!(second.recv().unwrap(), 2);
        assert_eq!(third.recv().unwrap(), 3);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_expired_input_and_capture_drop_panics_release_slot_and_keep_worker_alive() {
        struct Panics(Arc<AtomicU64>);
        impl Drop for Panics {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                panic!("expired owner Drop fixture");
            }
        }
        for owned in [false, true] {
            let budget = OwnedBudget::new(512 * 1024);
            let executor = Executor::new_with_allocation(
                PathBuf::from("expired-drop-panic-db"),
                resources(1, 1, 2, 1000),
                owned.then(|| allocation(&budget)),
            )
            .unwrap();
            let drops = Arc::new(AtomicU64::new(0));
            let captured = Panics(drops.clone());
            let expired = executor
                .try_submit_owned_at(
                    Instant::now() - Duration::from_secs(2),
                    Panics(drops.clone()),
                    move |_, _, _, _, _| -> Result<(), String> {
                        drop(captured);
                        panic!("expired never executes");
                    },
                )
                .ok()
                .unwrap();
            assert!(expired.recv().unwrap_err().contains("task panic"));
            assert_eq!(
                drops.load(Ordering::SeqCst),
                2,
                "both input and task captures cleaned despite separate Drop panics"
            );
            assert_eq!(executor.shared.state.lock().unwrap().active_queued, 0);
            assert_eq!(
                executor
                    .try_submit(|_, _, _, _| Ok(137u64))
                    .unwrap()
                    .recv()
                    .unwrap(),
                137
            );
            drop(executor);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_discarded_result_drop_panic_frees_cell_and_keeps_worker_alive() {
        struct Panics(Arc<AtomicU64>);
        impl Drop for Panics {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
                panic!("discarded result Drop fixture");
            }
        }
        let budget = OwnedBudget::new(512 * 1024);
        let executor = Executor::new_with_allocation(
            PathBuf::from("result-drop-panic-db"),
            resources(1, 1, 2, 1000),
            Some(allocation(&budget)),
        )
        .unwrap();
        let drops = Arc::new(AtomicU64::new(0));
        let drops_task = drops.clone();
        let (start_tx, start_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let pending = executor
            .try_submit(move |_, _, _, _| {
                start_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(Panics(drops_task))
            })
            .unwrap();
        start_rx.recv().unwrap();
        drop(pending);
        release_tx.send(()).unwrap();
        executor
            .try_submit(|_, _, _, _| Ok(()))
            .unwrap()
            .recv()
            .unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        drop(executor);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(!budget.ledger_health().invariant_broken);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn dispatch_job_layout_refusal_drops_capture_after_unlock_even_on_panic() {
        struct Capture {
            shared: DispatchArc<Shared>,
            lock_observed: Arc<AtomicU64>,
            panic_drop: bool,
        }
        impl Drop for Capture {
            fn drop(&mut self) {
                // 只try_lock：若回归为锁内析构，立即得到失败而不悬挂验证。
                self.lock_observed.store(
                    if self.shared.state.try_lock().is_ok() {
                        1
                    } else {
                        2
                    },
                    Ordering::SeqCst,
                );
                if self.panic_drop {
                    panic!("unaccepted capture Drop fixture");
                }
            }
        }
        fn bounded_result<R>(mut pending: Pending<R>) -> Result<R, String> {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(result) = pending.try_result() {
                    return result;
                }
                assert!(Instant::now() < deadline, "worker completion timed out");
                thread::yield_now();
            }
        }
        for panic_drop in [false, true] {
            let budget = OwnedBudget::new(512 * 1024);
            let executor = Executor::new_with_allocation(
                PathBuf::from("unaccepted-capture-db"),
                resources(1, 1, 2, 20000),
                Some(allocation(&budget)),
            )
            .unwrap();
            let (start_tx, start_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let held = executor
                .try_submit(move |_, _, _, _| {
                    start_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    Ok(())
                })
                .unwrap();
            start_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            // worker仍在held任务内：真实live固定、队列ring已分配且有空槽。
            let before = budget.snapshot().live_requested_layout_bytes;
            let cell = DispatchArc::try_new(ResultCell::<u8>::new(), Some(&budget))
                .ok()
                .unwrap();
            let cell_bytes = budget.snapshot().live_requested_layout_bytes - before;
            drop(cell);
            let live = budget.snapshot().live_requested_layout_bytes;
            let mut filler = CodecVec::<u8>::new_with_allocation(Some(&budget));
            filler
                .try_reserve_exact(budget.snapshot().limit_bytes - live - cell_bytes)
                .unwrap();
            let lock_observed = Arc::new(AtomicU64::new(0));
            let capture = Capture {
                shared: executor.shared.clone(),
                lock_observed: lock_observed.clone(),
                panic_drop,
            };
            let rejected = executor.try_submit_owned(211u64, move |_, _, _, _, _| {
                drop(capture);
                Ok(7u8)
            });
            // 先释放worker，再作断言，测试失败也不会留下无界等待的worker。
            drop(filler);
            release_tx.send(()).unwrap();
            let (reason, input) = rejected.err().expect("actual Job Layout must refuse");
            assert_eq!(input, 211);
            assert!(reason.contains("prototype_capture_owned_payload_limit"));
            assert!(budget
                .allocation_request_probe()
                .refused_kind
                .contains("q_executor::Job"));
            assert_eq!(
                lock_observed.load(Ordering::SeqCst),
                1,
                "capture Drop observes unlocked queue"
            );
            assert!(!executor.shared.state.is_poisoned());
            bounded_result(held).unwrap();
            assert_eq!(
                bounded_result(executor.try_submit(|_, _, _, _| Ok(223u64)).unwrap()).unwrap(),
                223
            );
            drop(executor);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }
}
