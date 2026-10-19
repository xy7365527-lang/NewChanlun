# #1527 / I-1 R4 调用链（producer → Python 准入 R4 → CLI）

整合分支 `codex/1527-owner-integration-20261019`（HEAD 见票评论），Python 控制器 + Rust 二进制 `947e05be`（SHA-256 1125211c…6977）。

## CLI 边界

- 操作员入口：`python3 s_session/s_service_control.py <start-s|recover|status|stop-s|…> --config <launcher.json> --state-dir <state>`（`main` 在 `s_service_control.py:774`）。
- 配置：`s-launcher/3`，必填 `startup_timeout_ms`、`binary`（Rust 二进制）、`db`（既存库）、`s_resources` 七字段；可选 `admission_job_limits`（与锚内 limits 交叉核对）。
- Rust 子进程：`<binary> serve|recover --protocol-revision s-session/3 --db <锚定的真实 DB> … --admission-job-limits <state>/s-admission/limits.json`。

## 调用序列

1. `read_config` 校验启动配置。
2. `Controller.__init__`（`s_service_control.py:262`）：S 域在构造时固定本次启动绝对期限，取 `.control.lock` 与 `<db>.control.lock`，构造 `RecoveryAdmission(owner_only=True)`；捕获 `S_CONTROL_ADMISSION_SHA256` / `S_CONTROL_ADMISSION_IDENTITY`。
3. `start_s` / `recover`（`:612` / `:741`，均经 `admission_action` 装饰）→ `RecoveryAdmission.prepare`（`recovery_admission.py:177`）：E1 读 `<db>.control-owner.json` 并核 db/state 身份 → E2 打开 `<state>/s-admission`（或 pending）并核目录身份 → E3 读 `binding.json`、`limits.json`，核文件身份与 SHA-256 → E4 与 env 声明交叉核对。
4. `publish`（pending → 已发布，不覆盖原子改名）、`child_environment` 注入 env，`revalidate` 后起 Rust 子进程。
5. Rust `writer_cli.rs:195` → `admission_anchor::from_args_for_db`（`admission_anchor.rs:916`）以固定 FD 再核同一锚，成功后进入业务 Driver；失败打印 `R4准入拒绝：<码>；阶段=…` 并退出 1。
6. 失败时 Python CLI stderr 输出脱敏 JSON：`category`、`code`、`stage`、`wire_bytes`、`elapsed_ms`，不含原文或 env。

## 锚的来源

生产首份锚由谁签发归决策票 #1567（未决）。仓内唯一造锚途径是测试专用夹具（Rust `admission_anchor::test_prepare`，`#[cfg(test)]`；Python `s_session/tests/r4_anchor_fixture.py`），由 `test_tb01c_r4_anchor_lock` 机械锁约束。因此成功收据被 #1567 阻塞。
