# #1505 D2 普通热 journal 私有视图原型

名分：prototype；基于 D1 b9f38106a5c0cc6f320798cf146e3bc7a6677c1b。仅非默认 feature `prototype_1505` / bin `p1505_fd_journal`，不接任何生产入口。状态为源码候选，需独立审查后在精确新 SHA 做受限远端验证。本机禁止编译、SQLite、真实信号和产品/性能/浏览器运行，禁止访问外盘。

## 核心边界

源 main/journal 固定为单链接 regular O_RDONLY/NOFOLLOW FD。SQLite 获得虚拟 RW：其自己的 journal/pager 实现把回滚写入固定 chunk arena，绝不直接写源。journal footer 前置门只拒绝有效非空 super-journal 引用，不解释恢复页。源 WAL/SHM 或未验证输入组合具名拒绝，D1 的限定 WAL 结果不计为 D2 通过。

覆盖层维护 logical_len、单调不增 visible_base_end、4096-byte chunks；truncate再增长不能复活源尾。多块write先备齐全部新块，预读失败或容量不足撤回pending块，再返回sticky错误。source值、arena容量、SQLite heap观测和进程RSS分开计；默认allocator内部realloc瞬时峰值和私有overhead明确NOT_MEASURED。

Registration具有 !Send/!Sync，Session借用owner。Context捕获pthread owner，FFI先raw检查线程，再取得非重入guard，再建立file/RefCell引用。可变file字段用raw字段操作，避免对整个file的可变借用与重入前ctx检查冲突；RefCell借用失败返回错误，不能变成journal不存在。SQLite worker threads强制为0并核回。

owned分配失败、底层EOF/identity错误用无额外分配的ErrorKind；外围Box/JSON/格式化与受控panic仍可能分配。受控arena失败和SQLite malloc失败不是全进程真实OOM实证，也不能证明失败时总能写完报告。

## 显式输入

```json
{
  "max_source_main_bytes": 2097152,
  "max_source_journal_bytes": 4194304,
  "overlay_payload_capacity_bytes": 8388608,
  "overlay_metadata_capacity_bytes": 1048576,
  "evidence_capacity_bytes": 4194304,
  "sqlite_heap_limit_bytes": 8388608
}
```

这是已批准的 D2-SMALL-01 测试参数，无默认；不是生产RAM合同。payload/metadata/trace的owned容量合计13MiB，SQLite配置8MiB单列，不作为全进程21MiB物理硬帽。driver的SQL结果、JSON、byte摘要和归档缓冲不属于VFS事件arena，RSS也不是同一口径。SQLite初始化后才安装hard_heap_limit，但记录初始化前段的xSize current/peak并在超额时失败；这属于分配后检测，不承诺内部瞬时物理峰值从未超额。

## 远端接口与期限

外层build采用D1相同debug profile，fresh CARGO_TARGET_DIR与精确候选SHA：

```text
cargo build --locked --no-default-features --features prototype_1505 --bin p1505_fd_journal
<fresh-bin> suite /absolute/new/evidence /absolute/limits.json
```

在build前取同一个CLOCK_MONOTONIC起点t0，传`P1505_DEADLINE_NS=t0+1170s`和`P1505_SUITE_DEADLINE_NS=t0+1020s`，均为纳秒。suite和其子进程不得重置期限，子进程继承共同值与suite值；余段保留给原件、归档、cleanup、inventory。同步内核I/O的硬截止仍依赖外层监督，并未由Rust轮询解决。

writer/oracle/reader是内部子命令；父suite不初始化SQLite。writer真SQLite事务spill，父核hash/长度和存活后向确切子PID SIGKILL，并核signal/reap；默认VFS oracle为不同inode逐字副本上的独立进程。reader独立进程初始化前装allocator观察器，只读源做私有恢复。原长大于当前main的案例明示为真实hot副本上的crash-image-mutation，禁止称自然crash形状。

套件成功仅写`PASS_D2_COMPONENT_ONLY`；所有原件和原始进程输出保留，套件不执行清理。完整机器接口见本票实施证据包的CI_CONTRACT.json/.md。真实宿主EINTR、监督器SIGTERM/SIGINT取消尚未覆盖；活statement关闭拒绝已在sql-denials中实现但NOT_RUN；不可用普通读或SIGKILL代替。

## late-exclusive 两项负控

首轮远端 run `38767596239` / SHA `2767648a1c738ea5bb320a5be98a2f4461b24b7b` 已编译成功，但套件在旧 late-exclusive 的错误断言处失败。原件显示 hot 源未 arm 时，SQLite 先以 journal flags 2049、2050 尝试打开，VFS 均以 CANTOPEN=14 拒绝，只有 main 的100字节头读取成功；没有发生 overlay 写。这证明安全门提前生效，不能要求同一 hot 案例继续走到 xRead IOERR_READ。

保留原 `late-exclusive` 的 hot 源，精确核两次 journal open-denied、CANTOPEN及仅头读取；新增 `late-exclusive-clean` 使用无 journal 的 clean 源，同样 `connect(false)`，要求非头 main 读取确实以 IOERR_READ=266 失败，且没有 open-denied。两例都必须无 arm、无 overlay 写/截短/删除，源双快照一致，文件关闭后才注销。分别保留 `fault-late-exclusive-case.json` 与 `fault-late-exclusive-clean-case.json`，外层计数由12/10/10/2变为12/11/10/2，不删除原案例，不接受“任一错误即可”。

首轮的11个正控、L01及前5个负控属于上述旧 SHA 的历史证据；随后4个负控和 reader SIGKILL/post-reader-kill尚未运行。本次修改后的源码仍须新 SHA 的完整独立运行，不能把旧、新结果拼成通过。

## 持续未完

生产v4仍BLOCK_NOT_MERGEABLE。正式producer/R4/CLI/admission、完整S store baseline、源RW最后检查至open窗口、历史49GB规模、64GiB/store、148GiB双跑前置及完整资金/隔离/恢复/双跑和六项全图仍待验收；#1279不关闭，前端归#1460，K4排除。
