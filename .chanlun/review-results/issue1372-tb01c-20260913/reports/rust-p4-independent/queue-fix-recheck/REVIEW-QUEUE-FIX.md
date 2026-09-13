# #1372 Rust 响应入队期限门独立复验

结论：新冻结候选 `b924a71fb11d36977819499d078ddc603ce3a466` 修复了旧 P4 已确认的排队跨期准入缺陷；本次窄范围没有新增已确认缺陷。独立原函数摘录检查 **5 通过、0 失败**。此结论不扩大为完整 TB-01-C、真实 GUI 或持续输入负载验收。

## 冻结绑定

- 新冻结二进制：`/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/s_structure_session`，3712672 字节，SHA-256 `5bb7f1dfd57e1d8e6cfc1097360246ae27333776504767df4aba1b9fba9ce71b`。本工位实读完整字节校验；本轮未运行该二进制。
- 新冻结源码清单：`/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/SOURCE.json`，SHA-256 `1d21724e108ee5c0f9ed99c5a23ff1ac95cf9012e9a3e68492082a71e111dc65`。全部 **425** 份快照逐项校验字节数及 SHA-256，均相符。
- 新 `mod.rs`：`/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/source/rust/src/bin/s_session_v2/mod.rs`，SHA-256 `64f8d0a8307e5c3f4173be94491b1e3cd4104276dce246d0cc544700e0746a6a`。
- 与旧 P4 清单对比：无新增、无删除，唯一不同文件是 `rust/src/bin/s_session_v2/mod.rs`；差异原文保存在 `P4-to-queue-fix.diff`，内容仅增加入队函数、以它替换原循环及增加回归测试。
- 根现有构建收据：`/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/BUILD-RECEIPT.json`，SHA-256 `7ea7eb10fc00546f1abc7ac9a7da50cc8ea20bcfb8968ea507943a069a43653d`；载明 S 测试 65 通过、0 失败、1 忽略（27.11 秒），release build 成功。此处列为根已执行收据，未冒称本工位独立重跑。

## 修复与调用边界

[冻结源码第 1400 行](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/source/rust/src/bin/s_session_v2/mod.rs:1400) 的 `enqueue_before_deadline` 在**每轮尝试入队前**核同一个绝对 `deadline`；到期即返回拒绝。Full 分支保留原请求，睡眠取 1 毫秒与剩余期限的较小值，再回循环首核期限。因此旧循环在 Full 后跨期、队列槽位释放时，下一轮先入队的问题已被移除。

[第 1437 行](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/source/rust/src/bin/s_session_v2/mod.rs:1437) 仍在完整帧解码成功后只创建一次响应期限；[第 1443 行](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/source/rust/src/bin/s_session_v2/mod.rs:1443) 将它传入新门，失败直接结束连接；[第 1447 行](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-queue-deadline-fix/frozen/source/rust/src/bin/s_session_v2/mod.rs:1447) 等待 actor 回包继续使用同一期限的剩余时间。已进入 actor 的权威工作不会因回包等待超期而被撤销或自动重试。

源码差异同时确认 `read_frame` 未修改：帧大小、单 LF 后 EOF、读取绝对期限与解码后期限复核仍保留。写出期限也未改。memo、fresh raw、独立索引、控制 fence 和 Arc 相关另外 424 份源码同字节；旧 P4 对这些范围的独立审阅和 11 项小域证据仍以旧报告身份保留，本次没有重跑或升级其名分。

## 本工位实际执行

直接从新冻结 `mod.rs:1400–1418` 摘取整个函数，函数体逐字不改，SHA-256 `1955bb4160991d1561c40df15be3b4a3c42fd752c919d978c378305a1b38af3e`。独立 harness 仅将请求类型 `Job` 设为 `u8`，使用真实 Rust 标准库有界通道与单调时钟，隔离队列准入行为。编译与执行完整命令、stdout、stderr、退出码、源码和附件 SHA 在 [RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-p4-independent/queue-fix-recheck/RESULT.json)。

| 检查 | 实际结果 |
|---|---|
| 已到期、空队列 | 拒绝，队列仍空 |
| 健康期限、空队列 | 入队，原请求值完整取回 |
| 队列持续 Full 至到期 | 拒绝，新请求未入队，原排队请求保持 |
| 接收端断开 | 拒绝 |
| 首次 Full 时未到期，跨期后释放槽位，再进入门 | 拒绝，释放后的队列仍空 |

第五项确定性构造与旧反例相同的“下一轮循环入口”状态，不声称强制复现操作系统恰在原 1 毫秒睡眠处的调度，也不称新冻结 S 的真实调度测量。编译和检查退出码均为 0；[完整输出](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-p4-independent/queue-fix-recheck/run.stdout)记录 `5 passed; 0 failed`。本轮没有启动产品服务、没有接触根既有 S/Q、没有写工作树或 Git/GH。

## 旧候选单列

旧候选 `f23d6016c29584d6f6d6e8281b362cc8b5f769ce` 的一个 P2 缺陷仍保留在 [REVIEW-OLD-P4.md](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/rust-p4-independent/REVIEW-OLD-P4.md) 与对应 JSON；原循环反例与旧 11 项坏库/健康/帧小域证据均未改写。新候选的关闭结论仅对应本报告已经实核的队列门。
