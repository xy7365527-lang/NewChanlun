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

launcher 不打开任何域的写连接；各域进程自行持锁与 epoch。S/E/B 任一退出 → 回收全部、退出码 1；
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
- S 库不存在：launcher 以 `init --protocol-revision s-session/3 ... --structure-source <state_dir>/structure-source.json` 建库，
  声明作为库固定属性落 `meta.structure_source`。
- S 库已存在：不带声明或声明与 manifest 不一致 → `InvalidDomain`，不改任何行。
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

## 已知卡点（#1769）

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
