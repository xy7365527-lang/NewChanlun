# #551 实装收尾 · Standards 轴评审

- 工作面：`/tmp/wt-551`（`ticket-551`），fixed point `20a14b9dd9` → `ab6dcd4ab0`（6 枚 commit）
- 标准源：`.claude/rules/common/*` 全部 + Fowler 12 条基线
- 性质：只读评审，未改动任何代码

## 结论

**PASS**（无 HIGH）。3 枚 MED + 4 枚 LOW，均为可后续收口项，不阻断合入。

## (a) 违反仓内文档标准

| # | 级别 | 位置 | 标准原文 | 事实 |
|---|---|---|---|---|
| 1 | MED | `rust/src/theta_v0/classifier/mod.rs:4134-4655`；`cand_event.rs` 整体 | coding-style.md《File Organization》「MANY SMALL FILES > FEW LARGE FILES … 200-400 lines typical, **800 max**」 | 增量方向与标准相反：`mod.rs` 4766→5223（`mod tests` 本轮 +430，现 2429 行），`cand_event.rs` 749→**1120**（本轮越过 800 线）。新增的 #551 生命史锁是内聚一族，可落 `rust/tests/` 或 classifier 子模块而非继续堆进两个超限文件 |
| 2 | MED | `cand_event.rs:556` `merge_episode_leg` | coding-style.md《Immutability (CRITICAL)》「ALWAYS create new objects, NEVER mutate existing ones」 | 就地改写域对象：先 `*acc = leg` 整体覆盖，再回补 `extreme`/`state`/`first_provable_at` 三字段（覆盖与回补的先后顺序本身承载语义）。**不在两条例外注记适用域**——既非回放驱动性能缓存（2026-07-27 裁定），也非带修订留痕的账本状态机（2026-07-28 裁定）。严格形式 = `fn merged(&CandidateObservation, &CandidateObservation) -> CandidateObservation` |
| 3 | MED | `mod.rs:4626` 函数名 + `mod.rs:4650` 断言消息 | coding-style.md《Checklist》「Code is readable and **well-named**」；no-patch-mentality.md 禁项 5「声明膨胀：在注释/文档中声明代码不具备的能力」的镜像 | 名 `…_forks_in_forbidden_direction` 与消息「禁止方向的分叉（**矛盾已上浮**）」在甲口径裁决后与事实相反（该方向已不禁止、矛盾已裁决），靠 doc 注释反向纠正。留名理由（测试名册差集对账）是流程约束，严格形式 = 改名 + 在名册对账里登记这次改名 |
| 4 | LOW | `rust/src/bin/issue550_event_battery.rs:31-38` | coding-style.md《Error Handling》「**Never silently swallow errors**」 | `timestamp()` 以 `.parse().unwrap_or(0)` 兜底，日期字段损坏时静默产出 timestamp=0，电池计数无感 |

## (b) Fowler 基线臭味

| # | 级别 | 臭味 | hunk |
|---|---|---|---|
| 5 | MED | **Duplicated Code + Shotgun Surgery** | 「业务载荷投影」逐字段同义地定义了两份：`mod.rs:4339 payload_eq`（10 字段合取）与 `issue550_event_battery.rs:290 type Projection` + `:303 projection_of`（同 10 字段元组）。`CandidateEvent` 加字段须两处同改；漏改任一处，两侧都继续静默绿 |
| 6 | MED | **Primitive Obsession / Data Clumps** | 同 #5 的 `Projection`：10 元组无名字段，且把已 `derive(PartialEq)` 的 `StructuralPredicates` 摊平成 `(bool, bool, bool)` |
| 7 | LOW | **Duplicated Code** | 「按 key 折最新 revision」的双层 for 折叠出现 4 处：`mod.rs:4138 latest_by_key`、bin 的 `print_summary` / `print_lifecycle_summary` / `print_projection_fork` 各一份 |
| 8 | LOW | **Speculative Generality** | `bin:206 print_projection_fork -> Result<(), String>` 函数体内无任何 `?` 或 `Err` 路径，恒 `Ok`；`cand_event.rs:398 on_invalidate(prior: &CandidateEvent, …)` 收下 `prior` 后立即 `let _ = prior;` 丢弃 |
| 9 | LOW | 格式 | `mod.rs:4160-4161` `candidate_rich_layer` 之后连续两个空行 |

判断为**无新增实例**：Mysterious Name（除 #3）、Feature Envy、Repeated Switches、Divergent Change、Message Chains、Middle Man（`judge_first_cached` 委托 `first_structural_gates` 后解包 4 个分量属正常适配）、Refused Bequest。

## 已核查但不构成发现

- **测试须本地通过**（testing.md）：`cargo test --lib theta_v0::classifier` = 474 passed / 1 failed / 6 ignored。唯一红 `signal::tests::extract_signals_bit_exact_digest_guard` 已在 `20a14b9dd9` 独立 worktree 复现，left/right 与 HEAD **逐位相同**（16618955402698307653 vs 10432481772907336594）⟹ 基线红，非本 diff 引入。commit message 中「基线即红」的声明属实。
- **fmt / clippy**：仓库全域预存 513 warning + 2 error（`strategy/nest.rs:270`、`backtest/wverify_run/tests.rs:477`），均在本 diff 之外；本 diff 新增区未新增 clippy 项。全域 rustfmt 未被强制（数百处预存差异），按「工具已强制的跳过」处理。
- **security.md**：无硬编码密钥；bin 的下标访问由 `n = min(各 vec 长度, limit)` 兜住，`bars.is_empty()` 在 `run()` 内前置。
- **git-workflow.md**：6 枚 commit 均为 `<type>(scope): <description>` + 正文，无 attribution 尾巴。
- **no-workaround.md / no-patch-mentality.md**：塌空—回长分叉未被抹平而是钉成测试并上浮，符合规则；甲口径落地后的诚实声明（真空分支、合成夹具恒 0、量度归属真实数据电池）在四枚锁的 doc 中逐条给出。

## 影响声明

本产出为只读评审记录，未改动任何源码或测试；新增文件仅本报告。
