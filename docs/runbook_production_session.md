# production_session 运行手册（simulation / dry-run）

> #1769（ADR 0039 迁移步骤 1，#1523-L）。本手册只覆盖可运行骨架；场所写代码、生产签锚（#1657）、
> P0 场所 profile 与成本项签署均不在本步范围。

## 构建

```bash
cd rust && cargo build --release --features production_session \
  --bin production_session --bin x_execution_session \
  --bin s_structure_session --bin e_economic_session --bin b_chong_session
```

`production_session` feature 只含 `s_session`（S/E/B 会话依赖与 X 骨架所需 rusqlite/libc），
不含 `backtest_bin`、`nautilus`（ADR 0039 B11）。launcher 从自身所在目录找四个域进程二进制。

## 进程与隔离

| 域 | 二进制 | 自有库 | 锁 | epoch |
|---|---|---|---|---|
| S | `s_structure_session serve --protocol-revision s-session/3` | `s.db` | `<db>.writer.lock` | `meta.writer_epoch` |
| E | `e_economic_session serve` | `economic.e.db` | 同上 | `meta(k='writer_epoch')` |
| Bᵢ | `b_chong_session serve --chong <id>` | `economic.b[i].db` | 同上 | 同上 |
| X | `x_execution_session serve` | `x.db` | 同上 | `x_meta.writer_epoch` |

launcher 不打开任何域库；各域进程自行持锁与 epoch。S/E/B 任一退出 → 回收全部、退出码 1；
X 退出只记日志，S/E/B 继续。SIGTERM/SIGINT → 向全部子进程发 SIGTERM，10 秒宽限后 SIGKILL，全部 wait 后退出码 0。

## manifest（`production-session-manifest/1`）

```json
{
  "manifest_revision": "production-session-manifest/1",
  "profile": "simulation",
  "state_dir": "/abs/launcher-state",
  "execution_capabilities": {"namespaces": ["hl.perp.BTC"]},
  "structure_source": {"category": "TestOnlyStructure", "namespaces": ["testonly.tick.ohlc"]},
  "s": {"db": "...", "session": "...", "catalog": "...", "profile": "...", "clock_plan": "...",
        "session_generation": "1", "delivery_retain_generations": "4",
        "admission_job_limits": "<state>/s-admission/limits.json", "writer_epoch": "1",
        "socket": "...", "resource_config": "<s-structure-resources/1 文件>"},
  "economic": {"manifest": "<economic-manifest/1 文件>", "e": {"db": "...", "epoch": "1"},
               "b": [{"chong": "one", "db": "...", "epoch": "1"}]},
  "x": {"db": "...", "profile": "<x-execution-profile/1 文件>", "writer_epoch": "1"}
}
```

### 结构源声明

- `category` 只认 `UsEquityStockLineTick`、`TestOnlyStructure`（ADR 0020:151-154、ADR 0023:9-34）。
- `namespaces` 与 `execution_capabilities.namespaces` 相交即 `InvalidDomain`，在拉起 S、创建 S 库之前退出——
  HL 行情即使与美股 bar 同形、类别标成个股线也不进 S。
- launcher 每次启动在拉起域之前，把 manifest 的规范声明写到 `<state_dir>/structure-source.json`（#1845）。
- S 库不存在：launcher 以 `init --protocol-revision s-session/3 ... --structure-source <state_dir>/structure-source.json` 建库，
  声明作为库固定属性落 `meta.structure_source`。
- launcher 每次调用 S `serve --protocol-revision s-session/3` 都传同一文件的 `--structure-source`。
  S 在取得自身写锁、完成原 admission 后，读取文件并与 `meta.structure_source` 的规范文本逐字比较。
  缺声明、文件不可读或损坏、库中声明损坏或不一致时，S 非零退出，不监听 socket；不一致错误点名两侧规范声明。
  launcher 按现有监督合同记录 S critical `exit`，最终 `stopped.failed=true`、退出码 1；S 错误文本写 S stderr。
- standalone S v3 `serve` 的 `--structure-source` 可选；不传时保持既有合同，不额外核声明。
- S.AcceptInput（`raw_input::accept_events`，v1/S2/S3 共用）对声明之外的 `source_namespace` 返回 `InvalidDomain`，
  不落行。不带声明的既有 TestOnly 库行为不变（但进不了 production_session）。

S `serve` 需要 admission 锚（`<db>.control-owner.json` + `state/s-admission/{limits,binding}.json`）。
生产首签归 #1657，未决前只能用 TestOnly 夹具造锚。

## X 执行 profile（`x-execution-profile/1`）

```json
{"profile_revision": "x-execution-profile/1", "mode": "simulation",
 "endpoint": null, "tb09_fingerprint": null,
 "credential_scope": {"env": "NC_X_VENUE_CREDENTIAL", "file": "/abs/credential-file"}}
```

- `mode` 必须等于 manifest `profile`。
- 凭据：launcher 读 `credential_scope.file`，只以 `credential_scope.env` 注入 X 子进程；S/E/B 以清空环境启动
  （只放回 PATH 与 S 的两个 admission 声明变量），launcher 自身环境不写凭据。凭据值永不落库、不打印。
- experiment/production 缺凭据：X 在构造任何传输之前以 `MissingDependency` 非 0 退出。
- production 缺 `tb09_fingerprint`：X 持久化 `send_state=refused`、`send_refusal_reason=MissingTb09Fingerprint`。
- 写调用（`submit`/`cancel`）恒返回 `UnsupportedCapability`；本步不含任何场所写代码。
- 查询：`x_execution_session query --db x.db`（只读）。

## simulation / dry-run 启动

```bash
production_session --manifest production.json   # 前台运行；Ctrl-C / SIGTERM 回收全部子进程
```

dry-run 与 simulation 同一流程：X 不需要凭据，不连接端点。

## 日志

launcher 日志写 stderr，每行 `production_session {json}`，经 `redact` 过滤（移植 `381fda3f`
`trading_system/execution/hl_paper_launcher.py:46-53` 的 `_redact`）：键名含
private key / secret / password / token / credential / api key（不以 `_env` 结尾）的值整体遮蔽；
24 位以上纯 `[A-Za-z0-9_-]` 字符串遮蔽。

## H1 生效条件

只有以 `s_session,s_incremental` 构建的 S 写者才启用 H1 准入。本手册[构建节](runbook_production_session.md#L9)的现有命令是 `docs/runbook_production_session.md:9-11`：

```bash
cd rust && cargo build --release --features production_session \
  --bin production_session --bin x_execution_session \
  --bin s_structure_session --bin e_economic_session --bin b_chong_session
```

该命令只启用 `production_session`，其依赖为 `s_session`，不含 `s_incremental`，因此不会启用 H1。feature 定义见 [`rust/Cargo.toml:81-91`](../rust/Cargo.toml#L81)。H1 写者构建沿用 [CI 的生产增量构建命令](../.github/workflows/ci.yml#L563)，出处 `.github/workflows/ci.yml:563`，在 `rust/` 目录执行：

```bash
../.github/scripts/run-cargo-with-telemetry.sh cargo build --release --locked --features s_session,s_incremental --bin s_structure_session --jobs 1
```

增量构建的 v3 `recover` / `advance` 必须显式传 `--resource-config`。允许参数表见 [`writer_cli.rs:82-93` 的 ADVANCE_FLAGS](../rust/src/bin/s_session_v3/writer_cli.rs#L82) 与 [`:104-113` 的 RECOVER_FLAGS](../rust/src/bin/s_session_v3/writer_cli.rs#L104)，必填检查分别在同文件 [`:464-468`](../rust/src/bin/s_session_v3/writer_cli.rs#L464) 与 [`:530-534`](../rust/src/bin/s_session_v3/writer_cli.rs#L530)。

`s-launcher/4` 的 `recover` 由 [`s_session/s_service_control.py:845-847`](../s_session/s_service_control.py#L845) 先确认原 S 已停止，再从该 `start-s` 进程记录的冻结 `resource_identity` 取路径，传入 `--resource-config` 并核验身份，见同文件 [`:862-868`](../s_session/s_service_control.py#L862)。`start-s` 冻结资源、传参并登记身份的位置分别是同文件 [`:725`](../s_session/s_service_control.py#L725)、[`:733`](../s_session/s_service_control.py#L733)、[`:744`](../s_session/s_service_control.py#L744)。不得另造资源档或改用当前配置的替代文件。

仅 `s_session` 的构建仍走原完整审计，不产生 H1 冷认证证据，不能作为 H1 验收。依据 `/Volumes/AgentStorage/h1-downstream46s/REPORT.md`，现有 [#1419 g77](https://github.com/xy7365527-lang/NewChanlun/issues/1419) 与 [#1673 全 95 harness](https://github.com/xy7365527-lang/NewChanlun/issues/1673) 都只编译 `s_session`，原样重跑不会走 H1。两处 harness 的增量构建与资源档迁移由独立后续单元承担，本节仅保留指针。

## H1 写入完整性 fail-stop

本节是 [#1481](https://github.com/xy7365527-lang/NewChanlun/issues/1481) 的 H1 冷认证到写者交接失败处置合同，适用于 Unix 上的 `s_incremental` 写者。完整性拒绝会保留 `NO_CKPT_ON_CLOSE=true`，刻意保留现场可能存在的 `-wal/-shm`，避免失败关闭时检查点改写主库/WAL。遇到下列拒绝，先停机保全，再交人工恢复门。

### 识别触发类别

[具名分类器](../rust/src/bin/s_structure_session.rs#L406) 精确匹配以下八类。表内路径相对仓库根目录。

| 错误原名 | 实际判据 | 分类位置 |
|---|---|---|
| `SourceIdentityChanged` | 路径或持有 FD 的身份/stat 与固定原件不符 | `rust/src/bin/s_structure_session.rs:409`；`rust/src/bin/s_session_v3/admission_provider.rs:75-86` |
| `SourcePathLost` | 路径元数据无法复核 | `rust/src/bin/s_structure_session.rs:410`；`rust/src/bin/s_session_v3/admission_provider.rs:77` |
| `SourceFdLost` | 持有 FD 的元数据无法复核 | `rust/src/bin/s_structure_session.rs:411`；`rust/src/bin/s_session_v3/admission_provider.rs:76` |
| `SourceMetadataUnavailable` | 原件元数据不可得，无法证明只是自身暂时失败 | `rust/src/bin/s_structure_session.rs:412`；`rust/src/bin/s_session_v3/admission_provider.rs:50`、`:64` |
| `SourceWalChanged` | 转换交接所需空 WAL 的类型、硬链接数、属主或长度不符，或固定 WAL 不是合法空 WAL | `rust/src/bin/s_structure_session.rs:413`；`rust/src/bin/s_session_v3/admission_provider.rs:141-149`、`:400-402` |
| `SourceTypeOrHardlinkUnsupported` | 原件不是普通文件/目录，或文件硬链接数不为 1 | `rust/src/bin/s_structure_session.rs:414`；`rust/src/bin/s_session_v3/admission_provider.rs:50-52` |
| `SourceChangedDuringPin` | 打开 FD 前后的原件 stat 不一致 | `rust/src/bin/s_structure_session.rs:415`；`rust/src/bin/s_session_v3/admission_provider.rs:64-66` |
| `ColdConversionMediumUnsupported` | 转换凭据要求 `MainOnly`，实际介质不符 | `rust/src/bin/s_structure_session.rs:416`；`rust/src/bin/s_session_v3/admission_provider.rs:388-390` |

Clamp 与迁移后 `data_version` 连续性复核还会以 `HistoricalAuditStale：` 为前缀报告下列完整性拒绝。固定文本逐字列出；底层 I/O 错误没有固定子类名，须保留完整错误文本。

| 错误文本或底层错误 | 实际置位判据与位置 |
|---|---|
| `HistoricalAuditStale：` 后接底层打开或元数据错误；`路径与持有文件身份/stat不一致` | 主库/WAL 打开失败，持有 FD 或路径 stat 失证，或二者 stat 不同。`rust/src/bin/s_session_v3/audit_owner/history_clamp.rs:45-65` |
| `H1组件要求既有WAL域，不自动改变journal_mode` | 实际 `journal_mode != "wal"`。同文件 `:144-149` |
| `HistoricalAuditStale：` 后接底层 WAL 头读取错误；`文件观测期间发生变化` | WAL 头读取失败，或读头前后主库/WAL stat 改变。同文件 `:203-221` |
| `锁内观测期间文件变化` | 一次锁内观测前后的文件集合不相等。同文件 `:229-254` |
| `私有认证交接期间发生外部SQLite提交` | 第二把锁首检的 `data_version` 与第一把锁继承值不同。同文件 `:258-263` |
| `版本、文件、控制原件、写计数或连接域改变` | 当前观测与夹持基线不相等。同文件 `:266-270` |
| `私有认证交接期间发生外部SQLite提交`；`失败后源连续性失证：{error}` | 迁移后任一失败收口时，复核 `data_version` 不等于继承值，或无法读取版本。`rust/src/bin/s_session_v3/audit_owner.rs:879-896` |

`HistoricalAuditStale` 前缀本身不是完整性分类器。明确的自身预算、期限、seed/adopt、迁移等失败，在未发现源完整性拒绝且迁移后连续性复核通过时，恢复正常关闭并返回原错误，可按既有启动合同直接重启，无需本节的介质人工恢复。`SourcePinFailed` 属元数据已核后的自身开 FD 失败；当前 `SourceWalMetadataFailed` 也未列入八类，不能擅自写成完整性拒绝。依据为 `rust/src/bin/s_structure_session.rs:402-430`、`rust/src/bin/s_session_v3/admission_provider.rs:153`；关闭配置恢复失败会把关闭错误附在原错误后，不能当成已经正常清理。若仍有旁挂或关闭失败，停止直接重试，保全后交人工门。

下次准入的[真实拒启条件](../rust/src/bin/s_session_v3/admission_provider.rs#L231) 是 `shm || (journal && wal_index.is_some())`，即主库 `-shm` 存在，或 `-journal` 与 `-wal` 同存，报 `UnsupportedWalShmJournalCombination`。因此完整性 fail-stop 保留的 SHM 会挡住下一次 `recover/start-s`；单独存在 WAL 不触发这一类别。重复启动不能代替介质恢复。

### 停机与证据保全

1. 使用同一配置和控制目录执行 README 已有的 status/stop，再查 status。下面逐字复用 [README:74-75](../s_session/README.md#L74) 与 [README:107-111](../s_session/README.md#L107) 的命令，路径须对应故障实例。

   ```bash
   ./s_session/launch_s.sh service status \
     --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
   ./s_session/launch_s.sh service stop \
     --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
   ./s_session/launch_s.sh service status \
     --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
   ```

   `stop` 按 Q→S 停止，仅管理已登记且身份匹配的进程。必须另行确认同库所有 S/Q、外部 SQLite 工具、调试器等进程与连接均已停止；status 不能证明外部连接不存在。停止超时、身份不明或仍有连接时，停在此步，不复制变化中的现场，不自动升级为 KILL，也不打开 SQLite 探查库。记录状态输出、停止结果及确认人。

2. 全部停止后，先记录原件绝对路径、设备号/inode、字节大小、mtime 与采集时间，复制后再次核对原件这些属性并核对副本字节。符号链接须按链接本身的元数据登记并记录链接目标，悬空链接也算现场原件。复制会产生新 inode，副本 inode 不能代替原件记录。主库、每个存在的旁挂、stderr 文件和控制目录内每份记录都须登记；不存在的旁挂也记录为不存在。复制或核对失败即停在人工门，保留错误，不覆盖旧证据包。

   保全清单包括主库、`-wal`、`-shm`、`-journal`，以及下列四个 audit-ledger 旁挂，名称依据 `rust/src/bin/s_session_v3/admission_provider.rs:117-123`：`.audit-ledger.sqlite`、`.audit-ledger.sqlite-wal`、`.audit-ledger.sqlite-shm`、`.audit-ledger.sqlite-journal`。同时保全数据库旁的控制/写锁及 owner 记录、整个原控制目录、冻结配置与资源原件、写者 stderr。launcher 管理的 S stdout/stderr 同写控制目录 `s.log`，进程记录为 `s.process.json/q.process.json`，依据 `s_session/s_service_control.py:329-330`、`:550-553`；若还有外部 stderr 重定向文件，也要复制。

   以下仅为文件保全命令，不是恢复命令。先将 `S_DB`、`S_STATE`、`S_EVIDENCE` 设为已核对的主库、原控制目录、全新仓外证据目录的绝对路径；证据目录须位于原件目录之外。`cp -RPp` 保留属性并复制链接对象，不跟随符号链接。任何命令失败即停止。元数据清单、status/stop 回执也存入该证据目录。

   ```bash
   (
   set -eu
   : "${S_DB:?须设置故障主库绝对路径}"
   : "${S_STATE:?须设置原控制目录绝对路径}"
   : "${S_EVIDENCE:?须设置全新仓外证据目录绝对路径}"
   mkdir "$S_EVIDENCE"
   mkdir "$S_EVIDENCE/database" "$S_EVIDENCE/control" "$S_EVIDENCE/logs"
   cp -RPp "$S_DB" "$S_EVIDENCE/database/"
   for suffix in -wal -shm -journal \
     .audit-ledger.sqlite .audit-ledger.sqlite-wal \
     .audit-ledger.sqlite-shm .audit-ledger.sqlite-journal \
     .control-owner.json .control.lock .writer.lock; do
     if [ -e "$S_DB$suffix" ] || [ -L "$S_DB$suffix" ]; then
       cp -RPp "$S_DB$suffix" "$S_EVIDENCE/database/"
     fi
   done
   cp -RPp "$S_STATE/." "$S_EVIDENCE/control/"
   )
   ```

   对清单中位于上述目录以外的配置、资源与 stderr 原件，逐份以 `cp -RPp` 复制到证据目录，保留路径映射。所有复制只保全，不移动或删除原件；复制不能保留原 inode，必须单独记录。

### 人工恢复门

**证据保全后停在人工恢复门。禁止手工删除 -shm/-wal。** 保持停写，不通过普通 SQLite 连接、手工检查点、改日志模式或换入口绕过准入。

已核对 [ADR 0042:38-42](adr/0042-s-writer-incremental-production-path.md#L38) 与 [README:111-120](../s_session/README.md#L111)：它们批准的是监督器 `recover`、增量持久态重建、随后 `start-s` 的既有链；README 的具体 `stop-s → service recover → start-s` 示例绑定冻结 op95、epoch 2 和 `recover-001`，不能直接套到任意完整性故障。这两份合同均未批准完整性拒绝后“用普通 SQLite 连接正常关闭以清理旁挂”。本节因此不提供该操作或后续可执行恢复序列，`recover` 也不能越过上面的介质拒门。

运维负责人将完整错误文本、触发类别、故障时间、二进制版本/摘要、配置/资源/clock plan、控制目录与进程身份/停止回执、外部连接停止确认、原件元数据清单和完整副本交给 #1481 主控；由 #1481 主控提交用户/恢复合同裁定人决定可信原件来源、冲突处置、允许的介质恢复步骤和与故障匹配的 epoch/恢复事件，并将批准结果补入运维合同。未经该决定不恢复写入。若后续批准普通连接正常关闭，须明确它会把 WAL 检查点进主库，操作后仍必须走 `recover/start-s` 的完整 H1 冷认证，不能将旁挂消失当成认证成功。

## 已知卡点（#1769）

> #1774 追加（2026-10-31，ADR 0057 追加修订段）：S 写者生产入口改经 `ThetaConfig::for_s_structure_writer()`
> 绑定已签的 synthetic profile（`venue-profile-synthetic-tb02a/2`），本卡点的 S 拒启原因已解除；
> 下文记录的是解除前的实测状态。replay `P0` 三项成本仍 `AwaitingPolicy{cost_inputs}`，
> 原八项生产验收与五项 TestOnly cfg 的去除仍归 #1769。

非 test 构建下 S 写者（v3 `serve`/`advance`、v1 `advance`）启动即因
`ThetaConfig::for_production_profile("P0")` 返回 `AwaitingPolicy{venue_profile, cost_inputs}` 退出，
launcher 因此会在 S 退出后回收全部进程、退出码 1。实测报错：
`AwaitingPolicy{venue-profile-P0.tick_size|slippage_bps|tax_bps|entry_delay_bars}：字段未签，不回退任何备用值`。
解除条件：P0 场所 profile（tick_size）与成本项（slippage_bps、tax_bps、entry_delay_bars）签署。
v1 `accept` 另被 `LegacyAdmissionBudgetAdapterMissing` 设计性关闭，S 唯一输入口是 v3 `serve` 的 socket ingest。

在此之前，`production_session_entry` 中依赖 S 写者运行的 5 个测试（simulation 拉起、凭据归属、
缺凭据时 S 照常接纳、S 拒收声明外 namespace、X 停止时 S 照常提交）会红；声明闸门本身由
`s_structure_session` bin 内单测 `v3::driver::tests::declared_structure_source_gates_accept_input`
在测试 Driver（`ThetaConfig::default()`）上验证。
