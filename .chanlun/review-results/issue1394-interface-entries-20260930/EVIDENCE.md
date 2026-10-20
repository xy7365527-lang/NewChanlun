# #1394 后端入口与读取接缝

本文件为绑定 open #1394 的工作草稿，承接已批准 #1340 接口增补与后端总图 #1323。产品前端 #1460 是独立图。本轮未 push、未合 main、未关票，goal 保持 active。

## 来源与版本

规范为 [已采纳原件](../issue1394-content-v3-20260916/APPROVED-SOURCE.md) 第 5.1–5.9 节、[增补登记](../issue1394-content-v3-20260916/SPEC-ADDENDUM.md)，以及已签 R2 SPEC 的 C09 与 S 接口合同。没有修改这些原件、旧封存方案、主仓未提交改动或 GOAL_SET/SUPERSEDE。

固定基线 `30f0f9e6eb38658f3df8627bd52e3083e8b4c343`。本轮整合在 `codex/1394-content-v3`；最终产品提交 `bdc50d324a0f1b858597257dcfc34b9302dccf1a`。本封包是后端局部进展的代码、验证与独评实体，不构成 #1394 联合验收或 main 合入就绪证明。

| 职责 | 隔离产品提交 | 整合提交 |
|---|---|---|
| E/B reader 资源与运输 | `0b59b46b1ff54ccb96eaf7189ee8f7c0f6307c8f` | `5c0dd7e361` |
| S3 writer CLI | `c27023c2057b74d15d1f8dce29ad0fae6891b79e` | `1a2d852cb4` |
| S 财务生产者 | `69867b59d64f68da3bee20851a569d340325f483` | `508fbac879` |
| 解析完成后的绝对期限 | `ae8c0ca0fde8cf449976d5bfbb6d53f83ef490c0` | `879d03a076` |
| pending 原身份、资源文件类型、profile 成交量 | `87a7b6d82879117b635cc4194031fa9f49785f96` | `b24cea679c` |
| 财务 DB 类型前检 | `316c8264338d85a66811372e33bd9774d41ea5a8` | `a20bdee2e0` |
| stdin 等待期间固定 DB 文件身份 | `424e3fae829c3fd01bc87f01ed6e6413dce7b20f` | `9611a33a9c` |
| 接纳前冻结 input profile/schema 对拍 | `bf25c5ed76d660b8d12a7181f2c16f2d8eebc8c2` | `bdc50d324a` |

共享 `mod.rs` 的冲突按两条独立路由合并，保留 writer 与 financial 分派；新财务文件与隔离产品字节相同。没有以覆盖一方解决冲突。

## 实际能力与未接线部分

S3 `advance` 可显式接收单份 envelope，也可仅推进原持久 pending；`recover` 保留严格升 epoch 和原身份。缺参、重复参数、typed payload、会话、clock 与已存 message/receipt 身份冲突均有前检，持同一写者锁后在自动恢复前重复执行原判据。正常新输入仍使用唯一 Driver。成交量来自已核冻结 profile 的 `volume_unit`，不再把 writer epoch 当作成交量。

原 `accept_core` 的 input schema/profile 两条配对条件及错误文本迁入同一纯 helper，旧入口与 S3 只读预检、持锁后恢复前检查、实际 raw 接纳前共同调用。合法旧 tick/price 输入不能进入已冻结 OHLC 的 S3 域，不增加非 OHLC 成功能力。

S3 `financial-basis-ref` 与 `financial-content` 使用 stdin 单份严格 JSON 到 EOF；显式选择 `s-session/3`。生产者在 SQLite 只读事务中核当前控制元组、pending、时钟与完整来源，返回七字段逻辑 basis、二十字段当前 snapshot、完整 wire 对象承诺和独立内容证明。财务 anchor 与 Q anchor 不混用。内存派生节点不写 S 权威库。

财务 CLI 在等待 stdin 前打开只读连接、核普通文件与 dev/ino；EOF 后再次核文件身份，再开启短读事务。等待期间不持读事务，不以 mtime/length 拒绝正常同库写入，也不冻结旧观察。实际连接仍用原 DB 路径及 WAL，没有 `immutable=1` 或 `/dev/fd` 替代。

E/B 新运输 API 解析六字段 reader 配额及原八字段 JobLimits；调用方传入同一 Job，限额必须与冻结配置相等。真实 stdin/stdout/stderr 字节、错误与重试累计；取单次、操作及 Job 的最早绝对期限，并在 JSON 解析和 Job 时钟检查之后再次复核。旧 `financial-basis` reader 的 16 MiB／10 秒函数体保持。

**未完成：**新 reader 尚未接入 `finance.rs`／manifest v2 的经济消费、全部历史引用闭包原子导入、Prepare→Reserve→Commit 或离线恢复。接口类型与成功局部测试不替代这些义务。

S3 `serve` 仍在取得写锁、开库和监听前返回 `AwaitingPolicy`。缺具名部署资源、旧 `audit_cache_source_bytes` 的合法映射，以及 C09 持久条件请求 FIFO／稳定点消费能力；ready／receipt／ingest 不冒充条件队列。没有填默认数字或用 Q16 配额替代 S writer／E/B reader。

## 检查与失败记录

根工位在整合产品上真实执行：

| 检查 | 结果与范围 |
|---|---|
| `cargo test --lib --features s_session financial_reader_tests -- --test-threads=1` | 8 passed，0 failed；绑定 `a20bdee2e0`，后续仅 S 财务文件变化，运输字节未变 |
| `cargo test --features s_session --bin s_structure_session -- v3::driver:: v3::financial_reader:: --test-threads=1` | 18 passed，0 failed，1 既有显式成本测量 ignored；绑定 `a20bdee2e0` |
| `cargo build --features s_session --bin s_structure_session` | exit 0；绑定 `9611a33a9c`，真实非 test CLI |
| `S3_FINANCIAL_CLI_BIN=<该构建> cargo test --features s_session --bin s_structure_session real_cli_reads_current_cut_after_stdin_wait -- --ignored --test-threads=1` | 1 passed，0 failed；显式运行了新增 CLI 用例，绑定 `9611a33a9c` |
| `cargo clippy --features s_session --lib --bin s_structure_session` | exit 0，仍有告警；绑定 `a20bdee2e0`，不称 lint 全净 |
| `cargo test --features s_session --bin s_structure_session old_tick_schema_cannot_enter_frozen_ohlc_s3_domain -- --test-threads=1` | 1 passed，0 failed；绑定 `bdc50d324a` |
| `cargo test --features s_session --bin s_structure_session tb02a_sealed_revision_history_and_independent_fact_index -- --test-threads=1` | 1 passed，0 failed；绑定 `bdc50d324a`，旧入口回归 |
| `cargo build --features s_session --bin s_structure_session`，最终产品补修后 | exit 0；绑定 `bdc50d324a`，没有把较早构建冒充新字节构建 |

真实 CLI 用例用 lsof 确认只读连接已打开；stdin 尚未 EOF 时由正式 Fixture S3 writer 提交新 cut，EOF 后 basis 的 snapshot 引用等于新 cut，后续 financial-content 成功。它证明受测小会话，不证明全 9011 或部署容量。

工作者及独评动态反例保留：

- 初始 Driver 测试曾有一项把 init 已创建的锁文件误当新副作用；改为真实持锁验证后通过，不隐去首轮失败。
- transport 初评 HIGH：60 ms 操作期限下，合法 1,500,004 字节 JSON 在 76／77 ms 返回 OK；150 ms 期限下，6,000,004 字节在 227 ms 返回 OK。新返回前期限门和解析耗时回归闭合。
- writer 两个持久身份冲突红测确实先提交旧 pending；补修后 generation、clock 和原 envelope 保持，2/2 从红到绿。
- 资源档 FIFO 红 1 → 绿 1；类型前检、非阻塞打开及 fd 类型复核闭合。
- epoch=2 时 volume=2 曾被错收；沿冻结 profile 的 unit=1 修正后，volume=2 拒绝、volume=1 新输入提交。
- 合法旧 tick/price 形状在冻结 OHLC S3 库中曾返回 Err 却留下 `raw_events=1`；共享原配对判据后，CLI 与直接 Driver 均拒绝，raw/message/clock/generation 保持 0，反例 1/1 与旧入口 OHLC 1/1 回归通过。
- 初始财务路径测试 8/12 超时；普通类型前检后 2 秒复测 12/12 exit 1，0 timeout。0.7 秒冷首跑仍曾有 1 次超时，不删该记录或称生产期限通过。
- stdin 期间普通 DB 换 FIFO 曾 2/2 超时；固定连接／身份后 FIFO 与另一普通文件替换各 2/2 返回 MissingDependency，静态 12/12 拒绝。旧超时子进程均限时终止、回收。
- 管理者另一次 S16 大原件只读 basis 探针在 45 秒观察限内超时并回收，原 DB SHA 仍为 `1704c9d9ef50a822653206e0bf1136b37280814322bbb44398761a38322c154a`。不计为成功、资源达标或 Q16 复测。其构建模式、完整命令和配额不足以支持部署判断。

原始日志在本票 `.scratch/issue1394-{s3-writer-entry,s3-financial-producer,finance-reader-transport}-20260930/`；本报告与随附摘要保存可关票引用的结论实体，原始临时工位和日志仍为未跟踪的本票草稿。未改 formal，无新增 fixture 漂移声明；未执行本轮远端 CI 或 DevSkim，不借 #1392 的旧扫描冒称本轮已扫。

## 独评与边界

实施者与评审者独立。初评分别覆盖三个固定产品差异；统一尾审覆盖 `508fbac879 → a20bdee2e0`，补审覆盖 `a20bdee2e0 → 9611a33a9c` 与 `9611a33a9c → bdc50d324a`。已知三个 HIGH、资源 FIFO MEDIUM、财务静态／stdin 替换 MEDIUM 均闭合，末份 Spec／Standards PASS 仅指该明确差异。详见 [独评实体](REVIEW.md)。

fresh message 的原始 source_coord 冲突或不合法 volume 仍可在独立旧 pending 恢复后返回业务错误。独评实测 generation 0→1、clock 行 1→3，未改原 envelope；既有合同没有规定所有业务拒绝均须回滚独立恢复，不将它混同已修的 message／receipt 原身份冲突。

新财务路径不支持 DB symlink，本轮规范未要求该支持。初次 metadata 到 SQLite 打开的极短敌对路径替换并未得到原子 handle 身份证明，调用侧绝对期限仍须生效；同步 OS spawn 自身也无法在同线程硬中断。未验证 generation=0、真实跨 epoch 财务读取、全量资源峰值、所有累计闭包或 9011 双跑。

## Sandcastle 的真实状态

两个产品执行 run `4e8e8944-b299-4633-9785-afe90fa494d9`、`56ee2d09-684a-4936-969d-b6cfd509ef5f` 都因 macOS fs-helper 卡在 getpwuid_r／DirectoryService 被取消、回收。实际产品代码由原生管理子代理接管，不称 Sandcastle 产品实施成功。

独立 canary run `5cfa0631-71bf-4fe6-a3a6-c83bf9864c18` 真实 completed／exit 0，DeepSeek V4.1 Flash max，保留 workspace-write。外工蜂仅通过 shell 重定向把 alpha 改 beta；未调用 apply_patch。Python 启动仍挂起。管理者外部核验仅一文件差异并提交 fixture `050b9626dc77d05048868f98b3ef2c3a4b8934dd`；不是外工蜂提交产品。根工位再核 fixture 干净、无 Python 进程 cwd 指向该 fixture。

这个 canary 证明受限 shell 编辑路径可用，不证明 Mac/helper 已修复、完整工蜂能力或产品交付。后续 Sandcastle 任务应保留隔离和原权限，以有界 shell 编辑、真实测试及调用方验收验证，不再把拿到 PID 或普通文本完成当作结果。

## 下一项仍在本图

继续新 E/B 当前来源消费、全部具名历史路径的本地完整证据闭包、同一经济操作 Job 与来源重读、三阶段及故障／恢复验收；形成具名实际资源后再运行服务、9011 双跑与联合验收。S3 条件请求与正式服务接线也仍需补齐。前端产品留在 #1460，不据此延长 #1323 的产品页面范围；后台事实、协议、观察与恢复义务保留。
