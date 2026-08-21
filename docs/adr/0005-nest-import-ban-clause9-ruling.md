# 条款 9 修订批复：判据层禁引 nest 产物（「互为验证」→「互相独立」，理由 B 收窄保留）

条款 9（`chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md`：判据 crate 禁引 nest 产物）自 2026-07-17 起从无批准记录，而测试守卫与 `interval_necessity.rs` 一直依据这条未批条款运转。#450 实测打掉了条款原文的理由「两套实现互为验证」：nest 侧实为**三套**实现（chain 侧 `NestCandidateEvent` / `judge_first` / `div_cand`），两两不等值，且 748 条经下沉确认的锚点与次级别分类器一类点**交集为空**——互为验证的前提（两边在测同一件事）不成立。#449（grilling，编排者 2026-07-27 裁定）裁定：**禁令维持，理由重写**；本 ADR 是该裁定的文书落库，即条款 9 的正式批复（修订版），DRAFT 文书其余部分维持 DRAFT 不变。

## 条款 9（修订版，批复文本）

> 实现路径（工程）：区间套必要条件应在递归塔内部原生实现（次级别构件、塔内时钟）；nest 管线保持**独立对照实现**身份，其产物禁止回灌判据 crate——理由不是类别错误，而是：必要条件不得由外挂观测器代实现；两套实现**互相独立**（非「互为验证」：#450 实测三套不等值、748 锚点交集为空，验证性前提不成立；独立性反而被三套不等值反证成立——judge 层一旦读证书，三套塌成两套，毁掉的正是唯一的对照面）。
>
> 补充（理由 B：依赖方向 / 未来函数防线，2026-07-28 只读核验后收窄）：`Classification`/tower 是 nest 证书的生产前件。core 判据不得用 nest 证书或 `turn_class` 回写、过滤或改判同一历史锚点的 `Classification`/`BspBits`。这些派生物可以携带晚于历史基例的 `judge_at` 或 `source_index`；缺少显式 as-of 与 `judge_at <= decision_anchor` 约束时，回灌会形成具体的未来信息入口。CI 的 import 禁令是保守的架构防火墙，不表示每个 import 本身已经产生未来函数。

## 理由 B 核验结论（codex 只读核验 2026-07-28；四处关键声称经抽核与源码一致）

1. **证书构建点：证实。** 方向为 `Classification/tower → NestCandidateEvent → TypedNestCertificate/index → admission`——`build_nest_certificate` 是 backtest 侧 `pub(super)`（`econ_positive.rs:898`）；typed 装配在 `nest.rs`（`assemble_typed_certificate`），索引在 `nest_index.rs:262`；当前生产调用来自 `admission.rs`。
2. **as-of/prefix 截断：证实，但须区分两个时钟。** 相对 replay bar `i`，生产 π 路径前缀因果（`classify_at(i)` 只 append 当前 bar；事件查询携 `as_of=i`；段扫描 `end_index <= as_of`）。相对历史基例时钟 `t`，构建器**无自截断**（`nest.rs`：「`judge_at` 只登记 D3 违反率，绝不作硬门」；较晚上级 `judge_at` 直接入证）。时间防线实际位于**消费端**：任一 `judge_at > anchor_index` 整证剔除（`admission.rs` 因果守卫）。
3. **未来信息回流路径：可构造，当前生产未接成。** 链路 = 历史锚点 `t` 的 BSP → `t'>t` 才出现的上级 event/三类点 → 较晚 as-of 入证（`turn_class.rs` 明确搜索 `point.source_index > base_turn`）→ **若** classifier 用它回写/改判历史 `t` → 回放再按历史 `source_index` 执行。仓内 deprecated 全窗入口（`runner.rs:285`）确实存在「全窗分类后放回历史 source_index 执行」的前视模式。故「任一 import 已产生未来函数」**证伪**——还须实际消费、回写历史判据、且缺失时间守卫。
4. **`nest_index.rs` / `turn_class.rs` 本身无回流，豁免成立。** 二者是下游装配/查表与只读派生件，不回写 classifier；`turn_class` 当前生产使用为 `TURN_CLASS` dump 只写不判（`p92_nest_replay_postruling.rs`）。

## Considered Options

- **废除禁令**（旧理由已塌，顺势放开）：被否。理由 A（对照实现独立性）有 #450 实测支撑，禁令死活不依赖旧文案。
- **理由 B 原样保留**（「import 即未来函数入口」的强表述）：被否。核验证伪强表述——回流路径可构造但当前未接成；收窄为「保守架构防火墙」表述后保留，与消费端因果守卫（`judge_at > anchor_index` 整证剔除）各守一段。
- **物理搬家**（nest 三件移出 `classifier/`，令目录 = 层，豁免自执行）：**否于成本而非原理**（裁定原文）。已知弱点在案：`GUARD-ROLE` 自述可骗；若出现第二例自称逃逸，此案重开。

## Consequences

- **豁免清单**：`nest.rs` / `nest_index.rs` / `turn_class.rs` 按 `GUARD-ROLE: nest-pipeline` 角色豁免；`interval_necessity.rs` 声明 `GUARD-ROLE: judge`，继续被拦。守卫改按角色豁免（认声明不认文件名）由 #451 实装（已关）。
- **#416 已解（已关）**：禁令维持 ⟹ `interval_necessity` 只能读候选层（`LevelState.bsp`），而 #450 实测候选层三类点 99.42% 未经下沉背驰确认——它天然见证不到背驰，只能当**只读观测器**，其「必要条件」定性须重写；`build_nest_certificate` 的 `pub(super)` 可见性**不放开**。第二层理由：即便放开，证书层对 84.11% 的信号为空（level0 免检直接放行），接上去买不到东西。
- **DRAFT 标注**：原 DRAFT 待裁清单已标明条款 9 已批及去向（本 ADR）；待裁项 1（塔内原生实装立项）维持待裁。
- **level0 免门暴露为独立问题**：84.11% 信号证书为空、99.42% 三类走 level0 免门，留 map #59 后续票处置，不属本批复范围。
- **程序性**：本 ADR = 条款 9 的首次正式批准记录；批准对象为修订版文本，非 DRAFT 原文。

## 修订（[#1061](https://github.com/xy7365527-lang/NewChanlun/issues/1061)，2026-08-18）：条款 9 理由 A 退役与对照臂退役依据

（append-only：条款 9 批复文本与理由 B 原文一字不动，本节为现行修订。）

- **对照臂退役**：map [#1055](https://github.com/xy7365527-lang/NewChanlun/issues/1055) 已裁「生产扫描成为唯一扫描」——3a 合并生产两路（`extract_signals_with_hist_anchored` + `observations_for_level`），3b P1 对照臂退役（前置 = Lean 镜像签收，[ADR 0025](docs/adr/0026-scan-unify-defense-lean-mirror.md) 裁定三）。
- **理由 A 退役**：理由 A 的前提「两套 Rust 实现互相独立（judge 层读证书则三套塌两套、毁掉唯一对照面）」随第二套实现（P1 装配线）在 3b 删除而物理消失。**禁令维持**，理由收敛为**理由 B 单腿**（依赖方向/未来函数防线，独立成立，与对照臂存废无关）。
- **对照面使命移交**：原「唯一对照面」的防 drift 使命由 **Lean 形式化镜像 + Rust↔Lean 提取对拍**接管（ADR 0025 裁定一/二；执行器 = strict_nest_check 转型为镜像对拍执行器，见 ADR 0025 #1060 补充节）；nest 对照侧 CandDeltaEvent 面保留为**纯投影审计面**（[#1059](https://github.com/xy7365527-lang/NewChanlun/issues/1059)，nest.rs + 8 审计 bin 零改动，诊断余量、非对照面）。
- **#799 裁定六例外面更新**：其援引的「已声明的独立对照臂」（本 ADR 条款 9）现役主体已退役——例外面作为**历史记录**保留（无现役主体），读者转读 ADR 0025（新防线）与 #1059（投影审计面）。
- **退役依据三件**：① 3a 合线结构性消灭装配 drift 类（[#1056](https://github.com/xy7365527-lang/NewChanlun/issues/1056) 考古：P1 独有射程 = 两条镜像线改不同步）；② 镜像签收为 3b 前置、无空窗期（ADR 0025）；③ 投影零改动延续消费面（#1059）。纯文书决策，代码零改动。
