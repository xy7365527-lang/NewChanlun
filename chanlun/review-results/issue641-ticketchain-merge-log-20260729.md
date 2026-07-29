# ticket 链（550→553→641b）合流 main —— merge log

日期：2026-07-29　工位：实装执行层（Opus，前台单线程，无子代理派发）
勘察依据：`/tmp/ticketchain-merge-survey-20260729.md`（纯只读勘察）
执行方式：侧 worktree（`/tmp/wt-merge-n3`，detached）做 merge + 全套验收，主仓事后 `--ff-only` 快进。

## 0. 定性（承勘察 §0）

本次不是"链上母本 main 零基座"的首次合流：

- **N1+N2 八票链早已并入 main**（`a559d2edba`，第二父 `654ee7a9c3` 即本次 merge-base）；
- **N3（#641）main 上已有一份手工移植**（`785732b3ba` → `be69a870db` → `0de84f4b09`），
  移植时间点约当 ticket-641b 的 `d5e55925a6`，**遗漏最后一轮 #653 影子评审收口**。

故本次实质 = **补 #653 收口轮增量**。改动面最终仅 6 文件、生产代码 3 处（其中 2 处在
`#[cfg(test)]` / 诊断 bin 内），全部字节门与产物对拍读数见 §3。

**090 教训（承勘察 §7-1）**：手工移植曾漏一轮，靠 diff 反查才发现。N4-N7 建议直接
`git merge` / `cherry-pick` 带 hash 溯源，不要手工摘抄改写。

## 1. 关键 hash

| 项 | hash |
|---|---|
| merge-base(`main` × `ticket-641b`) | `654ee7a9c3df8c41db7c6f1fd75ede5f3e681f89` |
| CONTEXT.md 词表收编 commit（主仓单独提交） | `b0613f1a352b0496880523f59510c1d739e01601` |
| merge commit（第一父 = main） | `37b34be5175ba2c91c91c581497d6b6de4e7c281` |
| 第一父（合流前 main = 上面的 CONTEXT commit） | `b0613f1a352b0496880523f59510c1d739e01601` |
| 第二父（`ticket-641b` HEAD） | `8b737f2a8c409fbd094112df2dc40109bff5f9d6` |

CONTEXT.md 那条改动是**另一并发 session 手写、留在主仓工作区未提交**的"级别链证书
（tower chain certificate）"词条。核对其已覆盖 ticket-641b 报告 §9.4 登记 8 点名的两处
（终态三态句补"≥1 有效链段"地板合取；`extends_lineage_key` 加注"查簿命中才写"），
故按纪律**只 add CONTEXT.md 一个文件**单独提交，主仓其余 89 项在飞改动一个不碰。

## 2. 冲突 6 文件逐条解法

| 文件 | 类型 | 解法 | 依据 |
|---|---|---|---|
| `chanlun/review-results/issue641-n3-chain-impl-20260729.md` | add/add | **两份并存不硬合并**：main 侧留原名（429 行），ticket 侧另存 `issue641-n3-chain-impl-ticket641b-20260729.md`（468 行） | 同一票两工位视角，均有史料价值；硬合并反而混淆工位 |
| `rust/src/bin/issue550_event_battery.rs` | 内容（2 块，5 行） | **取 ticket 侧**：`digest` 与 `summarize` 改在同一簿状态点取值（`advance` 之前捕获） | 修真实非幂等窗口（#653 影子评审 LOW-1）；main 侧无此修复 |
| `rust/src/theta_v0/classifier/chain_cert/mod.rs` | add/add（1170/1170，仅 1 行） | **取 ticket 侧**：`#[cfg(test)]` 探针 `floor_blocked` 触发条件补 `&& all_segments` | 仅测试诊断计数；生产判据（`all_segments` 定义与使用，591/601 行）两侧逐字相同，零生产行为变更 |
| `rust/src/theta_v0/classifier/chain_cert/tests.rs` | add/add（纯末尾追加 67 行） | **取并集**（= 取 ticket 侧） | 新增 `breach_reason_matches_interval_is_sub_conjuncts` 一致性锁（#653 影子评审 LOW-2），纯增量零删除 |
| `rust/src/theta_v0/classifier/mod.rs` | 内容（1 处冲突块） | **冲突块取 main 侧**；`chain_cert` 那行注释采用 ticket 侧更完整版本（纯 cosmetic） | ticket 侧多出的 `pub mod first_retrace_replay;` 是**陷阱**——main 已在 `8b8905def2`（#624 裁定 A）连文件下葬、内容迁入 `retrace_ledger`，拉回即 E0583。实际净 diff = +2/-1 行注释，模块声明未变 |
| `rust/src/bin/p123_fast_replay.rs` | 内容（3 块） | **整文件取 main 侧** | main 已用 `TargetedPassState` + `process_targeted_bar`/`finalize_targeted_pass` 独立重构等效实现同一功能。取后该文件相对 main **零改动**（`git diff HEAD --stat` 空），p123 字节门天然保号 |

`signal.rs`（#491 digest guard）/ `issue533` golden 系列：ticket-641b 从未触碰，**零冲突**，
main 现状直接延续。

**merge 相对合流前 main 的净改动面**（`git diff HEAD^ HEAD --stat`）：

```
 chanlun/review-results/issue641-n3-chain-impl-ticket641b-20260729.md | 468 +++++
 chanlun/review-results/shadow-641-review-20260729.md                 | 321 +++++
 rust/src/bin/issue550_event_battery.rs                               |   5 +-
 rust/src/theta_v0/classifier/chain_cert/mod.rs                       |   2 +-
 rust/src/theta_v0/classifier/chain_cert/tests.rs                     |  67 +++
 rust/src/theta_v0/classifier/mod.rs                                  |   3 +-
 6 files changed, 863 insertions(+), 3 deletions(-)
```

`formal/` 与 `rust/tests/fixtures/` **未触及**（`git diff HEAD^ HEAD -- formal rust/tests/fixtures` 为空）。

## 3. 验收门读数（全部在侧 worktree `/tmp/wt-merge-n3` 实跑，非估算）

### 3.1 单测

| 门 | 读数 | 判定 |
|---|---|---|
| `cargo test --lib`（debug，`CARGO_TARGET_DIR=/tmp/kimi-nest-target-mrgn3`） | **2513 passed / 0 failed / 138 ignored** | ✓ |
| `cargo test --release --lib`（合流后） | **2511 passed / 0 failed / 138 ignored** | ✓ |
| `cargo test --release --lib`（合流前基线 `b0613f1a35` 净树） | **2510 passed / 0 failed / 138 ignored** | 对照：+1（release cfg 下新增一致性锁） |
| `cargo test --lib chain_cert` | **24 passed / 0 failed** | ✓ |
| N3 golden/digest 点名（`digest_guard` / `fnv1a_golden` / `chain_certificate`） | **4 passed / 0 failed**，含 `chain_certificate_book_classify_fnv1a_golden`、`chain_certificate_book_incremental_equals_full_replay`、`summarize_and_digest_are_book_internal_readouts` | ✓ FNV golden 锁绿 |
| `#491` `extract_signals_bit_exact_digest_guard` | **ok** | ✓ ticket 分支上这条红是"基座早于 #491 重锚"的陈旧假象，merge 后随 main 侧 `signal.rs`/golden 胜出自动转绿，与勘察 §4 预测一致 |

### 3.2 #533 p123 字节门（release，`-- --ignored`）

- 首跑报 `P533 ENVIRONMENT`（缺 `analysis/data_cache/btc_1m_full.json`，被 `.gitignore:82` 排除、
  不随 worktree checkout 到位）。已向侧 worktree 与基线树**软链**主仓数据缓存后重跑（只读软链，
  未改主仓任何文件）。
- 重跑读数：**`p123_replay_2000bars_byte_exact` ok；`p123_replay_large_windows_hash_guardrail`
  FAILED**（1 passed / 1 failed）。
- 失败面：**仅 `max_bars=100000` 的 P116_DUMP 全文对拍**；20k 档 P116 全文 + 20k/100k 的
  stdout/dump SHA-256 全部通过（断言顺序 20k 在前，已全过）。
- **定性：先存红，非本次合流引入** —— 在合流前 main 净树（`/tmp/wtmrgn3-base`，
  `b0613f1a35`，独立 target dir）跑同一门，**同一测试同一断言同样 FAILED**（1 passed / 1 failed）。
- 差异面：实跑相对仓内 golden **多 4 行**（`TERM level=2 bar=92905 side=Short`、
  `TERM level=2 bar=99207 side=Long` 及对应两条 `CERT caliber=A/B ... ids=2:92905:82483-92905`）。
  该 golden 末次重锚为 `6381debba9`（2026-07-28，#619 条件收口）——即 main 上此后某次改动
  新增了 level=2 事件而未按 golden 变更纪律重锚。**本票不动 golden**（票面纪律：禁为变绿静默重生成），
  作为遗留上报（§5）。

### 3.3 p123 20k/100k 三面 merge 前后对拍（本票真正该验的门）

二进制：`/tmp/kimi-nest-target-mrgn3{,-base}/release/p123_fast_replay`；产物 `/tmp/wtmrgn3-{pre,post}-<窗>-<面>.txt`。

| 窗 | stdout | lifecycle dump | P116_DUMP |
|---|---|---|---|
| 20000 | **cmp=0** (1809B) | **cmp=0** (5,675,931B) | **cmp=0** (8840B) |
| 100000 | **cmp=0** (1632B) | **cmp=0** (27,167,860B) | **cmp=0** (21,930B) |

六面全 cmp=0 —— 与"p123_fast_replay.rs 整文件取 main 侧 ⟹ 相对 main 零改动"一致。

### 3.4 m8 三窗产物对拍（`M8_WIN_FILTER` × `OPSEM_DUMP_DIR`）

跑法：`OPSEM_DUMP_DIR=<dir> M8_WIN_FILTER=<tag> cargo test --release --lib
theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`
（产物 `/tmp/wtmrgn3-m8-{pre,post}-<窗>/`）。

| 窗 | trades.jsonl | tower_events.jsonl | center_lifecycle.jsonl |
|---|---|---|---|
| p3fold | **cmp=0** (1,332,996B) | **cmp=0** (330,026B) | **cmp=0** (510,454B) |
| wf7 | **cmp=0** (1,546,415B) | **cmp=0** (370,288B) | **cmp=0** (508,583B) |
| wf8 | **cmp=0** (1,275,806B) | **cmp=0** (354,946B) | **cmp=0** (532,466B) |

三窗 execR 读数两侧亦逐位相同（p3fold `-15558715` / wf7 `-22020957` / wf8 `-25896625`，
`stage=I` `R≤0`）。证实 `chain_cert` 的"纯产出零消费接线"承诺在数据面上成立。

### 3.5 fixture 漂移门

`python3 scripts/check_fixture_drift.py`（侧 worktree，`.lake` 用主仓缓存的**副本**跑，
避免向主仓 `.lake` 并发写入、也避开主仓 untracked 的 `formal/Research/`）：

- `[center] theta_v0_center_parity.json ← Origin/CenterConstruct.lean`：**✓ 无漂移**
  （0 字段差异，字节级一致）。
- `[parity] theta_v0_parity.json ← Origin/ParityFixtureExport.lean`：导出器运行失败，
  脚本 **exit=5**（按 CLAUDE.md 分域 = 工具链/导出器类，**非漂移**）。
- 真实错因：`Origin/ParityFixtureExport.lean:104:28 error: failed to synthesize instance
  Decidable (Overlaps a b)` + `:217:0 depends on the 'sorry' axiom`。
- **定性：main 先存，与合流无关** —— 主仓原生 lake 环境（`formal/` tree 与合流树逐位相同，
  `28f532a4237faba202f624eae119c990888a0c0d`）跑同一 `lake env lean` **复现同一错误、同一 exit=1**。
  本票不触及 `formal/`，作为遗留上报（§5）。

## 4. 封印保全核验（#553 P123_EVENT_DUMP 双跑 SHA / FNV golden 锁）

`git merge-base --is-ancestor <hash> 37b34be517` 逐个核验，**全部为祖先**：

- 从 #553 两份封印文档（`issue553-t4-acceptance-20260728.md`、
  `code-review-issue553-standards-20260728.md`）机械抽取的 7 个 commit hash：
  `2a75254b06` `439105c7e7` `49e1e4b284` `555ee412f2` `756dc87763` `dc0d61052e` `e8496fcaf0`
  —— **7/7 祖先 ✓**
- 链路关键节点 14 个（含 `654ee7a9c3` merge-base、`a559d2edba` N1+N2 merge、
  `4fa468711d`/`46c4c30dd3`/`d5e55925a6`/`b2f7834386`/`8b737f2a8c` ticket 链五提交、
  `785732b3ba`/`be69a870db`/`0de84f4b09` main 手工移植三提交、`8b8905def2` #624 下葬提交）
  —— **14/14 祖先 ✓**

merge（非 rebase）不改写历史，封印证据链引用的 hash 合流后原样可达，无需补救。
N3 `digest()`/FNV golden 锁绿见 §3.1。

## 5. 遗留（照实，两条均为 main 先存红，本票未动）

1. **#533 100k 档 P116_DUMP 相对 golden 漂移 4 行**（§3.2）。合流前基线同样红。
   golden 末次重锚 `6381debba9`（#619，7-28）之后 main 上某次改动新增 `level=2` 的
   TERM/CERT 事件而未按票面纪律重锚 golden 并说明原因。需单独挂票定位那次改动、
   判定"故意的行为变化"后再重锚（禁为变绿静默重生成）。
2. **`Origin/ParityFixtureExport.lean` 编译失败致 fixture 漂移门 parity 面跑不出**（§3.5）。
   主仓原生环境复现。`Decidable (Overlaps a b)` 实例缺失 + `sorry` 公理依赖，
   需 formal 工位单独处理；center 面无漂移。
3. **roster 未追加**：`.chanlun/agent-roster-*.md` 全部是主仓 untracked 的他 session 在飞产物
   （`agent-roster-20260728.md` mtime 今日 14:50，仍活跃），追加即触碰在飞面，
   按派发纪律的例外条款**跳过并在此注明**。
