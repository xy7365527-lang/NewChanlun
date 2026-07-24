# V3 NestLifecycleBook 下落核查（issue #229，wayfinder map #221 子票）

- 日期：2026-07-24
- 工位：AFK research subagent（只读核查；唯一写入 = 本报告）
- 触发：map #59 Decisions 登记「V3 活假设状态机切片——NestLifecycleBook 三态+first_provable 钟落地；实装落地 1785/0 绿」，#223 清点发现 worktree 工作区 `NestLifecycleBook` grep 零命中。
- 纪律：零 git mutation（仅 log/reflog/status/stash-show 只读）；rust 源码只读；主仓只读。

## 0. 定案（三判之一）

**丢失（lost）——实装从未进入 git 历史，仅以 untracked 文件存在于 worktree 脏树，0721 深夜至 0724 之间被非 git 操作删除，全库无副本、无 stash、不可从 git 恢复。**

- 非「脱节」：#59 登记内容在当时为真——验收报告、勘误、code-review 三方互证实装确曾落盘并全绿（§1）。
- 非「改名」：全特征符号零命中 + 同族符号语义排查排除（§3）。

## 1. 核查路径 A：验收报告对拍【确证】

报告 `chanlun/review-results/v3-lifecycle-statemachine-impl-20260720.md` 登记的改动面与现工作区逐件对拍：

| 登记物（0720 报告） | 现工作区 | 对拍 |
|---|---|---|
| `rust/src/theta_v0/classifier/nest_lifecycle.rs`（新建 1283 行，T1–T8） | 不存在（classifier/ 仅 nest.rs、nest_index.rs） | **不在** |
| `mod.rs` +4 行注册（`pub mod nest_lifecycle;`，原 mod.rs:76-79） | mod.rs:75/77 仅 `pub mod nest;`/`pub mod nest_index;`，无注册行 | **不在** |
| 测试基线 1770 passed/0 failed（报告 §3 输出全文） | 无法复跑（文件已失） | 不可复核 |
| #43 重审材料 `escalate/v3-lifecycle-r43-review-request-20260720.md` | 在 | 在 |

后续扩证（0721）：`erratum-v3-lifecycle-impl-20260721.md` 明载「证据已写入 `nest_lifecycle.rs:117-122` 模块头」、T11 `t11_real_provider_pan_live_force_overtake_auditable` 已实证——**证明文件 0721 仍存在且被 #64 工位续作**（规模超出 0720 版 T1–T8，与 #59「1785/0」而非报告「1770/0」一致）。`code-review-v1-v3-64-20260721.md` 同判互证。

## 2. 核查路径 B：git 历史【确证】

- `git log --all --oneline -S NestLifecycleBook`：**零命中**（worktree 与主仓同对象库，双侧各跑一次均零）。
- `git log --all --oneline -- '**/nest_lifecycle*'`：**零命中**（含 --diff-filter=D 复查）。
- ⟹ **实装从未被 commit 进任何分支**，一直是 untracked 脏树文件。
- `git stash list` 9 个 stash 逐个 `stash show --name-only` grep：**无一含 nest_lifecycle**。
- `git reflog`（本 worktree HEAD）仅 4 条，最后一条 = 0721 18:13 `ec728bf6cf` commit；**0721 后零 git 操作**。untracked 文件的 `rm`/`git clean` 本就不留 reflog 痕迹——删除方式无法从 git 侧区分，但可排除 checkout/reset 所致。
- 其他 worktree `/tmp/kimi-nest-c3`、`/tmp/kimi-nest-snapshot.AZ9J6Y` find：**无副本**。
- 主仓 `/Users/silencehan/Projects/NewChanlun/rust/src` grep NestLifecycleBook/first_provable：**零命中**；主仓 classifier/ 无此文件。

## 3. 核查路径 C：同族符号（改名假说）【确证：排除】

- rust/src grep `NestLifecycleBook|first_provable|LifecycleBook`、`活假设`：**全部零命中**。
- rust/src 内 `lifecycle`（不分大小写）命中 10 文件，逐一抽查语义：全部为既有 `CpLifecycleStatus`/`advance_cp_lifecycles`（recursive_tower.rs CpScanOwnership 机制，恰是 0720 报告 §2① 登记的范式来源）——**与 NestLifecycleBook 无符号承接关系，非改名**。
- 新增文件 `nest_index.rs`（mtime 0724 07:19）= #92/#93 证书索引（NestEventIdentity 主键），头部通读无 LifecycleBook/first_provable/活假设任何符号——**非改名载体**。

## 4. 消失窗口与旁证

- 窗口：**0721（erratum 写作，文件在 :117-122 有续作）→ 0724（#223 清点零命中）**。【确证：两端点；推断：窗口内某时点】
- 删除者/精确时点：**无 git 记录可归因（untracked 删除不产生任何 git 痕迹）**。【未核实】
- 旁证：worktree `git status` 现挂一大批 0718–0719 日期戳 chanlun/.chanlun 文件的未提交 `D`（删除）——工作区确实经历过清理动作；tracked 文件删除留痕，nest_lifecycle.rs 属 untracked 故无痕。nest.rs mtime = 0724 11:14（有未提交 M），说明 0724 上午工作区仍有工位活动。【确证：现象；未核实：与本案的因果】

## 5. #43 材料落点核对（顺带项）【确证】

map #59「#43 重审材料落 escalate」属实，两件俱在：

- `chanlun/escalate/v3-lifecycle-r43-review-request-20260720.md`：重审申请+证据包（四问备料，不代裁）。
- `chanlun/escalate/r43-lifecycle-ruling-20260721.md`：**裁定 #64 SUPERSEDE**（编排者拍板：061:26 活假设语义合法，#43 排除条款被证伪；§2 四问裁定 (a) Provisional 进谱系/Consume 仍 Closed-only 准、(b) Invalidated 证据保留入审计域准；§5 含实装授权范围）。裁定书为纯文档，不随代码丢失失效。

## 6. 处置输入（事实依据，不拍板）

**接线**：无可接线对象——代码实体已不存在，无任何可恢复源（git/stash/副本/主仓均无）。

**重建**（输入齐全，可行）：
- 规格：`chanlun/review-results/v3-lifecycle-statemachine-implementation-card-20260720.md`（完整实装卡，§0–§9）。
- 实装蓝图：`v3-lifecycle-statemachine-impl-20260720.md`（逐符号行号锚点、范式复用点、T1–T8 逐条断言明细）。
- 必带勘误：`erratum-v3-lifecycle-impl-20260721.md`——trend 域真实 ForceOvertake 不可达，真实反超只落 pan 活窗通道；重建时 T1 只能作合成测试，T11 须按 pan 通道重写（原 T11 代码随文件同失）。
- 评审约束：`code-review-v1-v3-64-20260721.md` Spec 轴 (a)1（T1 合成喂入绕过真实验证之判）。
- 裁定边界：`r43-lifecycle-ruling-20260721.md` §5（实装授权范围）与维持有效条款（nest.rs:802-810/:987-989、judge_at/divergence_confirmed 口径不动）。
- 规模参照：0720 版 1283 行/T1–T8/1770 绿；0721 续作后对应 #59 登记 1785 绿（含 T11 与模块头勘误段）。

**归档**：若选择不重建，须同步修订 map #59 Decisions 条目（「实装落地 1785/0 绿」与现状不符）并保留本报告 + 勘误 + 裁定链作档案。

## 7. 未核实项

1. 删除者与精确删除时点不可归因（untracked 删除无 git 痕迹；reflog 0721 后零操作）。
2. nest.rs 0724 11:14 未提交改动的内容与本票无关，未深查。
3. macOS Time Machine/Spotlight 快照、编辑器热退出缓存等 OS 级副本未查（出本工作面）。
4. #59 登记「1785/0 绿」的 1785 基线构成（相对 0720 报告 1770 的 +15）无完整测试清单留存，仅从 erratum 推知含 T11 等续作。
