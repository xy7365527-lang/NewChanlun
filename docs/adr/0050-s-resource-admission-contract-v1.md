# ADR 0050：S 资源准入合同 v1 与操作员签撤

> 正本范围：S 资源准入合同 v1 取值、版本登记和操作员入口。候选，待非作者独审。
> 依据：#1567 / ADR 0046、#1657；主控 2026-10-22 依 #1323 Notes N-2 审定取值规则。
> 受影响代码：`rust/src/bin/s_session_v3/admission_issuance.rs`、`s_session/s_service_control.py`；入口登记见 ADR 0039。

## 当前合同不能签锚

版本 `s-admission-small/1-awaiting-measurement` 只面向 S86 / MIN12 小库的 bounded-layout/1。9011 留给 v2。规范文件为 `s_session/contracts/admission-small-v1.json`。JSON 对象键排序、紧凑编码、无末尾换行的 SHA256 是 `5ef9d83dd6eb1ae4ebd9664d3de576c847ce0267be49eaad41bed29d1e7f636f`。

| 字段 | v1 值 |
|---|---|
| max_store_bytes | 68719476736 |
| max_staged_bytes | AwaitingMeasurement |
| max_total_node_bytes | AwaitingMeasurement |
| max_total_nodes | AwaitingMeasurement |
| max_total_records | AwaitingMeasurement |
| max_total_wire_bytes | AwaitingMeasurement |
| max_total_requests | AwaitingMeasurement |
| max_elapsed_ms | AwaitingMeasurement |

64 GiB 沿用 ADR 0037；七项不得用测试值或 v2 推导值代填。#1675 对 S86、MIN12 各作一遍完整准入审计，以两库较大峰值乘 2，独审后更版。不得为两遍审计自行翻倍。启动器窗口必须不小于合同 max_elapsed_ms，不够时改启动器配置，不缩合同。

#1675 更版时还须给出适用 S86/MIN12 的可信辨识证据和接缝。目前材料未给可用于机器判定的布局/具名库标识，本版本全局拒签，未用文件名或任意 CLI 字符串冒充该证据。按主控本轮指令，不新增部署 path/dev/inode 编译白名单，不把每次合法重建变成重编译政策。具体库身份仍由 R4 binding 与原 FD 核验承担。

## 一个注册表，两种调用

生效版本及规范合同编译进 S 二进制，签发和启动校验共用 `Contract::authorize`。输入文件仅是待核原件，不是清单；运行时没有环境变量、外部 JSON 或 CLI 开关可替换注册表。合同更版必须经审查构建。部署仍须使用受审二进制；内嵌清单不声称抵抗替换整个程序。

`AwaitingMeasurement` 不能解析为生产 JobLimits。版本缺失、旧版本、未知摘要、未实测值、启动窗口不足均拒绝。签发前先核合同，失败不创建 pending/published。

## 操作员用法

当前版本执行以下签发命令会返回 AdmissionContractAwaitingMeasurement。七项实测并独审更版后，替换版本名和合同文件。

```sh
s_structure_session admission-prepare --db /absolute/source.sqlite --state-dir /absolute/state --service-uid 501 --contract-version s-admission-small/1-awaiting-measurement --contract s_session/contracts/admission-small-v1.json --startup-timeout-ms 10000
s_structure_session admission-revoke --db /absolute/source.sqlite --state-dir /absolute/state --service-uid 501 --revocation-id operator-change-1
```

service-uid 是部署服务的实际 uid，命令须以同 uid 执行，并核 DB 和控制原件 uid；示例 501 不是项目政策值。签发不建立数据库和 state 目录，不授予交易发送权。活写者、maintenance/control 占用、既存 published/pending、未完成撤销、外来 uid 或可被组/其他用户改写的 DB 均拒绝。路径每级无跟随，DB 已解析路径沿 R4 既有语义。

签发持锁顺序为 state.control、DB.control、DB.writer、DB.maintenance。先写 0700 pending 与 0600 limits/binding，fsync 后原子更新 owner，再 no-replace 发布。失败保留证据，不清理后重签。owner 已落盘的完整 pending 可由既有 Python publish 续发布；不完整 pending 使用显式撤销。

撤销先落持久 intent，再把原目录移入 `.s-admission-revoking`，将 owner 改为保留 db/state_dir 的两字段记录，最后改名 `s-admission-revoked-<id>`。原件与 intent 保留。已落 intent 后中断，用同一 revocation-id 重试，仅补未完成步骤；不同 id、对象身份改变或已有归档拒绝覆盖。若在 intent 完整持久化前发生 I/O 故障，保留半成品并拒绝自动恢复，须按现场原件人工处置，不宣称任意磁盘损坏可自愈。换锚须先完成显式撤销。

## 启动与控制分开

launcher 的写动作链是 Python prepare 原件核验及前导记账 → admission-contract → publish → Rust R4 → 服务。此处 ADR 0046 的「调用 R4 之前」指 Rust 消费入口前，主控本轮已采纳。合同命令只取 Python 已核摘要，不重复读取控制原件；与后续进程共用原启动绝对期限。合同失败不 publish、不启动，不修改原锚或数据库。Controller.close 释放控制锁，下一合法生命周期可重新取得。

配置新增 `admission_contract_version`，仅 start-s/recover 要求。status/stop 不读合同，不因旧合同、坏 limits 或缺版本丧失控制能力。`recovery_admission.py` 与 Rust R4 六身份字段、八项 exact 检查未改。

## 验证与后续

靶向 Rust `admission_prepare_` 实跑含九项具名负控、pending、窗口、mode、签撤再签正控与撤销中断恢复。Python 合同门测试真实调用 Controller 和子进程，旧生命周期测试在测试夹具中显式隔离合同门。release 必须无 test_prepare 符号，且真实 admission-prepare 对本版合同拒签。CI 为 `.github/workflows/issue-1657-admission.yml`。

本票不提供正式成功收据；#1675 承接实测、适用库辨识、有效合同新版本和正式成功收据。合入顺序 #1527 → #1657，不把失败收据重新绑定到本票。
