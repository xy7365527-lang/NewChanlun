# CLAUDE.md

基因组已迁至 `AGENTS.md`（唯一正本，2026-07-28 #517 裁定：Kimi 主力运行时，AGENTS.md 为全 harness 正本入口）。本文件仅作 Claude Code 兼容指针保留。

## 形式化验证节拍（fixture 漂移 gate，issue #263 / #245 裁定）

凡改动 `formal/` 的 session，**收尾必跑 fixture 漂移检查**（两个机器导出 fixture ↔ Lean 源一致性）：

```bash
python3 scripts/check_fixture_drift.py
# 或（cargo 入口，#[ignore] 默认不跑）：
cd rust && cargo test --release --test theta_v0_fixture_drift -- --ignored --nocapture
```

- exit=0 无漂移；exit=1 真漂移（打出字段级差异——regen 重落 fixture 或回退 Lean 改动）；
  exit=3/4/5 为工具链/构建问题（**非漂移**：3=lake 不在 PATH，4=lake build 失败，5=导出器运行失败）。
- 冷环境（`formal/.lake/build` 缺失/为空）脚本先 `lake build` 全量构建（自包含无外部依赖；
  实测耗时见 `scripts/check_fixture_drift.py` docstring）。
- 覆盖 fixture：`rust/tests/fixtures/theta_v0_parity.json`（导出器 `formal/Origin/ParityFixtureExport.lean`）、
  `rust/tests/fixtures/theta_v0_center_parity.json`（导出器 `formal/Origin/CenterConstruct.lean:634`）。
  改动这两个 fixture 对应的 Lean 源后，用 `cd formal && lake env lean <导出器> > ../rust/tests/fixtures/<fixture>`
  重落盘（机器导出，禁手编）。
- CI 闭环（#265）：`.github/workflows/ci.yml` 的 `fixture-drift` job（push/PR 到 main 触发，与现有 job 一致）
  跑同一脚本，漂移即红并打出再生命令。
