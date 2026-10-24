# #1626 版本化字段实施与验证记录

日期：2026-10-25。工位：`/Volumes/AgentStorage/issue1626-versioned-fields-20261025/worktree`。
代码候选：`1af4e64ae0`；本报告单独作为文档提交，不把后续文档 SHA 当代码变更。

## 实际读取链

- **Q-4 `trend_scope`**：`PolarityMode` 的模式串只选择 profile；`run_positional` 经
  `active_trend_scope_for_profile` 读取 `trend-scope.json`，把 `policy.value` 送入 `run_fusion`，
  并将 `trend-scope/1` 写进 `PositionalResult.trend_scope_policy` 和 Python dict。
  `default=SelfLayer`、`hold26_anc=Ancestor`、`fusion_ta=SelfOrAncestor` 分字段读取，未用全局 SelfLayer 覆盖研究臂。
- **Q-6～Q-8 `tower_base`**：`tower-base.json` → `try_level_config_from` → `LevelConfig`。
  `classify` 在 L0 前拒绝非 segment；`compose_level` / `compose_level_resume` 接收 `config.level.tower_base`，
  `TowerPolicyStamp` 输出 `tower-base/1`。Python `active_tower_base` 经中间层和最终层消费同一记录。
  `kline3` / `trade3` 可解析但没有真实构件与数据 profile 时 typed reject，未用线段输入冒充。
- **Q-2 `selector_keys`**：`selector-keys.json` → `SelectorPolicyIO` → 四个合法 `SelectorKeys` 构造子。
  `selOrderWithKeys` / `selOrderBWithKeys` / `selectΘWithKeys` 真正按字段比较；`SelThetaRanked` 先过滤再按键选择，
  乱序输入仍选同一记录值。`endTime` 固定降序，未批准的升序在 IO 边界拒绝。
- **X-38 `tangency`**：`segment-second.json` → `TangencyPolicyIO` → `HasGap` / `Overlaps`。
  `gap_iff_not_overlap` 对两值分别成立；参数继续穿透 `SegmentFeatureComplete`、`SegmentAutoConstruct`、
  Claim10 `classifyTermination` 与 Claim5 `wellFormedV0` / `MoveWellFormed`。
  Origin/Claim10 显式 `closed` 保持现行行为，Claim5 显式 `open` 保持历史参考行为；没有隐藏的第二默认值。
  `ParityFixtureExport` 直接读取现役 `segment-second.json` 并传给导出函数。

## 测试与证据

- RED→GREEN 记录：`/tmp/nc-remote/20261025-025639-3b26e99abc.log`（缺记录类型/函数）、
  `/tmp/nc-remote/20261025-030233-e354ee63d9.log`（缺生产版本戳字段）、
  `/tmp/nc-remote/20261025-031010-599f985f57.log`（缺 compose 基底参数）。
- 最终 Rust：`/tmp/nc-remote/20261025-035202-1af4e64ae0.log`。
  `cargo check --locked --all-targets` 通过；相关测试 **155 passed / 0 failed / 5 ignored**。
  覆盖 policy records、tower policy、Fusion 78 项、compose、coverage element、incremental parity、voice_eat、pipeline_geometry。
- Python：`/Users/silencehan/Projects/NewChanlun/.venv/bin/python -m pytest ...` → **94 passed**。
  含 tower-base 记录、nested divergence、nested pipeline、endpoint、segment policy。
- Lean：`lake build` → **161 jobs**；`Origin/VersionedSelectorTests.lean` 与 `Origin/VersionedTangencyTests.lean`
  分别输出 `SELECTOR_POLICY_IO_OK`、`SELECTOR_NEGATIVE_OK`、`TANGENCY_POLICY_IO_OK`、`TANGENCY_NEGATIVE_OK`。
- `python3 scripts/check_fixture_drift.py`：center / certificate_chain / parity 均 **0 字段差异，字节一致**。
- `cargo fmt --all -- --check`、`git diff --check` 通过。未在本机运行 cargo/rustc 测试，Rust 验证均由 nc-remote 完成。

## 边界

- 既有六份 Lean 注释提交只作定位说明，未计作实现；本次是实际记录、生产读取、证明与 IO 验收。
- 没有新增 `sorry` / `admit` / 公理；既有 Lean 公理依赖未扩大。
- 不把 #1621 的 X 执行域、#1673 的 FIN/2 协议门或 #1323 全图六项 Destination 视为本票已完成。
- 非作者独审、GitHub CI、用户 main 合入闸及关票门仍待完成；本报告不表示已合入或已关票。
