# q3 阶段三 codex 侧异质审——审计报告

> 审计对象：`docs/formal-chain/proofs-full-strategy-20260703.md`（HEAD=ef273ac7fb）
> 任务：#131 阶段三，codex-challenger 工位
> 原始 codex 交互全量记录：`.chanlun/review-results/codex-review-20260703-074741-6191.md`

## 总体判定

**部分成立**（否定不是全盘打回，而是命中 1 处需修复的 BLOCKER + 4 处 CONCERN，且 CONCERN 中 2 条经复核可降级）。

§A（全定义唯一性 11 假设）、§B（互斥定理 + 2^10 穷举）、§D（证明义务登记表）核心锚点经我方 grep + Read 交叉核实（14 处抽样锚点：ledger.rs/runner.rs/mutex.rs/nest.rs/six_state.rs/coverage.rs 全部行号与语义准确），证明推导链本身无逻辑洞。问题集中在 **§E（阶段一结果包）未随阶段二重验更新**，属于文档内部一致性缺陷，不动摇 §A/§B/§D 的数学内容。

## BLOCKER（1 项，需修复）

**§E 引用了代码库中不存在的函数名与过时的场景数/穷举规模**

- 位置：文档 §E"边界条件"第 1-2 条
- 具体错误：
  1. `mutex_total_exhaustive_2pow8`——全仓库 grep 确认此符号从未存在，当前唯一的穷尽测试是 `mutex_total_exhaustive_2pow10`（`mutex.rs:296`，m=10）。
  2. "9 场景桶级等价"——当前 `shadow_fold_bucket_equivalence`（`mutex.rs:574`）实际是 12 场景。
  3. §E 同段"∃!机器证明看守"锚点写 `coverage.rs:3571`，与 §A 正文写的 `coverage.rs:4457` 互相矛盾，且两者都不是实际行号（真实为 `coverage.rs:4491`）。
- 根因：§E 是阶段一（m=8 时期）产出的结果包快照，文档头部声明"阶段二...锚点行号已全部重新 Read 验证并修正漂移（interp.rs/coverage.rs/mutex.rs/econ_positive.rs/transition.rs 五文件）"**未提及 §E**，§E 因而在 #124 把 P1..P8 重分解为 P1..P10 之后未同步更新，且不像 §F 那样有"本节正文保留为阶段二时点记录"的历史标注。
- 判定依据：文档自身在 §F 边界条件（第382行）声明的失效判据是"任一锚点符号 Read 不存在（非行号偏移）"——`mutex_total_exhaustive_2pow8` 完全符合这一判据，是文档自认的失效条件，非我方外加的标准。
- 影响域：**局限于 §E 本节**。§A/§B/§D 正文（互斥定理证明、2^10 穷举实装、D-5 逐谓词锚点表）全部使用正确的 `2pow10`/12场景/正确函数名——核心证明内容未受影响。
- 修复建议：改 §E 为 `2^10=1024`/`mutex_total_exhaustive_2pow10`/`12场景`，`coverage.rs:4491`；或比照 §F 做法在 §E 开头加"本节为阶段一时点记录，阶段二后过时数字见 §B/§D-5"的历史标注。二选一，工作量都是几行文字。

## CONCERN（4 项，经复核后的严重性判定）

1. **确认成立**：`pi_theta_step_deterministic_unique_order` 的行号在 §A/§E 两处引用（4457/3571）均不准确，实际为 `coverage.rs:4491`。函数本身存在且语义相符（同输入两次调用比较订单逐字段相等，符合"决定性=∃!可观测面"的看守语义）。按文档自身"行号偏移非失效判据"的声明，此项不构成打回理由，建议顺手修正行号。

2. **部分成立，判定降级**：codex 指出"determinism 测试只证明同一实现两次调用一致，不是数学意义的∃!证明"。核实文档实际结构后判定：文档的 ∃! 论证主体是 §A"证明结构"段的 11 假设复合链（"由假设1-4⟹Γ_t唯一...链上每一步都是全函数且输出唯一⟹复合全函数⟹∃!O_{t+1}"），determinism 测试被文档明确称为"机器证明**看守**"（sentinel/regression guard），而非"证明"本身——这个措辞已经部分对冲了 codex 的质询。codex 的观察是有效的精度提醒（看守和证明的边界需要更显式区分），但不构成新的逻辑洞，降级为措辞改进建议而非需修复的缺陷。

3. **不成立，判定驳回**：codex 指出"2^10 穷举复用 `Predicates::p(j)`，未验证谓词语义正确性"。核实后发现文档 §B 认识论等级段（第208行）已明文自我声明："Rust 穷举 2^10 = L1 管线正确性（验证互斥化实装无bug，**不验证**谓词 P_j 经验有效）"——codex 质询的边界恰好是文档自己已经诚实划定并声明在案的边界，不是未披露的缺口。这条 codex 发现不成立为新问题。

4. **确认成立，但为前瞻性加固建议非当前缺陷**：
   - `n_delta_rec` 依赖 `rungs` 从高到低排列的顺序前提，代码用文档注释声明（"`rungs`**从高到低**排列"）而非运行时断言强制。核实 `NestCertificate.rungs` 为 `pub` 字段，理论上外部可构造违反顺序的实例；但生产路径只有单一构造点，且有 `is_sub_nesting_bit_exact`（nest.rs:326）机器证明覆盖顺序相关行为。
   - `stage_progression` 白名单/黑名单边界（`transition.rs:309-316`）由注释声明，函数签名接收完整 `TwState`（含黑名单字段 `hwm_gain`），未用类型投影结构性阻止误读。核实当前函数体只实际读取白名单字段（`notional_in`/`holding`/`withdrawn`/`free`），当前无违反；风险在于未来重构可能无声破坏此边界且无编译期/测试兜底。
   - 两项都是真实的软件工程加固空间（类型级前提强制 > 注释级前提），但都不是当前已证明内容中的逻辑漏洞——当前代码路径下前提确实成立且有配套机器测试覆盖当前行为。判定：CONCERN 可接受留白，建议登记为后续加固项，不阻塞 q3 验收。

## 边界条件（本判定在什么条件下翻转）

- 若 §E 的 stale 引用被发现不是孤立的（即 §A/§B/§D 正文本身也存在类似的、我方抽样未覆盖到的行号/函数名错误），则本判定从"局部 BLOCKER"升级为"全篇锚点可信度存疑，需要逐条重新 Read 全量核验"。当前抽样（14 处锚点，覆盖 11 条假设中的 8 条 + §B/§D-5 核心 + §C.5 TW 不变量）全部准确，未观察到 §E 之外的类似错误。
- 若 rungs 顺序或 stage_progression 白名单边界在未来被证实已经在生产路径中被违反过（而非仅是理论风险），CONCERN 4 应升级为 BLOCKER。

## 影响声明

本次审计不修改任何代码或文档，只产出判定。涉及模块：`docs/formal-chain/proofs-full-strategy-20260703.md` §E（需修复）；`rust/src/theta_v0/strategy/{mutex,coverage,ledger}.rs`、`rust/src/theta_v0/backtest/runner.rs`、`rust/src/theta_v0/classifier/{nest,six_state}.rs`、`rust/src/theta_v0/closed_loop/transition.rs`（仅读取核实，未改动）。

## 谱系引用

- formalization-validity-domain 231号：L0/机器证明/L2 分级——本次审计的"看守 vs 证明"措辞质询即适用此谱系。
- no-patch-mentality 090号：声明膨胀禁止——§E 未标注历史性属于"残留过时表述未被诚实标注"的一种，与 090号精神一致，§F 的历史标注做法是正例参照。
