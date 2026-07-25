# 跨级链既有实现盘点与完成度评估（nest-chain-existing-inventory）

日期：2026-07-20　工位：调研文档（只读盘点，零代码改动）
环境：worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD = `640609071d`
方法：git log/show（只读）+ 源码直读；每条判定附代码/commit/文档锚。行号均为 HEAD（`640609071d`）树内行号。

---

## 1. 任务 commit 定位（问题①）

| 任务 | commit | 在 HEAD 祖先链？ | 改动文件 | 性质 |
|---|---|---|---|---|
| task-106 区间套必要条件递归塔原生实现 | `6075840687` | **否**（仅在 `main` 与 `merge-mainline-20260719` 上） | `rust/src/theta_v0/classifier/interval_necessity.rs`（+256，新建）、`rust/src/theta_v0/classifier/mod.rs`（+2） | 代码 |
| task-100 nest 证书×BSP 全量对账 | `fdb146ea50` | 是 | `chanlun/review-results/nest-cert-bsp-recon-20260717.md`（+32） | 文档（探针 `p100_cert_bsp_recon.rs` 在收口 commit `640609071d` 落盘） |
| task-101 证书→买卖点接入物化校验 | `b5f2b4764c` | 是 | `chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md`（+47） | 文档（探针 `p101_cert_bsp_tag.rs` 在 `640609071d`） |
| task-103 截断重放双测 | `fe03fd5c28` | 是 | `chanlun/review-results/p103-truncation-replay-doubletest-20260717.md`（+48）、DRAFT +3 | 文档；侧信道代码 `P92_CKPT`（`rust/src/bin/p92_nest_replay_postruling.rs:484-485`）在 `640609071d` 落盘 |
| task-105 Trend C 终段修复 | `1a377f68a6` | 是 | `rust/src/theta_v0/classifier/level_view.rs`（+5/-2，:691 `rev().find`→`find`）、`chanlun/review-results/p104-trend-div-attribution-20260716.md`（+19/-2） | 代码+文档 |

关联 commit（同一波）：task-104 归因 `717a93e6aa`、裁定申请 `d8e5da82df`（均在 HEAD 祖先链）；task-107 级别标定 `f49475510e`/`d32f6d8b2b`/`d3ba02dbf3`、task-108 收敛探针 `e80294b291`（**均不在** HEAD 祖先链，与 task-106 同在 `main`/`merge-mainline-20260719`）。

⚠️ **分支事实（090 照实）**：task-106 的 rust 实装 `interval_necessity.rs` **不存在于本分支 HEAD 树**（`git ls-tree HEAD` 无此文件，`git merge-base --is-ancestor 6075840687 HEAD` 为否）。HEAD 收口 commit `640609071d` 提交信息中的「#105/#106/#111 遗留 rust 实装」指 p105/p106/p111 **探针 bin**（`rust/src/bin/p105_cert_level_spectrum.rs` 等），不是 task-106。下文对 task-106 的评估基于 `git show 6075840687` 读到的文件内容。

---

## 2. 既有实现全量盘点表（问题②③素材）

### A. classifier/nest.rs —— 严格证书结构 + 三门 DFS 装配（N^δ 递归核·塔外）

| 环节 | 锚 | 实现内容 | 完成度 | 证据 |
|---|---|---|---|---|
| 梯级结构 | `rust/src/theta_v0/classifier/nest.rs:155-165` | `NestRung{confirm_src, child_interval, interval, cand}`：本级定位区间 + `Cand^δ_ℓ` 取值；`Cand^δ` 定义式缺失（spec 疑点2，:151-153 `[需人工确认]`） | 完整（结构）；Cand 定义式缺（上游） | nest.rs:151-153 |
| 证书结构 | nest.rs:234-247 | `NestCertificate{side, terminal, base_interval, rungs}`；rungs 从高(ℓ)到低(e+1)，空=纯基例 | 完整 | nest.rs:244-246 |
| 递归判定 | nest.rs:320-341 | `n_delta()`/`n_delta_rec`：基例 `Conf^δ_e=confirm_side`；递归步 `cand ∧ is_sub(child,parent) ∧ 递归`——**真跨级递归** | 完整 | nest.rs:325-341 |
| 严格装配 | nest.rs:627（`assemble_typed_certificate`）、:692（`extend_typed_upward` 三门 DFS）、:759（`assemble_typed_certificates`）、:862-1076（snapshot/terminal 双口径） | 事件驱动严格装配，链不完整不出半成品（:852） | 完整 | nest.rs:846-856 |
| 消费域 | `rust/src/bin/strict_nest_check.rs:1587`；探针 p92/p107/p108/p109/p111/p116/p123/p124；`runner.rs:221` sidecar（env `THETA_STRICT_NEST_SIDECAR` 门控，runner.rs:242-246）；`turn_class.rs:367` 等 | **生产入场门不消费**；全量 BTC 仅产 91 张证书 | 部分（塔外装配物） | roadmap `chanlun/plans/mainline-merged-roadmap-20260717.md:73`「`NestCertificate` 为塔外装配物」；nest-exit 卡 `chanlun/review-results/nest-exit-implementation-card-20260719.md:77` |

### B. strategy/nest.rs chi_bool + interp.rs nest_confirm —— 通用递归核 + 基例消费

| 环节 | 锚 | 实现内容 | 完成度 | 证据 |
|---|---|---|---|---|
| 递归核 | `rust/src/theta_v0/strategy/nest.rs:122-145` | `chi_bool(ev, chain)`：空链=false；末级=ev 取 `confirm_ok`；递归步 `candidate_ok ∧ 次级严格低一级 ∧ sub_b(次⊆本) ∧ 递归`；全定义绝不未定义 | 完整（真跨级） | nest.rs:112-121 docstring |
| 区间套判定 | strategy/nest.rs:105-110 | `sub_b`：次级别 chosen ⊆ 本级别 chosen；无候选=false | 完整 | — |
| 基例消费 | `rust/src/theta_v0/strategy/interp.rs:382-395` | `nest_confirm` 构造**单级链**（e=ℓ，candidate_ok=true）调 `chi_bool` ⟹ 只产**基例 Conf^δ**；诚实标注「Classification 不导出塔，只产证书基例」（interp.rs:375-377） | 雏形（基例 only，照实标注） | interp.rs:369-377 |
| 入场侧消费域 | interp.rs:285、:1034 | `assemble_gamma` 系给候选贴 `nest_confirmed` 标；**贴标不参门**（runner.rs:5823「贴标字段不参门，门消费 build_gate_certificate，单一裁决源」） | 雏形（只贴标） | runner.rs:5823 |
| 出场侧消费域 | `rust/src/theta_v0/strategy/exit.rs:181-184` | `reverse_nest_cert_base` = v0 基例门；照实「门拒绝率≈0……不是真跨级门」（exit.rs:175-180）；env `THETA_NEST_CERT_GATE` 门控（exit.rs:42-44）；**v1 E2E-N3 真链接口已预留未接入**（exit.rs:45、:154-157） | 雏形（v0 基例当门） | exit.rs:170-180 |

### C. econ_positive.rs —— 生产门的多级 N^δ 构造（塔外近似）

| 环节 | 锚 | 实现内容 | 完成度 | 证据 |
|---|---|---|---|---|
| 多级构造 | `rust/src/theta_v0/backtest/econ_positive.rs:890-971` | `build_nest_certificate`：执行级 `tower[lvl]` 定位段（:899-900），rungs 从 `tower[lvl+1..]` 逐级找含 `source_index` 段（:926-954）；**partial chain 合法**（无上级包含段 break 不拒，codex #39 Q1 裁定，:936-941）；走 `from_parts` 数据载体路径（:970），**非**三门 DFS | 部分（构造真跨级，但是塔外近似） | nest-exit 卡:77「econ 门逐级 is_sub∧cand∧递归（econ_positive.rs:890-1009）是塔外近似」 |
| 门函数 | econ_positive.rs:681-693 | `build_multilevel_nest_cert` = 构造 + `n_delta()` | 部分 | — |
| 二通道证书 | econ_positive.rs:1572-1584 起 | `build_gate_certificate`：Nest（n_delta）/ Xzd（小转大 gate_pass）二通道 | 完整（在其声明域） | — |
| 入场路径消费 | 统计信号收集 econ_positive.rs:364-374（`collect_signals` 循环内）；runner 生产门 `nest_gate_admit` runner.rs:996-1044，调用点 runner.rs:1022（升格路径 b）、runner.rs:5928（econ 准入门）、runner.rs:1711 附近注释同源声明 | 生产+统计同源单门（090 no-patch） | 完整（接线）；部分（链深度） | 见下行实测 |
| 真跨级产量实测 | econ_positive.rs:349-352 | **「BTC 300K 实测 95.36% 通过门信号 rungs 空 ⟹ n_delta 退化为 base-case Conf^δ_e，仅 4.64% 真跨级 J 嵌套（max 深度=1）」** | **部分（结构在、产量极低）** | econ_positive.rs:349-352（引 `acc_classification_level_hole_dx` 有效深度分布） |
| 深度诊断 | econ_positive.rs:973-985 | `effective_nest_depth` 与生产门同算法同源（bit-exact 深度读数） | 完整 | — |

### D. descend_type1_anchor_depth —— 下钻方向（上钻的对偶）

| 环节 | 锚 | 实现内容 | 完成度 | 证据 |
|---|---|---|---|---|
| 递归下钻 | econ_positive.rs:713-741 | Type2/3@ℓ 精确点=次级别 Type1（定律一，第29课L396）；沿 `sub_moves` 找 `end_index==source_index` 段跑完整 `div_cand`，锚成立则**真递归**继续下钻（:737-740）；`sub_moves` 空=递归底=None=小转大；⊆ 由 Compose 不变量结构性保证不重查（:708-709） | 完整（在其声明域：存在性锚，非完整 N^δ 链） | econ_positive.rs:695-712 docstring |
| 递归深度 | :737-740 | 返回 `Some(d)`，d=穿越层数；**深度值只消费于 `is_some()` 判别**（:822/:841/:1251）与归因贴标（:5288），深度本身不入门、不参与信号排序 | 完整（判别式） | 单测 :2339（Some(1)）、:2393（Some(2)） |
| 调用域 | `cand_delta_type2_completion` :815-823；`cand_delta_type3_retest` :834-842（lvl==0 免门）；`pan_div_gate_pass` 通道1 :1251 | Type2/Type3 信号的 base gate + Q4 盘整背驰承接 Nest 通道 | — | — |
| 生效入场路径 | base gate → `build_nest_certificate` :915/:1042 → `build_gate_certificate` :1584 → ①统计门 econ_positive.rs:364 ②runner 生产门 runner.rs:1022/:5928；PanDiv → `gate_pan_div_for_production` :1294 → runner.rs:1793 | Type1 信号**不经**此（Type1 走 per-rung `div_cand`，:856） | — | — |

### E. task-106 interval_necessity.rs —— 塔内原生必要条件检查器（**不在本分支**）

| 环节 | 锚 | 实现内容 | 完成度 | 证据 |
|---|---|---|---|---|
| 必要条件检查 | commit `6075840687`：`rust/src/theta_v0/classifier/interval_necessity.rs`（256 行） | lvl k（k≥1）点的必要条件 = 存在 lvl k-1 **同侧**点，`source_index` 落在本级离开窗口 `[min(center.end,src), max(center.end,src)]` 内；窗口式判据（P108 实证：同 bar 硬判据 70-92% 误杀，离开窗口 100% 成立）；只消费 `LevelState.centers/bsp`，不触 nest 产物；**只读不回写 bit，不是过滤器**；lvl0 vacuous；无中枢=不可判单列 | 雏形（检查器非过滤器；未并入本分支） | interval_necessity.rs 模块 docstring（条款 9 + p108 裁定建议 1-3） |
| 测试 | 同文件 `#[cfg(test)]` | 4 单测：窗口见证满足/缺失、异侧不见证、无中枢不可判、双 bit 双行 | 完整（测试） | — |

### F. task-100/101/103 —— 验证与外围接入层（文档+探针，非链本体）

| 任务 | 锚 | 结论 | 对跨级链的意义 | 完成度 |
|---|---|---|---|---|
| task-100 | `nest-cert-bsp-recon-20260717.md:7-31` | 91 证书（A46/B45）× 28,417 BSP：w1440 全覆盖、median dt=0；反向覆盖仅 0.17-0.97%（证书密度低 ~300 倍） | 证书落点与 BSP 体系一致；证书**不能**作独立入场源 | 完整（对账 PASS） |
| task-101 | `cert-bsp-binding-ruling-DRAFT-20260717.md:12-45` | 接入=sidecar 打标（条款 1：判据层不可见）；91/91 绑定、0 tie；条款 7 多证书合并（末端 39 张同簇）；**DRAFT 待主人裁** | 链产物的外围接入方式裁定（未终审） | 部分（DRAFT） |
| task-103 | `p103-truncation-replay-doubletest-20260717.md:15-36` | unfold 66/66 精确展开（无迟到/早现/未覆盖）；1 幽灵=**top 链身份迁移**（base 判定 append-only 成立）；D3 逐边时序无单调约束 | 证书链的中段因果性验证：exec-base 稳定、top 身份可迁移 ⟹ 稳定主键=exec-base（DRAFT 条款 8） | 完整（双测） |

### G. task-105 —— 基例上游修复（Conf^δ_e 的输入质量）

| 任务 | 锚 | 内容 | 完成度 |
|---|---|---|---|
| task-105 | `level_view.rs:691`（commit `1a377f68a6`）；`p104-trend-div-attribution-20260716.md:60-75` | Trend C 终段 `rev().find`→`find`（离开最后中枢后第一个同向段）；修复前 Trend 背驰确认 1/807（C 段被锚到数据末端恒 false），修复后 429/521（82.3%），Pan 对照不变，classifier 单测 317 过 | 完整（在其声明域：基例背驰确认输入修复，非跨级步本身） |

---

## 3. 完成度判定（问题③）

### 3.1 rungs 是真跨级还是同级基例？

**结构上真跨级，实证上 95%+ 退化为基例。**

- 结构：`build_nest_certificate` 的 rungs 从 `tower[lvl+1..]` 上级层收集（econ_positive.rs:919-954），`n_delta_rec` 逐级 `cand ∧ is_sub ∧ 递归`（nest.rs:325-341）——递归步是真跨级语义。
- 实证：econ_positive.rs:349-352 照实注释——通过门信号 **95.36% rungs 空**（退化 `Conf^δ_e` 单 bit 方向确认），仅 4.64% 真跨级，**max 深度=1**。
- 严格装配侧同病：task-107（commit `f49475510e`，不在本分支）实测 91 证书全绑 classifier lvl=0，「同级不对应」——nest exec/top 级别标定与 classifier lvl 断裂。

### 3.2 descend_type1_anchor_depth 的递归深度与调用域

- **递归深度**：理论上沿 `sub_moves` 可递归到 level0 递归底（:719-722, :737-740），单测见证 Some(1)/Some(2)（:2339/:2393）；但返回值只以 `is_some()` 消费（:822/:841/:1251），**深度数值不进任何门/排序**（仅 :5288 归因贴标）。
- **调用域**（谁调用）：`cand_delta_type2_completion`（:822）、`cand_delta_type3_retest`（:841）、`pan_div_gate_pass` 通道1（:1251）。
- **生效入场路径**：Type2/Type3 信号与 PanDiv 承接信号——①统计层 `collect_signals`（econ_positive.rs:364-374）；②runner 生产开仓准入门 `nest_gate_admit`（runner.rs:1022/:5928）；③PanDiv 生产门 `gate_pan_div_for_production`（:1294）→ runner.rs:1793。Type1 信号不经此函数（走 per-rung `div_cand`，:856）。

### 3.3 task-106 与 interp.rs nest_confirm 的关系

**两套并存、不同层、无调用关系，不是同一套。**

- task-106（`interval_necessity.rs`）：classifier 层**塔内原生必要条件检查器**——lvl k 点需 lvl k-1 同侧见证落在离开窗口内；只读、不是 N^δ 递归、不过滤不回写；条款 9 明确与 nest 管线互为独立对照（nest_isolation_guard 公理级守卫）；**且不在本分支**。
- interp.rs `nest_confirm`：strategy 层 N^δ **基例**（单级 Conf^δ），经 `chi_bool` 机器。
- 二者判据形状不同（窗口见证存在性 vs 方向化确认 bit），语义不同（必要条件检查 vs 证书基例），无任何相互调用（task-106 只消费 `LevelState.centers/bsp`；nest_confirm 只消费 `BspBits`+dir）。

### 3.4 总体判定：跨级 N^δ 链 = **部分**

已就位（链的各环）：

1. 递归核两套且均完整：`chi_bool`（strategy/nest.rs:122-145，全定义 Bool 递归）、`n_delta_rec`（classifier/nest.rs:325-341，证书递归）。
2. 多级构造在：`build_nest_certificate` rungs 收集（econ_positive.rs:890-971）。
3. 严格三门 DFS 装配在：`assemble_typed_certificate(s)`（nest.rs:627-792）。
4. 生产门接线在且单一裁决源：`nest_gate_admit`（runner.rs:996-1044）↔ `collect_signals`（econ_positive.rs:364）同函数同判据。
5. 下钻对偶在：`descend_type1_anchor_depth`（:713-741）。
6. 验证层在：task-100/101/103（对账/打标 DRAFT/截断重放双测）+ sidecar（runner.rs:221）。
7. 基例输入质量修复：task-105（Trend 背驰确认 1/807→429/521）。

缺口 X（缺这些才到「已完整」）：

1. **塔内原生输出缺**（E2E-N3 未结算）：证书链全部是**塔外装配物**，不是 `Classification` 原生、可复验、可持久化输出——roadmap `mainline-merged-roadmap-20260717.md:73` 明示；nest-exit 卡:77 称 econ 门为「塔外近似」。
2. **真跨级产量缺**：95.36% 退化基例、深度≤1（econ_positive.rs:349-352）；级别空洞（`acc_classification_level_hole_dx`）未结算。
3. **级别标定断裂**：nest exec/top ↔ classifier lvl 不对应（task-107 `f49475510e`，91 证书全绑 lvl=0；该 commit 与 task-106/107/108 均不在本分支，在 `main`/`merge-mainline-20260719`）。
4. **Cand^δ 定义式缺**：spec 疑点2，`[需人工确认]`（nest.rs:151-153）——递归步的 Cand 只作 0/1 取值消费，判据定义未封闭。
5. **必要条件检查器未并入本分支**：task-106 `interval_necessity.rs` 在 `main`/`merge-mainline-20260719`，本分支 HEAD 树无此文件（git ls-tree 实证）。
6. **出场侧真链缺**：出场仅 v0 基例门（拒绝率≈0 照实，exit.rs:175-180），v1 E2E-N3 真链仅接口预留（exit.rs:45、:154-157）；mod.rs close 桶与 interp.rs 规则2 close 桶证书化登记为待接线（exit.rs:46-47）。
7. **task-101 接入裁定仍 DRAFT**：sidecar 打标条款待主人裁（`cert-bsp-binding-ruling-DRAFT-20260717.md:47-50`）。

---

## 4. 可复用项清单（v1 真链可直接复用）

| 可复用项 | 锚 | 复用方式 | 注意 |
|---|---|---|---|
| `chi_bool` 递归核 | strategy/nest.rs:122-145 | v1 真链的判定机（进出场同一台机器先例已立：exit.rs:170-173） | 需喂真多级 `chain`（当前 interp 只喂单级，interp.rs:388-394） |
| `NestCertificate`/`NestRung` + `n_delta_rec` | classifier/nest.rs:155-341 | 证书载体 + 递归判定直接复用 | 双构造路径：`from_parts` 数据载体（不保证装配前置）vs 三门 DFS（保证）；v1 须走 DFS 或等效守卫（nest.rs:250-259 诚实边界） |
| `assemble_typed_certificates` 三门 DFS | nest.rs:627-792 | 严格装配生产函数（探针已验证：task-103 unfold 66/66） | 当前消费域=探针/sidecar，未入生产门 |
| `build_nest_certificate` rungs 收集循环 | econ_positive.rs:919-954 | 塔→rungs 的含段查找（partition_point + partial chain 裁定 codex #39） | 是「塔外近似」（nest-exit 卡:77）；v1 若升格塔内原生须迁出 econ |
| `descend_type1_anchor_depth` | econ_positive.rs:713-741 | 下钻方向（Type2/3 锚 + PanDiv Nest 通道） | 只判存在性；深度值当前不入门 |
| `cand_predicate::div_cand` | econ_positive.rs:725-732、:797-804 调用点 | Cand 判据单一来源（上钻/下钻同一 `div_cand`，:711-712） | Cand^δ 定义式缺口（spec 疑点2）仍需裁定封闭 |
| 真嵌套塔 + Compose ⊆ 不变量 | `classifier/recursive_tower.rs`；不变量消费 econ_positive.rs:708-709、:957-969 | 跨级 ⊆ 的结构性保证（免逐边重查） | task-103 实证 D3 逐边时序无单调约束——逐边确认钟不能假设单调 |
| 塔导出桥（环3b 喂入段） | interp.rs:397-428 起 `coverage_elements_with_tower` | 真父子候选元素的既有桥 | 547 铁律（父只来自 sub_moves 真包含） |
| 出场侧升格预留点 | exit.rs:181 `reverse_nest_cert_base`；exit.rs:154-157 | v1 升格只换该谓词实现，四析取结构不动 | 设计已预留，实装未做 |
| 生产门单一裁决源 | runner.rs:996-1044 `nest_gate_admit` | v1 真链当门的现成接线位 | 与统计门同函数约束（090 no-patch）须保持 |
| 验证基建 | `strict_nest_check.rs:1587`；`p92_nest_replay_postruling.rs:484-485`（P92_CKPT 侧信道）；p100/p101 对账探针 | v1 真链的回归/对账/截断重放双测直接复用 | task-103 口径差异（p92 终端 66 键 ≠ nest 91 张，p103 文档:40-42）需先知 |
| 稳定主键纪律 | DRAFT 条款 8（`cert-bsp-binding-ruling-DRAFT-20260717.md:29-31`） | 打标/发布主键=exec-base 判定事件，top 链身份仅归因 | task-103 幽灵实证支撑 |

---

## 附：本盘点未覆盖（诚实边界）

- task-107/108 的报告正文未逐行读（commit 不在本分支；仅 commit message 与 task-106 文件内容锚定）；其结论（级别标定断裂、收敛探针数据）以 commit message 为锚，v1 设计前应读 `main` 分支上的 p107/p108 报告全文。
- `recursive_tower.rs` 内部（Compose/CP 扫描）未展开读——本盘点只锚定其不变量的消费点。
- 91 张证书的逐张谱系未核对（task-100/101 报告已给聚合口径）。
