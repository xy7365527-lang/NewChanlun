# #1768 冻结 pre-W 旧规则 oracle 重建夹具

七份 SQLite 是 `tb01c_query_tests.QueryTests` 与 `test_tb02d_projection_cache.ProjectionCacheTests` 42 项历史读的输入。它们是用已批准 TestOnly 旧 oracle 在冻结 pre-W 源码上**重建**的同输入材料，不是找回的历史原件（#1419 comment6265726014、comment6449214001）。

| 文件 | generation | raw_events | 用途 |
|---|---:|---:|---|
| `g0..g4.sqlite` | 0..4 | 0..4 | 四价格 10000、11000、10500、12000 逐次 accept+advance 后的切面；`g4` 为 QueryTests 种子 |
| `g0-pending.sqlite` | 0 | 1 | init 后 accept seq0/price11500，未 advance |
| `g4-pending.sqlite` | 4 | 5 | g4 后 accept seq4/price11500，未 advance |

会话合同逐字取自 `tb01c_query_tests.QueryTests.input`：session `tb01c-query-unit`、namespace `testonly.tick.ohlc`、epoch 1、instrument `TEST.TICK`、profile `testonly_tick_1_1_ohlc`。规则串为 manifest 的 `rule_revision_const`，即 W 之前的旧规则。

## 来源与重建

- 源提交 `283635555c539fac8e27b304dd4257d5a09531c9`，分支 `codex/1768-testonly-seed-29ax`。导出器是 `rust/src/bin/s_structure_session.rs` 的 `cfg(test)` 模块 `issue1768_seed_export`，在 pre-W 旧 parser 上经已有 mixed 真计量 `WriterScope` 写入，八帽 1000000，native RAM 两项 `1 << 26`。
- 生成运行 https://github.com/xy7365527-lang/NewChanlun/actions/runs/40873859891 （completed/success），工作流 `.github/workflows/issue-1768-seed-export.yml`。本目录文件与该 run artifact `seed/fixture-advance-testonly-policy-records/frozen/` 逐字节相同。
- 重建：在上述提交上 push 触发同一工作流，或执行 `cargo test --locked --features s_session --bin s_structure_session -- --ignored --exact issue1768_seed_export::issue1768_seed_export_fixture_advance --nocapture`（环境 `S1768_SEED_EXPORT_DIR`）。Rust 仅在 CI 或 nc-remote 构建。
- `export-manifest.json` 是该次导出的原 manifest（含每步准入账）。`export-validator.json` 是原公共 `AuditReader` 对七库私有副本的核验输出。

## 读取约束

原件带持久 WAL 头。普通打开会在旁边生成 `-shm`/`-wal`。测试只经 `tb01c_query_tests.load_frozen` 读取：存在任一侧件即拒绝，否则 `immutable=1` 只读并 backup 到测试私有副本。负控修改只施加在私有副本上。
