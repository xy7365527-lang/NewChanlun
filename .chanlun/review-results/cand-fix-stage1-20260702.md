# Cand^δ 修复段1：Type2/3 存在性免本级背驰段门（结果包）

- task: #12（编排者 2026-07-02 终裁「按这个实装」——统一下沉锚定形式第一段）
- owner: ws-cand-fix
- date: 2026-07-02
- 认识论等级: 实装 L0（代数分流）+ 验收 L2（真实 BTC 全历史 4,613,599 bar 逐信号分级别重跑）
- 谱系: 673号（Cand^δ_ℓ 范围误用，codex#7 + task#8 双坐实）、606号（区间套有效域=Type1）
- 前置证据: source-audit-type2-divergence-20260702.md（L1 存在性不由背驰）、codex-h1-ndelta-gate-20260702.md（H2-overfiltering-bug）、h2-sample-verification-20260702.md（1473 信号 gate_pass=0 坐实）

## 1. 结论

`build_nest_certificate`（econ_positive.rs）的候选谓词 `Cand^δ_ℓ` 按 bsp 类型分流：
- **Type1**（本级趋势背驰段=区间套原文对象）：照旧走 `div_cand` 背驰段候选谓词（四条件），代码路径逐字不动。
- **Type2/Type3**：不经本级背驰段谓词准入，`cand_k=true`（存在性由结构分类前提保证）。

判型：`is_type1 = (δ==Long ? bits.buy1 : bits.sell1)`（在函数入口计算一次，整条证书统一）。
含 Type1 bit 的信号仍走 div_cand（bit-exact）；仅 Type2/Type3（无 Type1 bit）免门。

## 2. 定义依据

- **Type2 存在性**：第17课L60 完备性——第一类买点后第一次次级别回抽结束点，安全性由「走势必完美/不患」保证，**未挂钩背驰**（source-audit L1 层，逐字已验证）。第53课L28：Type2 正为「小转大、该级别无一类背驰段」而设——「该级别趋势背驰段不存在」是 Type2 存在的前提条件，套用本级背驰段谓词=范围误用。
- **Type3 存在性**：离开中枢回抽不破 ZG/ZD（结构分类前提），非本级背驰。
- **区间套有效域**（606号）：`div_cand`/N^δ 区间套的原文对象是 Type1（趋势背驰→第一类买卖点）。定律一下沉锚定（Type2/3 精确定位下次级别找第一类）是段2（task #13），段1 只解「存在性免门」。
- **对照原文分层**（source-audit）：L1 存在性/安全性（不由背驰）↔ L4 精确定位（区间套+定律一）分层，段1 修 L1，段2 修 L4。

## 3. 验收（三项可证伪，全绿）

隔离环境：git worktree @ HEAD(cf6a5cdfe4) + 仅本次 econ_positive.rs 改动（git diff HEAD 已验证 3 hunk 全为 673 改动）。
（worktree 额外补 HEAD 既有编译缺口 mu_estimator::shrunk_view 漏 `trades` 字段——commit 8ccbe58137 遗留，gap3-bridge 已在主树未提交修复；本补丁仅为放行验证链接，非交付。）

### (1) 全历史分级计数重跑（L2，2314.68s / 4,613,599 bar）

| level | tower段数(结构) | 类型 | pre-fix sig_post | post-fix sig_post | 变化 |
|---|---|---|---|---|---|
| 0 | 40028 | Type3 | 7744 | 11153 | +3409（Type3 免门）|
| 1 | 12077 | Type2 | 0 | 1059 | +1059 |
| 2 | 3565 | Type2 | 0 | 324 | +324 |
| 3 | 992 | Type2 | 0 | 69 | +69 |
| 4 | 244 | Type2 | 0 | 21 | +21 |
| 5 | 61 | Type2 | 1 | 4 | +3 |

- **level1-4 sig_post 从 0 → 1059/324/69/21 = 1473 信号解封**（精确匹配 H2 预测的 1473 = 1059+324+69+21）✓
- **tower 结构计数不变**：40028/12077/3565/992/244/61（第7级=6）与验收基准逐级一致 ✓
- 门滤除全部归零（level0-4 门滤除=0，level5=3 为区间嵌套非本级门）——所有 Type2/3 存在性通过。
- 附带（任务原意内）：level0 Type3 额外解封 3409、level5 额外解封 3。段1 明文覆盖 Type3，故 level0/level5 Type3/2 一并免门是正确行为，非越界。总 Type2/3 解封=4885。

### (2) Type1 路径 bit-exact（等价守卫）

- `div_cand` 函数逐字未动；Type1 信号（is_type1=true）仍以相同实参调用 div_cand。
- 新增回归测试 `multilevel_nest_cert_type2_bypasses_divergence_gate`：同塔 hist=0 下，Type1(buy1) div_cand 条件4 假 ⟹ n_delta=false（守门不变）；Type2(buy2) 免门 ⟹ n_delta=true。**双向锁**。
- 既有 5 个 multilevel_nest_cert 测试全绿（含 Type1 cand_false 传播、Type1 wrong_dir、Type1 两级链正例）——Type1 判定逐条不变。
- 说明：全历史收集路径中无 Type1 信号（level0 全 Type3、level1+ 全 Type2），故 Type1 bit-exact 在单元测试层（结构守卫）验证，非计数层——因无 Type1 计数可比。

### (3) 全量 cargo test --lib 绿

**1369 passed; 0 failed; 94 ignored; 0 filtered out（EXIT=0）**——本次改动跨全库零回归。

## 4. 边界条件（结论翻转条件）

- 若 bsp 分类器对 Type2/3 的结构前提判定有误（把不满足「回抽不破」的段误标为 Type2/3），段1 会让伪信号免门通过——**段1 把存在性判据的责任完全移交给分类器**（信任 bits 的 buy2/buy3/sell2/sell3 已编码结构前提）。分类器若污染，门不再兜底。
- 若某信号同时具 Type1 与 Type2 bit（buy1+buy2），判为 Type1 走 div_cand（有本级背驰段则用其原文对象判据）——此选择保 bit-exact，但若认为「一旦是 Type2 就应免门」则结论不同（当前实装：Type1 bit 优先）。
- 段1 不解精确定位：Type2/3 免门后其 rung 区间仍参与 [J⊆J] 嵌套检查（H2 证该检查从未拒绝这 1473，reject_elsewhere=0）；精确定位（定律一下沉找次级别 Type1）留给段2 task #13。

## 5. 谱系引用

- 673号（生成态）：Cand^δ_ℓ 区间套候选谓词范围误用——615/671 同族（力度/极值谓词污染结构分类致过度过滤）。段1 是 673 修复方案「按 bsp 类型分叉」的第一段（存在性免门），编排者终裁授权。
- 606号：区间套有效域=Type1。
- 615/671：同族先例（μ_f⊊缠论 / MACD C≥A preveto）——本号净新维度=Type1/Type2 判据在 (m1,m2) 上的结构互斥（Extreme 创新极值 ⊥ 走势完备性/不患）。

## 6. 影响声明

- 改动文件：`rust/src/theta_v0/backtest/econ_positive.rs`（唯一交付改动）——`build_nest_certificate` 加 `is_type1` 分流 + `cand_k` 按类型分支；新增 1 个回归测试。+86/-18 行（含测试）。
- 不改定义文件、不改 `cand_predicate.rs`（div_cand 逐字不动）、不改 `nest.rs`。符合「只改实装使其符合原文分层」。
- 下游影响：
  - 解锁 task #13（段2 定位增强）——Type2/3 存在性已放行，定律一下沉锚定可在其上做精确定位。
  - N^δ 门后信号量 level1-4 从 0→1473（+level0 3409 Type3），μ̂ 分桶（P4）现能看到 level≥1 信号，acc-alpha 回测（task #3）的中间级样本从空变为有。
  - 真跨级触达改善：depth=0 占比 pre-fix 95.36% → post-fix 96.50%（level0 Type3 增量拉高分母）；depth≥1 绝对数上升。
- 未自 commit（Lead 统一 commit）。worktree 验证环境待清理。
