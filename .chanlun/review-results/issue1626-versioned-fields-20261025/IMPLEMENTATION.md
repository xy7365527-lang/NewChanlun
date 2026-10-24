# #1626 版本化字段实施与验证记录

日期：2026-10-25。工位：`/Volumes/AgentStorage/issue1626-versioned-fields-20261025/worktree`。
首次代码候选：`1af4e64ae0`；Q-4 独审返修候选见末节。本报告单独作为文档提交，不把后续文档 SHA 当代码变更。

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

## Q-4 预注册准入返修（2026-10-25）

当前代码候选：`05ecc0338c`。非作者独审 H-1 指出三个 profile 都可接受三种 trend_scope，形成未预注册祖先臂；本节为实际返修，不改写前述实现与原始证据。

- `TrendScopeProfile` 现在绑定唯一预注册值：`default=SelfLayer`、`hold26_anc=Ancestor`、`fusion_ta=SelfOrAncestor`。`parse_trend_scope_for_profile` 对其它组合返回 typed `InvalidPolicy`，在 `run_fusion` 前停止；`default=Ancestor` 的旧错误测试已改成拒绝。
- `run_positional_with_trend_scope_record` 与生产主入口共用同一解析和准入门；`None` 仍读取现役记录，没有新增第二生产配置源。测试注入仅用于合法新版本和变异验收。
- `trend-scope.json` 的 options 改为各 profile 的真实边界；新增作用域仍须另行预注册，不是全面禁止合法新版本。
- 生产入口负控覆盖全部六种跨 profile 组合：fusion_t 的 Ancestor/SelfOrAncestor、hold26_anc 的 SelfLayer/SelfOrAncestor、fusion_ta 的 SelfLayer/Ancestor，均返回对应字段的 `InvalidPolicy`。三个合法 profile 的新版本均通过并携带 `trend-scope/2`。
- 首轮 `b0ae62cd4b` 的负控已通过，但正例夹具缺 `trend_flips/dir_flips`，不能计作通过；`05ecc0338c` 只补合法磁带后重跑。
- 最终 nc-remote 日志 `/tmp/nc-remote/20261025-041454-05ecc0338c.log`：policy records 11/11、positional 16/16、Fusion 78/78，共 **105 passed / 0 failed**；`cargo check --locked --all-targets`、`cargo fmt --all -- --check` 均通过，EXIT=0。
- Q-6～Q-8、Q-2、X-38 的实现未因本返修改写；当前 head 仍需重新整票独审、GitHub CI 和用户 main 合入闸。

## Q-2 原证书链返修（2026-10-25）

当前代码候选：`f34b8c823e1dfaede3b6b93b33a63b14b590213f`。前次独审 H-1 指出 `NestLevel.chosen` 仍走旧硬编码选择器；本节为实际生产链返修，不改写前述实现和原始证据。

- `NestLevel` 现在携带 `SelectorPolicy`，`chosen` 调用 `selectΘWithKeys ℓ.policy.keys`。`nestCertB` 要求相邻级 `policy` 完全相等，`NestCertPartialCorrect` 同步记录这一前件；混合记录的证书链直接失败，不允许每级各取默认。
- `ChosenByΘ` 使用 `IsSelectedByKeyWithKeys`，新增通用键表的折叠成员、最优性、传递性、三歧性和唯一性证明。旧 `selOrder`/`selectΘ`/`IsSelectedByKey` 仅是显式测试默认的别名，不再是生产 `chosen` 的独立配置源。
- `SelectorPolicyIO` 新增 `readNestChain` 与 `chiBoolWithRecord`：实际 JSON 在 IO 边界读取一次，经 `nestChainWithPolicy` 注入同一证书链，真实贯通 `subB` / `nestCertB` / `chiBool`。缺记录、null、`AwaitingPolicy`、非法方向及未批准 `endTime=ascending` 均拒绝，未回退默认。
- `Origin.VersionedSelectorTests` 已加入 Lake 根目标，构建会持续执行：`chosen_default_matches_legacy` 默认逐位等值；合法 start_time 变异经 `NestLevel.chosen` 改变选择；`chiBoolWithRecord` 消费 version 2；`mixed_selector_chain_is_rejected`；缺文件、null、AwaitingPolicy 与非法值负控。
- 最终验证：`lake build` **162 jobs** 全过，日志含 `NEST_CHOSEN_DEFAULT_OK`、`NEST_CHOSEN_MUTATION_OK`、`CHIBOOL_RECORD_OK`、`SELECTOR_MISSING_REJECT_OK`、`SELECTOR_NULL_REJECT_OK`、`SELECTOR_AWAITING_REJECT_OK`、`SELECTOR_INVALID_REJECT_OK`。`python3 scripts/check_fixture_drift.py` 三项仍 **0 字段差异、字节一致**；`git diff --check` 通过。
- Q-4 的预注册准入、Q-6～Q-8 的塔底记录与 X-38 的相切语义未因本返修改写；没有新增 `sorry` / `admit` / 公理。当前 head 仍需重新整票独审、GitHub CI 和用户 main 合入闸。

## X-38 相切行为与 IO 验收返修（2026-10-25）

前次整票独审指出：Claim5 的 `open` 没有真实三笔相切行为断言，`Origin.VersionedTangencyTests` 未进入默认构建，IO 负控缺缺记录、null 与非法值。本节为对应验收返修，不改变生产定义、Q-4 预注册准入、Q-2 原证书链或 tower_base。

- 新增同一三笔相切实例 `tangentClaim5Segment`：三笔公共区间只在价格 10 相切，逐笔方向、端点分型和价格均良构。构建期证明 `wellFormedV0 closed` 成立，而 `wellFormedV0 open` 不成立；不再以 `#check` 或 closed 函数绑定冒充 open 行为。
- 同一边界在 `HasGap` / `Overlaps` 与 Claim10 `classifyTermination` 下也分别验证 closed/open 的相反结果；Origin/Claim10 继续 closed，Claim5 分别检查 closed/open，未改变既有语义。
- `Origin.VersionedTangencyTests` 已加入 `formal/lakefile.toml` 的 Origin roots。默认 `lake build` **163 jobs** 全过，日志明确包含 `Built Origin.VersionedTangencyTests`，不是仅手工执行一个未挂根文件。
- 现役记录读取与合法 open 新版本均覆盖：`TANGENCY_POLICY_IO_OK`、`TANGENCY_OPEN_MUTATION_OK`。
- IO 负控完整：缺记录、null、`AwaitingPolicy`、非法相切值分别拒绝，日志为 `TANGENCY_MISSING_REJECT_OK`、`TANGENCY_NULL_REJECT_OK`、`TANGENCY_AWAITING_REJECT_OK`、`TANGENCY_INVALID_REJECT_OK`；没有备用默认或意外接受。
- `python3 scripts/check_fixture_drift.py`：center / certificate_chain / parity 均 **0 字段差异、字节一致**。`git diff --check` 通过；没有新增 `sorry` / `admit` / 公理。未在本机运行 cargo/rustc 测试。
- 本次只改 `formal/Origin/VersionedTangencyTests.lean` 与 `formal/lakefile.toml`，并追加本报告。GitHub CI、当前 head 的整票非作者独审及用户 main 合入闸仍待完成，不预写通过。

## X-38 open 变异重推导尾部返修（2026-10-25）

- 尾部代码提交：`58359067830327654cd90705f99e12ec5809cc2a`。
- 独审缺口：此前 open 变异只检查记录字段，未证明该记录驱动业务重推导。
- 修复：在同一个 `#eval` 中以 `p.tangency` 调用 `classifyTermination`，并要求同一三笔相切输入进入 `secondKind`；保留 record id/version 断言与四类 IO 拒绝负控。
- 实证：`cd formal && lake env lean Origin/VersionedTangencyTests.lean` 输出 `TANGENCY_OPEN_MUTATION_OK` 及四类拒绝标记；`lake build` 163 jobs 全过，实际 `Built Origin.VersionedTangencyTests`。
- fixture：`python3 scripts/check_fixture_drift.py` 三组均 0 字段差异、字节一致；`git diff --check` 通过。未改生产定义，未运行本机 cargo。
