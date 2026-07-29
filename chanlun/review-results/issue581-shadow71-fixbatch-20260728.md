# #563 影子评审 MED/LOW 小修批（M2/M4/M5/M7 + LOW L2-L5）

- 日期：2026-07-28
- 前置：#571 已落定（commit `1ba0bcc49e`/`51600f1648`），阻塞解除后开工
- 工位：`/tmp/kimi-nest-mainline`；开工核面：`git status --short` 仅 6 份既有 review-results 未跟踪文件，无 rust 改动
- 纪律：全程前台单线程，无子代理、无后台任务；禁 git 写操作（本报告为唯一写入）；未触碰并行线在飞面（见 §3）
- 评审面来源：`chanlun/review-results/shadow-71-20260728.md` §1 M2/M4/M5/M7 + LOW L2/L3/L4/L5

---

## §1 逐项订正

### M2：μ estimand 定义站点未随量纲③订正

- **位置**：`mu_estimator.rs:1-13`（模块头公式块）、`:34`（不计算 τ_γ 段落）、`:293-295`（`MuObservation` 文档）；`l3_delta_r_alpha.rs:41`
- **订正**：模块头新增「量纲③重锚」小节，显式引用裁定 #65（`chanlun/escalate/chi-dimension-ruling-20260721.md`）与其原文公式 `X = (δ(P_exit−P_entry) − fee) / P_entry`；三处提及 `marginal_return` 作为 X_γ 终值来源的文档改为指向 `chi_dimension_three_return`（`marginal_return` 仅是其内部绝对额子步骤）。`l3_delta_r_alpha.rs:41` 同步。
- **核验**：追踪生产喂入点确认——`l3_delta_r_alpha.rs:271-273`（`est.observe(MuObservation { class: t.entry_z, x_gamma })`，`x_gamma` 来自 `chi_dimension_three_return`）、`pi_bsp_timing.rs:371`/`:465`（`chi_dimension_three_return` 直喂）——三处生产调用点全部经 ③，文档订正与代码一致。
- **行为变化**：零（纯文档）。

### M4：`gamma_dump.rs`/`issue71_chi_gamma.rs` 缺认识论等级标注

- **`gamma_dump.rs:1`**：新增「认识论等级（formalization-validity-domain 231号）：L1（纯只读外化，零信息增量，同 `OpsemDump` 先例）」，比照 `opsem_dump.rs:164` 姊妹模块写法。
- **`wverify_run/issue71_chi_gamma.rs`**：原文件无 `//!` 模块头（直接以 `use` 开头）。新增完整模块头：功能描述、**L2**（真实 BTC 历史数据驱动）等级标注、跑法。
- **行为变化**：零（纯文档）。

### M5：§5-B「基线腿」退化为同进程自比

- **诊断**：`run_baseline_arms`/`assert_baseline_run`/`three_leg_asserts` 的 `unset` 参照臂与其余三臂同 commit 同进程跑，只证明「打开只读 dump 通道不扰动其余三臂」，不证明「四臂是否共同偏离 #71 之前的历史行为」——与本仓旧案「基线逐位相同验收句悬空」（`project_wf8_fourlayer_baseline_negative`）同型。
- **订正（选项二：显式锚定历史 SHA + 注明依据）**：
  1. 新增常量 `PRE_ISSUE71_BASELINE_SHA = "b8a4e75e7109d48191fdb66053c28befb857de2a"`（`717fc4fa35` 的父提交，#71 落地前最后一次提交，`git log 717fc4fa35^` 核验）。
  2. 模块头新增「§5-B①锚定边界」小节：显式声明当前自比语义边界、给出手工跨 commit 复核的三步程序（`git worktree add` → 历史 checkout 跑同款 harness → 逐字节比对）、并诚实声明**未内建**自动跨 commit diff 的原因——需要真实 BTC 数据集且不能在此评审 session 中编造历史数值（090 禁令）。
  3. `run_baseline_arms`/`assert_baseline_run` 加函数级文档，明确 `unset` 是「本 commit 内自比参照臂」而非历史基线。
  4. 常量实际消费进 `render_report` 输出（避免 dead_code，也让报告产物携带锚点，便于人工复核时对照）。
- **行为变化**：零（断言逻辑、四臂跑法均未改，只加文档 + 一个字符串常量注入报告文本）。

### M7：`chi_dimension_three_return` 前置条件仅 `debug_assert`，release 下失守

- **`mu_estimator.rs:396`（原 `:388`）**：`debug_assert!` → **`assert!`**（release 生效），补充 `# Panics` 文档说明为何必须 fail-fast（Inf/NaN 一旦进 Welford 会永久污染桶 mean，无法事后剔除，代价高于崩溃）。
- **`pi_bsp_timing.rs:371`/`:465`（两处生产 μ 喂入点，此前无任何守卫）**：新增显式 `entry_px > 0.0` 守卫（等价于 `entry_notional > 0.0`——`voice.qty` 本 bin 恒为 `1.0`，见 `:244` `Q_Θ=1 单位`），与 `l3_delta_r_alpha.rs` 的 `LedgerDisposition::NonPositivePx` 同一防线；跳过喂 μ 但不跳过平仓/移出 `active`。新增诊断计数字段 `PassResult.diag_nonpositive_px`，末尾打印一行诊断。
- **释义边界确认**：`l3_delta_r_alpha.rs` 路径已由 `ledger_disposition`（`NonPositivePx` 判定）上游过滤，本次改动不影响其行为——`assert!` 在该路径上不可达，一致于改前。
- **release 语义变化（单列，验收要求）**：
  - **改前**：release 下 `entry_px≤0`（若出现）⟹ `chi_dimension_three_return` 静默产出 Inf/NaN ⟹ 污染对应 z 桶 Welford 累加器（该桶 mean 永久 NaN，不可逆）。
  - **改后**：`pi_bsp_timing.rs` 两处生产调用点显式跳过（不再触达 `chi_dimension_three_return`），改为计入 `diag_nonpositive_px` 并打印诊断；`l3_delta_r_alpha.rs`/单测调用点因上游已过滤/入参恒正，`chi_dimension_three_return` 内的新 `assert!` 预期永不触发。
  - **指纹面**：真实标的池（报告 §3③ 已确认）历史上不产生 `entry_px≤0`，故本次语义变化在当前 BTC 数据面上**不改变任何已发表数值**——`assert!`/守卫均是未触发的死代码路径，纯粹的前置条件强化。若未来数据面出现该值，行为从「静默污染」变为「显式跳过+计数」（`pi_bsp_timing.rs`）或「hard panic」（任何未来遗漏守卫的新调用点），均严格优于「静默污染」。

### LOW L2：θ scan 网格量纲注释未随③订正

- **位置**：`l3_delta_r_alpha.rs:1830`（`[-1e-6,-1e-3,-1e-1,...]` 诊断网格）
- **订正**：新增注释说明该网格是量纲①（绝对额）时代遗留刻度，在③（相对收益）量纲下同样近似 `-∞`（与 M1 同源，M1 不在本批范围，未改动数值本身——本项仅订正标注，诊断脚本非生产口径）。
- **行为变化**：零（`eprintln!` 诊断脚本，未改数值）。

### LOW L3：诊断臂写失败 panic 与 crate 内约定不一致

- **核实**：`open_ledger.rs:345` `write_trade_fail_loud`（`#571` 新增）与 `opsem_dump.rs:359` `write_tower_event` 均已是 `unwrap_or_else(|error| panic!(...))`——`OpsemDump` 侧现状**已经**与 `GammaDump` 一致 fail-loud（design 文档 §3 所述的旧 `.ok()?` 口径已在 #571 前后的独立改动中被替换，是本次评审基座 commit `717fc4fa35` 之后才发生的收敛，不是本批引入）。真正与 fail-loud 不同的只有 `admission.rs` 的 `t5a_chain_dump`（`#[cfg(test)]` 限定），其"不 panic"是显式声明的测试期例外。
- **订正**：`gamma_dump.rs` 模块头新增「写失败处理策略」小节，把这一现状**显式写实**——`GammaDump`/`OpsemDump` 是生产构建常驻只读外化通道，一致 fail-loud；`t5a_chain_dump` 是仅 `#[cfg(test)]` 存在的一次性诊断挂件，两者是"两类载体各自定策"而非"同一政策的两种落地"，消除"不一致"的印象。
- **行为变化**：零（纯文档，代码 panic 行为本就已一致，只是文档未记录）。

### LOW L4：位置布尔旗标 + 新 `#[ignore]` 无跑法 doc

- **位置**：`wverify_run.rs:596` 附近 `issue71_chi_gamma_validation` 测试
- **订正**：补函数级文档 + 跑法（`cargo test --release --lib theta_v0::backtest::wverify_run::issue71_chi_gamma_validation -- --ignored --nocapture`），与相邻 `m6_btc_oos_r_decomposition` 的既有写法同款。`issue71_chi_gamma.rs` 模块头同步补跑法（见 M4）。位置布尔旗标（`run_arm(&root,"arm0_both",true,true,..)`）未改——本项范围只覆盖"无跑法 doc"半句，改参数签名属更大改动，不在本批授权范围内。
- **行为变化**：零（纯文档）。

### LOW L5：钩子位措辞偏差

- **位置**：`fill.rs:1288`（`gamma_chi_admitted` 计算处，`gamma_dump.is_some()` 分支）
- **订正**：新增注释订正 gap2 设计稿 §4①「生产路径零额外指令」的不精确表述——关闭时该分支仍逐 bar 求值一次布尔判断（O(1)），只是不产生实际分配/写入；「零额外指令」应读作「零额外分配/写入」。行为无变（关闭时逐字节不变，R5-1 铁律不受影响），仅设计稿声明面订正。
- **行为变化**：零（纯文档）。

---

## §2 编译核验

```
cd rust && cargo build --release --all-targets --features backtest_bin
```

- 本次改动的 7 个文件（`mu_estimator.rs`/`l3_delta_r_alpha.rs`/`gamma_dump.rs`/`wverify_run/issue71_chi_gamma.rs`/`wverify_run.rs`/`pi_bsp_timing.rs`/`fill.rs`）**全部编译通过，无新增 error**。
- 构建整体失败原因**唯一**：`tests/econ_oddeven_diagnosis.rs`（`LevelState` 缺 `cp_ownership`/`level_projection`/`pan_div` 三字段 + `Rc<Vec<_>>` 迭代/类型不匹配共 4 处 error）——`git log -1` 核实该测试文件与 `classifier/mod.rs::LevelState` 的最近一次改动均早于本 session（2026-07-27 及更早提交），与本批改动无关，即验收注明的「当前 2024/1/136 唯一失败 #115」。
- 指纹前后对照：改动前后该失败集合不变（仍是同一 4 条 error，同一目标 `econ_oddeven_diagnosis`），本批未新增/未消除任何编译失败面。

---

## §3 并行线在飞面（未触碰，如实记录）

session 期间 `git status --short` 观测到以下文件被**非本次改动**修改（本次全程只读，未 `Edit`/`Write` 任一文件）：

- `chanlun/agent-roster-2026-07-21.md`
- `rust/src/bin/p123_fast_replay.rs`
- `rust/src/theta_v0/backtest/open_ledger.rs`

三者均落在纪律「禁碰并行线在飞面与 agent-roster*」的保护范围内，本次评审严格未读改这三个文件的内容用于决策依据（仅在核验 M7/L3 时**只读** `grep`/`sed -n` 过 `open_ledger.rs` 确认 `write_trade_fail_loud` 现状，未对其做任何写操作）。

---

## §4 结果包六要素

1. **结论**：#563 shadow-71 评审的 M2/M4/M5/M7 四项 MED + L2/L3/L4/L5 四项 LOW 全部逐项订正，8/8 完成；改动全部为文档/注释/显式守卫层面，唯一携带 release 语义变化的是 M7（已单列指纹对照，当前真实数据面上是零触发的死代码路径强化）。
2. **定义依据**：M2 依据裁定 #65（`chi-dimension-ruling-20260721.md` §1 原文公式）；M4 依据 formalization-validity-domain 231号 + `opsem_dump.rs:164` 先例写法；M5 依据 gap2 设计稿 §5-B①原文 + no-patch-mentality「诚实：代码能做什么就声明什么」+ 090 禁止编造历史数值；M7 依据 coding-style「Fail fast with clear error messages」+ security.md「不吞异常」；LOW 批依据各自缺口条目原文（shadow-71-20260728.md §1）。
3. **边界条件（结论翻转条件）**：
   - M7 的「零触发」结论 ⟸ 真实标的池若出现 `entry_px≤0`，则 `pi_bsp_timing.rs` 的 `diag_nonpositive_px` 计数会 >0（需另查数据源异常），此时应视为新发现而非本批遗留。
   - M5 的「显式锚定」修复若被判定为不满足设计稿 §5-B①「必须自动跨 commit diff」的强验收，则需回到选项一（真正实现跨 worktree 自动对照）——本批选了设计稿明示允许的选项二并已声明依据。
   - L3 的「现状已一致」结论 ⟸ 若发现 `OpsemDump` 或 `GammaDump` 未来又出现 `.ok()`/吞异常写法，则该结论需撤回。
4. **下游推论**：① M1（τ² 冻结先验重锚）与 L2 同源，仍待后续票处理，本批未改数值；② M6（χ 否决裁定书族谱补录 harness）、M3（GammaDump 双源准入量计算）不在本批范围，仍待处理；③ M5 的手工复核程序（`PRE_ISSUE71_BASELINE_SHA` + worktree 对照步骤）是文档承诺，尚未有人实际跑过，下次 #71 相关改动应验证该程序可执行。
5. **谱系引用**：裁定 #65（`chanlun/escalate/chi-dimension-ruling-20260721.md`）；χ 否决裁定书（`chanlun/escalate/chi-line-falsification-ruling-20260728.md`）；shadow-71 评审（`chanlun/review-results/shadow-71-20260728.md`）；gap2 设计稿（`chanlun/review-results/gap2-gamma-candidate-dump-design-20260719.md`）；「基线逐位相同验收句悬空」旧案（`project_wf8_fourlayer_baseline_negative`，M5 同型）；231号认识论等级；090号声明膨胀禁令。
6. **影响声明**：改动 7 个 rust 源文件（`mu_estimator.rs`/`l3_delta_r_alpha.rs`/`gamma_dump.rs`/`wverify_run/issue71_chi_gamma.rs`/`wverify_run.rs`/`pi_bsp_timing.rs`/`fill.rs`），全部为文档/注释/显式守卫/一个新增诊断计数字段，**不改变任何既有生产数值输出**（M7 的守卫在当前数据面上零触发）。未做任何 git mutation（无 add/commit/push），未碰任何并行线在飞面文件。本报告为唯一新写入文件。
