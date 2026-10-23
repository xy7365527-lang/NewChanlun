# #1488 评审链核对（REVIEW-CHAIN，0085）

对应关票门 G7「评审面 == 关票面」(a)。

- 阶段一核对对象：`6badaa6049..026e3e97b0` 的全部提交（含两次 merge）。`6badaa6049` 是上一次并入的 main；`026e3e97b0` 是 merge2-0083 的合并面。
- 阶段二延伸到 squash 源 `0fb96c00ae`：`026e3e97b0 → f0aeaf2817（0086）→ 327aa94c39（0089）→ 8a1924993b（pre-squash）→ dbfb13d90b（再合 main aa1c3885f8）→ 5417eea488（再合 main 21b145250e）→ 0fb96c00ae（再合 main 66c409e7e3）`，见 §3。

工具是 `workers/close-0085/chain.py`（仓外）。它把每个独审的 `base..head` 展开成提交集，与目标集求差。原始输出在 `workers/close-0085/chain-026e3e97b0.json`。

**结论：`6badaa6049..327aa94c39` 首尾相接全覆盖**（阶段一 55 个缺口提交由 gap-review-0087 五段补齐；0084 → 0088 → 0090 尾部相接）。`327aa94c39` 之后的 pre-squash 提交、三次再合 main 的合并解法（含 `5417eea488` 的告警门控）、P58 全量 fmt 与本目录文档尚无终审结论：负责人所派 squash 面独审 0091 对 `336b53db8d` 预审判 **FAIL**（P1-1 bsp 白名单锚，已在终面修复；P2-1/P2-2/P3 交负责人裁定），终面待 0091 复审（§3.3、§3.4）。

## 0. 总数

| 项 | 数 |
|---|---|
| 目标集 `rev-list 026e3e97b0 ^6badaa6049` | 384 |
| 其中 main 侧（`6badaa6049..6c6acd5fe0`，经 merge 带入，属 main 自身评审范围） | 27 |
| 本票侧 | 357 |
| 本票侧被某个非作者独审区间覆盖 | 302 |
| 阶段一缺口 | 55（四段，见 §2；已由 0087 补审闭合） |

## 1. 各独审区间

| 独审 | 区间 | 区间内提交（∩目标） | 结论 | 说明 |
|---|---|---|---|---|
| 42-INDEPENDENT-COVERAGE-REVIEW | `b46ba29699..5cb7253d46` | 26 | 逐文件 hunk 覆盖 | 产品树 5cb7253d46，42 文件 |
| budget-handoff-independent-review-0011 | `5cb7253d46..813e1f92b3` | 1 | FAIL（3 P1 / 2 P2） | 发现由 0024 起逐条复核 |
| uo-independent-review-0024 | `959d860065..5759bbec6a` | 64 | FAIL | v4 实施首轮。被审 diff 只限 `-- rust/` |
| uo-independent-review-0026 | `5759bbec6a..b6969af4cc` | 22 | FAIL | 返修 |
| uo-independent-review-0028 | `b6969af4cc..4022f12079` | 16 | FAIL | 返修 |
| uo-independent-review-0030 | `4022f12079..62bad42a1b` | 18 | PASS_WITH_FINDINGS | v4 冻结（`codex/1488-unified-ownership-v4-frozen-20261020`） |
| recover-independent-review-0032 | `62bad42a1b..ea363d5070` | 33 | PASS_WITH_FINDINGS | |
| recover-independent-review-0036 | `62bad42a1b..69e2568ea4` | 68 | PASS_WITH_FINDINGS | |
| journal-independent-review-0038 | `69e2568ea4..cc9fe7e1d6` | 13 | PASS_WITH_FINDINGS | |
| recover-independent-review-0040 | `cc9fe7e1d6..280437d999` | 7 | PASS_WITH_FINDINGS | |
| recover-independent-review-0043 | `280437d999..38dd2ce493` | 13 | PASS_WITH_FINDINGS | |
| review-0045 | `62bad42a1b..fff166540e` | 113 | PASS_WITH_FINDINGS | 冻结源整体终审（`codex/1488-recover-journal-frozen-20261020`） |
| merge-review-0052 | merge `57f9680c4e` | 1 | RESOLUTION_APPROVED_MERGE_BLOCKED | 冲突解法通过。阻断项 r14 由 0054 解除 |
| r14-review-0054 | `57f9680c4e..c2bdf9b662` | 5 | APPROVED_WITH_FINDINGS | |
| review-0060 | `bba547492d..df0b98f0f7` | 3 | PASS_WITH_CONDITIONS | 只看 `rust/` |
| review-0068 | `727b3bfb45..78523300ad` | 17 | PASS_WITH_FINDINGS | degrade-0065 |
| review-0070 | `44d312d7cf..5aff2d1b41` | 2 | FAIL（返修见 0072） | fix-0069 |
| review-0073 | `5aff2d1b41..3ee05a2713` | 1 | PASS | |
| review-0082 | `3ee05a2713..26391ae34f` | 12 | PASS | p3fix-0077、p24adj-0079 |
| merge2-review-0084 | merge `026e3e97b0` | 1 | CONDITIONAL（无 P1；P2-1/P2-2/P3-1） | 合并面冲突解法 |
| gap-review-0087 ① | `813e1f92b3..959d860065`（对 026e3e97b0 存活 hunk） | 1 | CONDITIONAL（P2 ×2） | ①-F-1 由 0089 修复；①-F-2 由 #1612 R2 切片承接 |
| gap-review-0087 ② | `c2bdf9b662..bba547492d`（S3） | 30 | PASS | |
| gap-review-0087 ③ | `df0b98f0f7..727b3bfb45`（第 6–16 处裁改断言） | 5 | PASS | ③-P3-4 B11 不可达证明经负责人追认 |
| gap-review-0087 ④ | `78523300ad..44d312d7cf`（five-0067 + merge） | 19 | PASS | |
| gap-review-0087 ⑤ | `c2bdf9b662..809710da97`（accept-s4，S4 锁） | 52 | CONDITIONAL（P2 ×2） | ⑤-P2-1 由 0089 补跨臂负控；⑤-P2-2 负责人认可「可达闭包逐位」口径 |
| premerge-review-0088 | `026e3e97b0..f0aeaf2817`（premerge-0086：并入 S4、再合 main 02b5b571eb、清零自带红） | 5（rev-list 共 63，余为 merge 带入的 accept-s4 52 提交与 main 侧提交） | CONDITIONAL（唯一 P1-1 已由 0089 `ffb89f7fa8` 关闭；`ratification_18_23.ok=true`） | remerge diff 0 行；#1597 额外替换 0 处 |
| fix-review-0090 | `f0aeaf2817..327aa94c39`（fix-0089） | 6 | CONDITIONAL（无 P1；P2-1 只改论证） | 生产可达性更正见 CLOSE-DECLARATION §4 |

区间之间有重叠（0032、0036、0045 都从 `62bad42a1b` 起），不影响覆盖结论。

## 2. 阶段一缺口（已闭合）

阶段一核出的四段缺口与 S4 段，已由 gap-review-0087 逐段补审（§1 表中 0087 ①–⑤）。下表保留原核对记录。

| # | 缺口区间 | 提交数 | 规模 | 内容 | 现有间接证据 | 建议补审范围 |
|---|---|---|---|---|---|---|
| ① | `813e1f92b3..959d860065` | 1（`959d860065` WIP checkpoint） | 31 文件 +1821/−961 | 预算所有权重构的未完成检查点。它是 0024 的 base，所以不在任何被审 diff 里 | 0024 只审 `959d..5759 -- rust/`。0011 审到 `813e`（FAIL）。NONPASS-QUOTA-RECEIPT 记它为 head。0067 `368e2bcb13` 修了它引入的逐插入×深度重复计费，说明此段确有产品缺陷漏过 | `git diff 813e1f92b3 959d860065`。可只审合并面上仍存活的 hunk：`git diff 813e1f92b3 959d860065` 与终面逐 hunk 对照 |
| ② | `c2bdf9b662..bba547492d`（S3） | 30（含 N1–N5 负控提交与 revert） | 6 文件 +652/−23 | S3 产品改动：`e94be462c2` fractal/bi 后缀一次 rebuild_suffix；`d4686c7a09` aux plan 未变行跳过、supersedes 只扫 Δ_R。另有 S3 锁和锁表 | 只有 CI 39384343438 84/84、accept-s3 作者报告、负控 N1–N5。0060 的 base 是 `bba547492d`，没有把它包进去 | `git diff c2bdf9b662 bba547492d`，重点看 2 个 perf 提交的生产路径 |
| ③ | `df0b98f0f7..727b3bfb45`（0059/0061） | 5 | 7 文件 +203/−55 | `460f7a6cfd`、`998ce0218b`：第 6–16 处裁改断言。`0e4c4e659e`：夹具基线。`c7afa89948`、`727b3bfb45`：只改注释 | 断言内容由主控逐条裁定（issuecomment-6287615661），但改写后的测试代码没有非作者独审。0068 的 base 是 `727b3bfb45` | `git diff df0b98f0f7 727b3bfb45`（核改写是否与裁定逐字一致、负控是否真红） |
| ④ | `78523300ad..44d312d7cf`（five-0067 侧支 18 提交 + merge `44d312d7cf`） | 19 | 5 文件 +37/−16（净） | **产品修复** `368e2bcb13`：冷捕获原件树许可按节点线性计费。`3c2615cdae`、`9de6923fd6`、`32c5ec7c04`：夹具/oracle。`7765c998f8`：第 17 处裁改断言。其余为探针、负控及其 revert | 0070 的 base 是 `44d312d7cf`。0067 作者报告与 CI 39481122836 可作旁证 | `git diff 78523300ad 44d312d7cf`，重点 `368e2bcb13` 与 `32c5ec7c04`（P51 区分力变弱） |

建议用一次独审覆盖 ②③④，以及 ① 的存活 hunk。四段总规模约 +2.7k/−1.0k，其中 ① 占大头。另一种做法是对 squash 面 `git diff 6badaa6049 <squash>` 做一次整体终审，一次补齐缺口和 (b)；但规模约 270 文件，代价高。

## 3. 尾部（阶段二）

### 3.1 0084 → 0088 → 0090 首尾相接

| 段 | 区间 | 独审 | 结论 |
|---|---|---|---|
| 合并面 | merge `026e3e97b0` | merge2-review-0084 | CONDITIONAL，无 P1。P2-1（main 前进）由 0086 再合 main 处理；P2-2（6 处自带红）由 0086 清零；P3-1 由 0086 修 |
| premerge-0086 | `026e3e97b0..f0aeaf2817` | premerge-review-0088 | CONDITIONAL。P1-1（S4 在 extras debug 超时）由 0089 `ffb89f7fa8` 改 release-only 关闭；第十八～二十三处裁改 `ratification_18_23.ok=true` |
| fix-0089 | `f0aeaf2817..327aa94c39` | fix-review-0090 | CONDITIONAL，无 P1。P2-1 更正生产可达论证（代码无需改）；P3-1 由 pre-squash 改注释；P3-2 写入声明已知限制 |

### 3.2 阶段一缺口与 S4

由 gap-review-0087 覆盖（§1），评审链缺口全部闭合。S4 锁按负责人裁定 (i) 并入（0086 `de027d90ab`），accept-s4 段即 0087 ⑤。

### 3.3 尚未独审的尾部（交负责人派 squash 面独审）

| 提交 | 内容 | 现有机械证据 |
|---|---|---|
| `8a1924993b` pre-squash | 0088 P3-1/P3-2/P3-3/P3-6、0090 P3-1：只改测试与 CI 注释/yml，4 文件 +15/−5 | 远端 lib bridge 24/0、bin share_parent 1/0（log `20261021-212614-8a1924993b`）；探针 `20261021-212331-46ca88b909` 定 strong=268 |
| `dbfb13d90b` 再合 main `aa1c3885f8` | `02b5b571eb` 之后 main 新进 6 提交（#1652 #1655 #1658 #1646 #1650 #1656）；文件交集只有 `rust/src/bin/s_session_v3/mod.rs`，git 自动合并无文本冲突 | **需补合并面独审**（D8）。squash 面验证见 CLOSE-DECLARATION §8 |
| `5417eea488` 再合 main `21b145250e` | 原 squash（`d5ae676bf9`，基于 `aa1c3885f8`）验证完成、开 PR 前 fetch，main 已进 #1649、#1632（#1608 告警清零 + `ci.yml` rust-check 三档 `RUSTFLAGS=-D warnings` + #1394 编译负例脚本，152 文件）、#1631。与本票 46 文件交集，15 文件 22 处文本冲突，main 一侧全是 #1608 告警门控（`cfg`/`cfg_attr` allow、条件 `use`、`fetch_update`→`try_update`），无语义改动。解法：(a) 冲突块全取本票侧（本票已重写这些处）；(b) 回退 main 两处 `cfg(test)` 门控——`object_index.rs` 的 `NodeSource, NodeStore` 导入、`cache.rs` 的 `clear_handles`，本票非测试路径在用（不回退则 E0599/E0425）；(c) 按四类上下文（s_session/native × 测试/非测试）实测告警，给 s_session 档 94 处告警逐项加 item 级 `#[cfg_attr(<实测条件>, allow(<lint>, reason = "#1488 再合 #1608：…"))]`（四类全告警时用 `#[allow]`），共 114 行、24 文件，不改表达式与控制流（脚本 `workers/close-0085/gate_warn.py`）；native 档 54 条告警不在 CI 门内，未处理 | **需补合并面独审**（D8）。机械证据：探针 `c27482b3fe`（树同 `5417eea488`）三档 `-D warnings` rc=0；`dd633662da` 两个编译负例 PASS（log `20261022-005256`）；squash 面验证见 CLOSE-DECLARATION §8 |
| `0fb96c00ae` 再合 main `66c409e7e3` | 基于 `5417eea488` 的 squash（`5b5e264276`）全套验证跑完时 main 又进 #1659（`issue1588_parse_mem_probe.rs` 与 `s_session_v2/mod.rs` 的 #1653 拒绝文案、新增测试 `v2_serve_stale_begin_with_missing_rule_revision_says_missing`）、#1662（theta_v0 classifier/parser/policy_records）。git 自动合并，无文本冲突；`s_session_v2/mod.rs` 两侧都有改动（本票侧为告警门控与 fmt 涉及的行，main 侧为 `serve` 拒绝分支与测试），自动合并 | **需补合并面独审**（D8）。squash 面验证见 CLOSE-DECLARATION §8 |
| squash `codex/1488-squash-20261021` | 以 main `66c409e7e3` 为父，树 = `0fb96c00ae` 树 + P58 全量 `cargo fmt`（29 文件 +1082/−336；rustfmt 排版：空白、尾逗号、match arm 块括号，非空白 token 变化只这两类）+ 本目录文档与 s5/s86/a5s6/min12 | fmt 前（`0fb96c00ae`）`2b82e386…` 与 fmt 后（squash）`e4f7ee3b…` 生产 nobid 同长 10600736 B，581 字节差全在 `.data.rel.ro`，RELATIVE 重定位两侧 6641 条全等，560 条 panic Location 记录全部只变行号（log `20261022-033217-0fb96c00ae`、`20261022-033549-336b53db8d`） |

### 3.4 squash 面预审 0091（`workers/squash-review-0091/`，对 `336b53db8d`）

0091 是负责人派的 squash 面独审，在 `336b53db8d` 上预审，RESULT `verdict: FAIL`。逐项与本工位处置：

| 编号 | 发现 | 处置 |
|---|---|---|
| 第 2 项 | pre-squash `8a1924993b`：只改测试/CI 注释与 yml，断言更严 | PASS |
| 合并面 `5417eea488` | 冲突取本票侧逐块看完；两处 `cfg(test)` 回退有编译证据（E0599 `object_index.rs:1903`、E0425 `audit_owner.rs:1968`） | PASS（附 P2-1/P2-2）。`dbfb13d90b`、`0fb96c00ae` 的 remerge-diff 为空 |
| 第 3 项 | P58 fmt 29 文件只有空白/逗号/大括号；nobid 560/560 `ALL_PANIC_LOCATION` 复核一致 | PASS，但引出 P1-1 |
| **P1-1** | fmt 把 `content_producer.rs` 中 `raw_bar_at` 的 `.filter(\|bar\| bar.source_index == coord)` 行从 :3109 推到 :3114，`bsp.rs:684` 白名单锚未跟，lib 测试 `no_stray_bsp_binding_rewrites_outside_registry` 失败，main `ci.yml` rust-test-default 会红（0091 log `20261022-031741-336b53db8d`；同测试在 `0fb96c00ae` `20261022-045449`、main `66c409e7e3` `20261022-045538` 为 ok） | **已修**（负责人 04:05 裁定）：终面把锚改为 :3114（核实是同一 `.filter` 行），只动测试。重验见 CLOSE-DECLARATION §8 第四轮 |
| P2-1 | 28 条无条件 `allow` 的 reason「各构建组合均无使用方（可能只在其他 testonly feature 下用）」不实或属推测：一部分在 `s_allocation_json_testonly` 下有使用方（0091 probe log `20261022-023733-321fe5ef58`） | 未改，**交负责人裁定**（只是属性的 reason 文本，不影响编译结果与生产二进制） |
| P2-2 | native 档（`s_session,s_capture_native_testonly`）`RUSTFLAGS=-D warnings cargo check --all-targets` rc=101；main 该档 0 告警，squash 59 条（0091 log `20261022-025240-336b53db8d`） | 未改，**交负责人裁定**。该档不在 main `ci.yml` 门内，§8 照实记 |
| P3-1 / P3-2 / P3-3 / P3-4 | `fetch_update` 两处复活（main 已改 `try_update`）；`parse_batch_value` 被本票复活加 allow；`object_index.rs:38` 整组 `cfg_attr` 比 main 粗；`strong_count == 268` 精确计数脆弱 | 未改，交负责人裁定 |
| P3-5 | 声明 :73 行号旧，:90 引 main `21b145250e` | **已改**：补 squash 面行号（`mod.rs:2465/2574`、`cache.rs:781`、`s_structure_session.rs:5486`），:90 改为 main `66c409e7e3` |

0091 待办：P1-1 修后在终面复跑 `cargo test --all-targets` 与 `--lib --features backtest_bin`，并在终面与 PR 上补审第 1、4、5、6 项。

## 4. 阶段一尾部记录（原文保留）

- `026e3e97b0` 的冲突解法由 merge2-review-0084 审，结论 **CONDITIONAL，无 P1**。
  - P2-1：main 已前进。
  - P2-2：本票源带 6 处自有红：content_bi:3882、content_publication:2406、staged_nodes:675/721/765，以及 bsp 白名单锚点（content_producer.rs:3090，实际在 :3109）。其中 content_bi、content_publication、bsp 在 main 上是绿的，squash 后 main ci.yml 的 rust-test-default、rust-test-s-session、rust-test-s-session-extras 会转红，必须在合入前清掉。
  - P3-1：#1488 的两个 CI job 没有 timeout-minutes，且用了 `if: always()`。
- 0085 动手时 origin/main 已到 `da537bfbcb`，比 `6c6acd5fe0` 多 5 个提交：`d4e6a20230` #1597 SourceCoords、`084ebd700f`、`7835d593f3`、`12e7e16322` ADR 0042、`da537bfbcb` #1629。与 #1488 有 7 个文件交集。阶段二再合 main 后，新 merge 的冲突解法**需补合并面独审**（D8）。
- squash 提交本身：内容 = 合并面树 + 本目录文档/收据 + 全量 `cargo fmt`（P58）。fmt 部分靠「fmt 前后 nobid 逐字节比较、差异只在 panic Location 行号」作机械证据（见 squash 验证收据）。是否另做非作者独审，由主控定。

## 5. 与 S4 锁的关系（旁注，已按 (i) 处置）

`codex/1488-accept-s4-20261020`（mb `c2bdf9b662`，52 提交，改 driver/state/prepared/audit_owner/锁表，+438）**不是合并面祖先**。所以 S4 锁 `v3::driver::tests::s4_generation_incremental_matches_cold_full_six_stages` 不在 `026e3e97b0`，锁表 118 行里也没有它。s1prep-0081 SLICE1-PLAN W22 也记「仓内今天 0 命中」。

CLOSE-GATE-DRAFT A④ 前半把它当作本票证据，并要求「终面 CI 跑这把锁」。这一条在终面不成立。可选处置：
- (i) 并入 accept-s4，并补审上述 52 提交；
- (ii) A④ 前半只以 S3 四锁 + N1–N5 负控销项，S4 锁转 #1640（切片 1，W22 已计划新建）。

负责人 2026-10-21 14:30 裁定选 (i)：0086 步 1 并入，评审链加 accept-s4 段（0087 ⑤）。
