# 结果包：Task #175 etabucket-impl——ηBucket（γ_t 四桶）第 15 维实装

工位：ws-etab | 2026-07-04 | 分支：gap3-rework-codex9-fix（基 a3fb547375 之后 HEAD 3e1767b2ed）
测试：`cargo test --release --lib` **1474 passed / 0 failed / 110 ignored**（HEAD+本任务纯 patch 的
隔离 worktree 验证，不含并发工位在飞未提交改动）

## 1. 结论

ηBucket 按终裁实装面全量落地，零占位零简化：

| 件 | 落点 |
|---|---|
| `EtaBucket { Deficit, Zero, PositiveUnsafe, PositiveSafe }` | `strategy/ledger.rs` RiskPolicy 旁；derive 全家桶同 TStage #149 先例（声明序=η 缓冲递增序，仅供 BTreeMap 报告） |
| 分类函数 `RiskPolicy::eta_bucket(&self, s: &TwState) -> EtaBucket` | PDF §10 分段式**逐式按原文分支序**：η<0→Deficit；η=0→Zero；0<η<η_*→PositiveUnsafe；η≥η_*→PositiveSafe。η_t=`TwState::tw()`、η_*=`RiskPolicy::eta_star()`——零新数据源 |
| MuClass 第 15 维 `pub eta_bucket: Option<EtaBucket>` | `mu_estimator.rs`，紧邻 t_stage 第 14 维；`from_certificate` 恒 None（231 诚实 None） |
| ZExt 装配 | `selector.rs` ZExt 补字段+NONE；`z_of_candidate` 透传；`runner.rs` π fill loop `ext_i` 填 `Some(tw_policy.eta_bucket(&tw))`——与 `t_stage` 读**同一 `tw` 变量同一时点**（②'/②'' 账本更新后），非另开账本查询，零时序错位 |
| 输出键防泄漏 | `perm_test.rs` fullz 置换键 / `wverify_run.rs` fullz_key / `l3_delta_r_alpha.rs` 枚举投影——三处显式 `eta_bucket: None`（η 随 bar 变（Realize 漂移），t_stage 三处先例同款） |
| UClass 不读新维 | `project_to_u` 字段显式构造不经新维；`g3_project_to_u_ignores_new_dims` 测试扩展 eta_bucket 装饰断言（selection 抗碎裂，约束②） |
| RiskMode 独立轴 | 不合并（§6 明文并列独立维）；EtaBucket 枚举文档显式声明 |
| econ 统计层 | 4 处 ZExt 字面量（生产 collect_signals 2 + dx 探针 2）显式 `eta_bucket: None`（无 TW 账本，诚实） |
| 注释关闭 | `mu_estimator.rs` 模块文档 ηBucket 条目从「缺口维持」改「第 15 维已接（#175，引终裁文件名）」；`zdims-impl-20260704.md` 遗留① 追加关闭注记（引终裁+本包，不改写历史正文） |

新增测试：`ledger.rs::eta_bucket_four_value_boundaries`（四值可达 + η=η_* 归 PositiveSafe ≥ 边界 +
η=0=η_* 归 Zero 原文分支序优先 + κ>0 抬 barrier 改判）；`selector.rs` ZExt 透传/None 断言扩展；
`runner.rs` fill loop entry_z 携 `Some(PositiveSafe)` 真值断言（typed_ledger_reverse_close_root）+
两个 χ 护航测试手建键同口径扩展。

## 2. 定义依据

- **分段式**：`docs/formal-chain/完整的策略.pdf` §10 γ_t 四值分段（Deficit/Zero/PositiveUnsafe/
  PositiveSafe 按 η_t 对 {0, η_*} 分割）+ §6 z 第 16 维「ηBucket：负成本缓冲状态」。分段式从终裁
  文件 `a5-etabucket-stance-ruling-20260704.md` 取用（该文件系 Read pages 逐页核对原文），未自造边界。
- **被分类量身份**：终裁（d49610da63）立场B——η_t 生产者=`TwState::tw()`，即 `enter_ready()` 比较式
  `s.tw() >= self.eta_star(s)` 的左操作数；γ_t = 该比较判据的离散化，非独立账本缓冲实体。
- **边界语义**：η≥η_* 含等号（PDF 原文 ≥）⟹ η=η_* 归 PositiveSafe；η=0 时即使 η_*=0 使 η≥η_*
  同时成立，按 PDF 原文分支序 Zero 先判 ⟹ 归 Zero（实装用 if-else 链保序，测试钉死）。

## 3. GOLDEN/断言处置（逐个说明）

- **GOLDEN digest 变动 = 0**：`eta_bucket` 不进结构六 bit / `class_index` / digest 覆盖域（t_stage
  #149 同款），置换桶键（perm_test base）/UClass 折叠均显式排除新维 ⟹ 全部 GOLDEN 原值通过，
  全量 1474/0/110 即证，无重算需要。
- **诚实重算的断言（非 GOLDEN，测试内扩展）**：runner 两个 χ 护航测试手建 est 键补
  `eta_bucket: Some(PositiveSafe)`——非任意值：nav0 平价零已实现 PnL/未退本金 ⟹ η=tw()=nav0 恒等于
  η_*=L^wc=notional_in（κ=0 baseline）⟹ 恒 PositiveSafe（≥ 边界），与 fill loop 查询键同口径
  （训练/查询共用 ext_i，键不齐则全表 miss——G2 教训）；`typed_ledger_reverse_close_root` 补
  entry_z.eta_bucket 真值断言同理。

## 4. 边界条件（结论翻转条件）

- 若发现 PDF 存在第二个与 `TwState::tw()` 语义不同的账本 η 对象，或 `enter_ready`/`eta_star` 与
  PDF §10 的字段级对应实为巧合 ⟹ 终裁翻转 ⟹ 本实装的被分类量选取随之作废（终裁文件同款翻转条件）。
- 若生产路径出现 η_*<0（当前不可能：`l_wc` 下界 0 + κ≥0 构造不变量 + Q=notional_in 非负）⟹
  分段式四支的穷尽互斥前提破 ⟹ if-else 链的原文分支序语义（Deficit 优先）成为实质裁决而非等价
  实现，需回 PDF 复核。
- runner `tw_policy` 若从 κ=0 baseline 改为可配置 κ ⟹ 第 15 维取值分布变（PositiveSafe 收窄），
  存量含 eta_bucket 分层的跑批结论不可直接对比。

## 5. 下游推论

- μ 桶键 +1 维（fill loop 生态 `Some`，随 bar 可变——Realize 漂移可迁移桶）⟹ 旧跑批 μ̂ 与新桶键
  不可直接对比；涉全维分层的存量结论重跑须按新口径标注（t_stage #149 同款警示）。
- 枚举↔fill loop 投影/置换键/wverify 键三处显式 None ⟹ 既有 perm_p/verdict 语义不变（键侧边缘化，
  条件均值塔性质精确）。
- #162（A1-zdims-residual：Jchain+d 维）/#164（A3-weakforce）解锁各半（本任务是其 blocker）。
- `interp.rs`/`mutex.rs` 仍只消费 `enter_ready` 匿名布尔（非阻塞——布尔闸门不依赖桶值）；若日后
  EnterReady 诊断日志需区分四态，接入点已就位（#158 报告消费者清单条目，非本任务范围）。

## 6. 谱系引用

- 终裁 `a5-etabucket-stance-ruling-20260704.md`（立场B 成立，三方法论独立路径收敛：#158 首确认 +
  codex-gap2rulings 条目7 + #174 直接回溯 PDF）——本实装的判定依据。
- 231号（formalization-validity-domain 诚实 None）：统计层/裸证书 None 语义。
- 090号严格性 + no-patch：零常量占位、零死字段、PDF 分段式逐式不自造边界。
- #149 zdims（16ba38d5d5）：t_stage 第 14 维模板——本次装配点/防泄漏/UClass/测试五件套逐款沿用。
- G2(#132)/G3(#138)：训练/查询同口径护航点先例（ext 共享变量层保证）。

## 7. 影响声明

- 改动收入本 commit：`strategy/ledger.rs`（枚举+分类函数+边界测试）、`backtest/{mu_estimator,
  selector,runner,perm_test,wverify_run,l3_delta_r_alpha}.rs`（第 15 维+装配+防泄漏+测试）、
  `econ_positive.rs` 仅 4 个 #175 ZExt hunk（按 hunk 过滤 stage）、`zdims-impl-20260704.md`
  遗留① 关闭注记、本文件。
- **不收**（并发工位在飞，与本任务无数据依赖）：`econ_positive.rs` 的 #170 探针 hunk（ws-xzdc3）、
  `strategy/interp.rs` A9 ActiveLeg 改动（ws-a9pos，当前使主工作树暂不可编译——本任务验证在
  HEAD+纯 patch 隔离 worktree 完成，1474/0/110）、`bin/pi_bsp_timing.rs`。
- 行为影响：χ 分桶在 π fill loop 生态更细（eta_bucket=Some）；χ≡1 路径 bit-exact 不变（新维不进
  订单决策，仅进 z 键与训练/查询两侧同值）。
