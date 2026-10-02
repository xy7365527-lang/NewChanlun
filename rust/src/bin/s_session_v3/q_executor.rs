//! #1394 C §5.6.1：固定 worker 与有界 Q 读取队列。
//!
//! 这是内部执行器，不注册 HTTP、不构造部署默认值，也不把 `Job` 跨线程传递。
//! 每个 worker 持有自己的长期 `AuditReader`；提交的闭包只在该 worker 上借用
//! reader、冻结资源和共享租约表，完成后再释放 active+queued 槽位。Drop 只关闭新提交，
//! 已接受的排队任务仍按其 queued_at 执行或过期，然后等待 worker 退出。

use super::{
    capture::AuditReader,
    q_lease::ClientLeases,
    q_resources::{Limit, QResources},
};
use std::{
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver},
        Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// worker 内部的执行结果；`ResetReader` 表示任务闭包 panic，必须丢弃该 worker 的 reader。
/// completion 延后到 reader 清理和 active+queued 槽位释放之后执行。
type Completion = Box<dyn FnOnce() + Send>;
enum RunOutcome {
    Complete(Completion),
    ResetReader(Completion),
}

type RunTask =
    Box<dyn FnOnce(&mut AuditReader, &QResources, &ClientLeases, Instant) -> RunOutcome + Send>;
type RejectTask = Box<dyn FnOnce(String) + Send>;

struct QueuedTask {
    queued_at: Instant,
    run: RunTask,
    reject: RejectTask,
}

struct QueueState {
    closed: bool,
    active_queued: u64,
    queue: VecDeque<QueuedTask>,
}

struct Shared {
    state: Mutex<QueueState>,
    wake: Condvar,
    capacity: u64,
    queue_timeout: Duration,
}

/// 尚未接收的单次任务结果。调用 `recv` 只等待本任务 channel，不持队列锁。
pub(super) struct Pending<R> {
    receiver: Receiver<Result<R, String>>,
}

impl<R> Pending<R> {
    /// 监听循环只观察终态，不在一个慢连接上阻塞接纳或收尾其他连接。
    pub(super) fn try_result(&mut self) -> Option<Result<R, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Incomplete：Q executor worker 在返回结果前退出".into()))
            }
        }
    }
    pub(super) fn recv(self) -> Result<R, String> {
        self.receiver
            .recv()
            .map_err(|_| "Incomplete：Q executor worker 在返回结果前退出".to_string())?
    }
}

/// 固定数量 worker、有限 active+queued 槽位的内部 Q 执行器。
pub(super) struct Executor {
    shared: Arc<Shared>,
    resources: Arc<QResources>,
    leases: Arc<ClientLeases>,
    workers: Vec<JoinHandle<()>>,
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
        if let Some(p) = &allocation {
            p.validate()?;
        }
        let worker_count = resources.limit(Limit::QueryWorkers);
        let worker_count_usize = usize::try_from(worker_count)
            .map_err(|_| "Incomplete：max_query_workers 超出本机线程索引范围".to_string())?;
        let capacity = resources.limit(Limit::PendingConnections).min(
            worker_count
                .checked_add(resources.limit(Limit::QueryQueue))
                .ok_or_else(|| "Incomplete：Q executor active+queued 容量溢出".to_string())?,
        );
        if capacity == 0 || worker_count_usize == 0 {
            return Err("Incomplete：Q executor worker/active+queued 容量必须为正".into());
        }

        let shared = Arc::new(Shared {
            state: Mutex::new(QueueState {
                closed: false,
                active_queued: 0,
                queue: VecDeque::new(),
            }),
            wake: Condvar::new(),
            capacity,
            queue_timeout: Duration::from_millis(resources.limit(Limit::SocketReadMs)),
        });
        let leases = Arc::new(ClientLeases::with_allocation(
            resources.as_ref(),
            allocation.as_ref().map(|p| &p.budget),
        )?);
        // 不按配置值预分配句柄或队列；只有成功创建的 worker 才占用实际 Vec 容量。
        let mut workers = Vec::new();
        for index in 0..worker_count_usize {
            let worker_shared = Arc::clone(&shared);
            let worker_resources = Arc::clone(&resources);
            let worker_leases = Arc::clone(&leases);
            let worker_path = path.clone();
            let worker_allocation = allocation.clone();
            let name = format!("s-query-{index}");
            let spawned = thread::Builder::new().name(name).spawn(move || {
                worker_loop(
                    worker_path,
                    worker_resources,
                    worker_leases,
                    worker_shared,
                    worker_allocation,
                )
            });
            match spawned {
                Ok(handle) => workers.push(handle),
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
        })
    }

    pub(super) fn leases(&self) -> &ClientLeases {
        self.leases.as_ref()
    }

    pub(super) fn resources(&self) -> &QResources {
        self.resources.as_ref()
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
        .map_err(|(error, ())| error)
    }

    /// 在读取连接前占槽；拒绝接纳时把原连接交还调用者发送有界503。
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
            Ok(state) => state,
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
        let (sender, receiver) = mpsc::channel::<Result<R, String>>();
        let complete_sender = sender.clone();
        let panic_sender = sender.clone();
        let run: RunTask =
            Box::new(move |reader, resources, leases, queued_at| {
                match catch_unwind(AssertUnwindSafe(|| {
                    task(input, reader, resources, leases, queued_at)
                })) {
                    Ok(result) => RunOutcome::Complete(Box::new(move || {
                        let _ = complete_sender.send(result);
                    })),
                    Err(_) => RunOutcome::ResetReader(Box::new(move || {
                        let _ = panic_sender.send(Err(
                            "Incomplete：Q executor task panic；已清理本 worker reader".to_string(),
                        ));
                    })),
                }
            });
        let reject_sender = sender;
        let reject: RejectTask = Box::new(move |error| {
            let _ = reject_sender.send(Err(error));
        });
        let queued = QueuedTask {
            queued_at,
            run,
            reject,
        };
        state.active_queued += 1;
        state.queue.push_back(queued);
        drop(state);
        self.shared.wake.notify_one();
        Ok(Pending { receiver })
    }
}

fn worker_loop(
    path: PathBuf,
    resources: Arc<QResources>,
    leases: Arc<ClientLeases>,
    shared: Arc<Shared>,
    allocation: Option<super::q_allocation::Profile>,
) {
    let reader_path = path.clone();
    let mut reader = AuditReader::new_with_allocation(path, resources.as_ref(), allocation.clone());
    loop {
        let task = match take_task(&shared) {
            Some(task) => task,
            None => break,
        };
        if task.queued_at.elapsed() > shared.queue_timeout {
            let QueuedTask { reject, run, .. } = task;
            // 被排队闭包持有的socket等输入必须先关闭，再给调用方终态。
            drop(run);
            release_slot(&shared);
            reject("Incomplete：Q executor 请求排队超过 socket_read_timeout".into());
            continue;
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let QueuedTask {
                queued_at,
                run,
                reject: _,
            } = task;
            run(&mut reader, resources.as_ref(), leases.as_ref(), queued_at)
        }));
        match outcome {
            Ok(RunOutcome::Complete(completion)) => {
                release_slot(&shared);
                completion();
            }
            Ok(RunOutcome::ResetReader(completion)) => {
                let _ = catch_unwind(AssertUnwindSafe(|| reader.close()));
                reader = AuditReader::new_with_allocation(
                    reader_path.clone(),
                    resources.as_ref(),
                    allocation.clone(),
                );
                release_slot(&shared);
                completion();
            }
            Err(_) => {
                let _ = catch_unwind(AssertUnwindSafe(|| reader.close()));
                // The task closure was consumed before the outer panic, so its receiver
                // observes a disconnected channel. The slot is still released below.
                reader = AuditReader::new_with_allocation(
                    reader_path.clone(),
                    resources.as_ref(),
                    allocation.clone(),
                );
                release_slot(&shared);
            }
        }
    }
    let _ = catch_unwind(AssertUnwindSafe(|| reader.close()));
}

fn take_task(shared: &Shared) -> Option<QueuedTask> {
    let mut state = match shared.state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    loop {
        if let Some(task) = state.queue.pop_front() {
            return Some(task);
        }
        if state.closed {
            return None;
        }
        state = match shared.wake.wait(state) {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
    }
}

fn release_slot(shared: &Shared) {
    let mut state = match shared.state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.active_queued = state.active_queued.saturating_sub(1);
    drop(state);
    shared.wake.notify_one();
}

fn close_shared(shared: &Shared) {
    let mut state = match shared.state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.closed = true;
    // 已入队任务保留：与旧 executor.shutdown(wait=True) 一样，worker 继续执行它们，
    // 或在真正取出时按原 queued_at 判定排队超时。
    drop(state);
    shared.wake.notify_all();
}

impl Drop for Executor {
    fn drop(&mut self) {
        close_shared(&self.shared);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
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
}
