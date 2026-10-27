# PR1681 接续 PR1693 集成独审

结论 PASS。无 CRITICAL 或 HIGH，概念层无矛盾。冻结 head `4260f24ef01b9177f99dfb028c0a794d6fcf8ad8` 可进入 CI 终态门。下面三处 LOW 不阻塞合入，但建议在本票或后续票清掉。

审查者为 Claude 家族子代理 `/root/review1681_after1693_26`，非作者。作者为 GPT/codex 家族。工位 `/Volumes/AgentStorage/issue1681-native-repair-20261026` 只读，`git rev-parse HEAD` 实测等于上述 head。原 head `5351fc021f0f86f391f025f0d6a6ef78bb26dcc9`，合入 main `df7beb0c8da5d8f3b6053f85813b86613dcc6849`，merge-base `40f11b540f02666a854bcf48e5ee259ff88af8bb`。审查时刻取 `date` 为 2026-10-26 13:04:51 CST。

## 审查范围如何收窄

相对两个父提交都不同的文件只有三个。`rust/src/theta_v0/config.rs`、`policy_records.rs`、`run_records.rs`。其余 main 继承文件与 main 逐字相同，#1602 侧其余文件与原 head 逐字相同，沿用旧审，不重审。

`policy_records.rs` 用 `-U0` 增删行对拍。merge-base→main 的增删行与 5351→head 完全相同，merge-base→5351 的增删行与 main→head 完全相同。这说明自动合并是两侧 hunk 的纯并集，没有丢行或改写。

## 唯一文本冲突

`config.rs:233-239` 的 `LevelConfig::from_active_record` 取 main 的 Result 路径，同时读 `active_tower_level_budget()?` 与 `active_tower_base()?`，经 `try_level_config_from` 对 `kline3/trade3` 返回 typed `PolicyError::Invalid`（`policy_records.rs:569-588`）。解法正确。原 #1602 侧 `Ok(Self::from_budget(...))` 会经 `level_config_from` 的 `unwrap_or_else(panic!)`（`policy_records.rs:591-594`），已被替换。

## 生产消费链

`collect_run_records`（`run_records.rs:416-464`）新增两步。`active_tower_base()` 错误记入 `tower_base` 槽，预算与塔底都在时再以 `try_level_config_from` 组装，失败同样记入 `tower_base` 槽，`RunPolicyError::take`（`run_records.rs:87-89`）只追加不回退。七元组任一为 None 即返回汇总错误，没有备用值。`ThetaConfig::from_records`（`config.rs:760-764`）改为直接消费 `set.level`，不再调会 panic 的兼容构造。`for_production_symbol`、`for_production_profile`、`diagnostic_fixture` 都走这条链，生产入口因此保持原合同：缺口一次汇总为 `RunPolicyError`，不 panic，不回退常数。

下游 `classify_incremental`（`classifier/pipeline.rs:627-637`，main 原样）对非 `Segment` 的 `tower_base` 在任何 L0 构造前 assert 停止。研究工具就地把值改成 `Kline3` 也不会被线段输入冒充，`classifier/tests/tower_policy.rs:107-113` 锁定该停止。

## 各域无 fallback

资金与场所三参数、Python 转发相关代码相对原 head 零差异，沿用 `1681-MAIN-INDEPENDENT-REVIEW26C.md` 及消费订正。trend_scope 的解析、profile 预注册拒绝与消费点相对 main 零差异，`parse_trend_scope_for_profile` 对跨 profile 赋值、缺字段、非法值都停止。tower_base 只在 `try_level_config_from` 一处准入，`read_str`、`read_u32`、`read_bool`、`read_f64_array` 的未签判 `AwaitingPolicy` 语义两侧均未改。没有出现宽严两档接在同一 if-else 上的结构，不触 #799 收敛通则。

## record_refs 标签与 signed 负控

`config.rs:821-834` 在 `tower-level-budget` 之后插入 tower-base 引用。值等于当前生效记录的值时保留签署版本，否则标 `research-override`。`admissible()`（`config.rs:847-852`）要求全部为 `Signed`，于是覆写即不可准入。全仓按位置解析 `record_refs()` 的消费者只有 `ffi.rs:117` 与 `bin/theta_replay.rs:175`，两者都只映射成标签列表，插入不破坏它们。全仓硬编码完整标签串的只有 `config.rs:884` 与 `:1024`，都已加上 `tower-base/1`，没有删去原有标签。

负控 `config.rs:1022-1030` 有效。先以精确标签串和 `admissible()==true` 证明签署配置可准入，再把值改成 `Trade3`，断言出现 `tower-base/research-override` 且 `admissible()==false`。把 stamp 去掉，或改成比较 PolicyRef 而非值，两条断言都会变红。`:1046-1047` 在诊断夹具上再锁一次 `Kline3`。

比较对象是当前生效记录而非来源记录。今天两者等价，因为 `try_level_config_from` 只放行 `Segment`，`from_records` 也只用生效记录。将来若生效记录换成另一个有真实构件的值，需要回头改成对来源记录比较。

## R14 账目未被迁就

`git diff --stat 047676ae25..4260f24ef0` 对 `audit_owner.rs`、`cross_request_source_cache.rs`、`s_structure_session.rs` 为空，R14 测试和 DDL 自上次窄独审以来零差异。最终日志 `1681-after1693-validation26/20261026-122715-4260f24ef0.log` 首行绑定完整 head，原样输出 sealed=399158、node_source_claims=803、claims_bytes=111361、layout_drop=16640、schema_growth=57、sealed_before_p2a=527159，1 passed、0 failed、1037 filtered out，EXIT=0。账目与 047676 时逐项相同，`527159-399158=128001=111361+16640` 仍成立。新增标签进入 `structure_deltas.policy_records` 列，但不进入封存普查，实测数字未动。

## 工程层发现

| 严重级别 | 数量 | 状态 |
|---|---|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 3 | note |

**L1 死入口仍带 panic。** `config.rs:241-244` 的 `LevelConfig::from_budget` 在 head 全仓零调用，唯一旧调用点已改成 `set.level`。它经 `level_config_from` 在塔底非 `Segment` 时 panic，是一条绕开 typed 路径的公开旁路。建议删除，而不是以保留旧入口为由留下。

**L2 一次收齐有一个缺口。** `run_records.rs:430-435` 只在预算可解析时才检查塔底是否有构件。反例为 `tower-level-budget.l_max` 写 `AwaitingPolicy` 且 `tower-base` 写 `kline3`。首次只报 `[tower_level_budget]`，补签预算后第二次才报 `[tower_base]`。两份记录都是 `include_str!` 编入的常量，只在改代码时出现，影响限于报错轮数。修法是把“塔底有无构件”从 `try_level_config_from` 拆成不依赖预算的独立检查。

**L3 新接线的 typed 拒绝没有行为锁。** 生效塔底记录不可注入，`run_records` 测试（`run_records.rs:682-694`）只覆盖 `segment` 成功路径。把 `level` 改回 `Some(LevelConfig::from_budget(...))` 这种会 panic 的写法，现有测试仍全绿。语义审已确认当前代码正确，typed 拒绝本身由 `policy_records` 的 `tower_base_record_drives_config_and_rejects_unsupported_base` 锁住，组合层缺锁不阻塞。补锁需要给 `collect_run_records` 一个塔底文本注入点，与 R14 无关。

另记 `RunRecordSet.tower`（`run_records.rs:409`）在生产中已无读者，只有测试读取，与 `level.budget` 重复。不计入发现，删留由作者决定。

## 验证复用与剩余 CI

复用作者三份远端日志，不重跑。`20261026-122321-fd7da32c7d.log` 为合并提交上的 config 10、run_records 8、policy_records 12、ffi 3、tower_base 4、资金与 trend_scope 组合 1 及 backtest_bin check，EXIT=0。`20261026-122551-4260f24ef0.log` 为最终 head 上的 rustfmt check 与 config 10，EXIT=0。`fd7da32c7d..4260f24ef0` 只有格式 patch 和 config 测试尾部，前一份日志对未改生产代码继续有效。本审未运行任何 cargo 或 rustc。

CI run `40269311535` 绑定 head `4260f24ef01b9177f99dfb028c0a794d6fcf8ad8`。13:04:51 CST 回读时无失败作业，`test`、`rust-check`、`rust-test-default`、`rust-test-backtest-bin`、s-session shard 2/3 与 extras content/economic、`fixture-drift` 等已 success。仍未完成的是 `rust-test-s-session` shard 0 与 shard 1、`rust-test-s-session-extras (native)` 运行中，`rust-test-s-session-extras (owned)` 排队。本审 PASS 不替代这四项终态，合入仍需整次 run `completed/success`。

本审不证明全图 Destination 完成，不合 main，不关 #1602。

## 原则对选择的影响

Prove It Works 使 R14 结论取自绑定 head 的原始日志加三文件零差异，而不是作者报告的转述。Test Behavior, Not Implementation 使负控按“去掉 stamp 是否变红”来判有效，并据同一检验指出 L3 的组合层缺锁。Boundary Discipline 使塔底准入只认 `try_level_config_from` 一处，L1 的 panic 旁路因此被点名。
