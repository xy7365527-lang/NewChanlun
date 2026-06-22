---
id: '554'
number: 554
title: "A 路 D_TOP 区间套精确切换 L3 否证——链坍缩，有效域≈空集 @ a0=segment（第27课区间套三必要条件在 1min a0=segment 真实数据几乎从不同时满足）"
type: 概念发现
status: settled  # A 路 D_TOP L3 否证已结算（有效域≈空集 @ a0=segment）；a0=stroke 更细尺度未试（开放轴）
date: '2026-06-22'
settled_date: '2026-06-22'
level: "A 路本体 = L0（rec_engine 最高级别区间套 D_TOP 精确切换 + anchor 条件门控 + 平空对称，worktree 3cc8c49b7a，113 单测绿 bit-exact）；A 路 D_TOP L3 否证 = L3（T_A_PATH_NESTED 8 标的 ×3 strat_pct，a0=segment，worktree wf_2f91805d-f6f-1，env 743s 全绿，OFF/ANCHOR 对照）；有效域 ≈ 空集 @ a0=segment（n_a_path_switches 21/24 cell = 0，其余 ≤1）"
epistemological_level: "L3（8 标的 ×3 真实数据，a0=segment，否定性结果——D_TOP 区间套链 @ a0=segment 全面坍缩，有效域 ≈ 空集，formalization-validity-domain 有效域 ≪ 定义域第 N 例）。**否证的是『区间套 D_TOP 精确识别 @ a0=segment』这一形式的有效域**（≈ 空集），不是『对称双吃目标本身不可达』——区分形式（@ a0=segment 区间套精度）vs 目标（吃每级别涨跌幅绝对值）。a0=stroke 更细尺度（526 A0Source）未试，前景不乐观（pcf 高层链坍缩可能结构性），不声称可达、不声称全分辨率不可达。"
负责工位: "A 路本体实装 worktree 3cc8c49b7a（commit，113 单测绿）+ 8 标的 L3 后台 agent（wf_2f91805d-f6f-1，T_A_PATH_NESTED，743s 全绿）"
provenance: "[新缠论:实装+八标的 L3 回测+诊断]"
negation_source: homogeneous
negation_form: separation
negates: "553 号 A 路开放轴的隐含命题『区间套级联精度 > cascade 粗识别，能避免强牛误触发、吃到跌那一半』——被证伪：区间套 D_TOP 链 @ a0=segment 坍缩（n_a_path_switches 21/24 = 0），A 路退化为『anchor 锁核心 + 低级别空腿照常失血』混合态，比 OFF 和 ANCHOR 都差（仅 4/24 巧合 = OFF，15/24 偏离 >10pp）。精确识别『走势完成』@ a0=segment 在 1min 分辨率不可操作。"
topo_effect: "sever:T_A_PATH_NESTED-d-top-switch-path:downstream"
# sever（分离型）：A 路 D_TOP 区间套精确切换路径与 anchor 趋势底仓本被当作可叠加的精度提升半步（553 §五开放轴：区间套级联减滞后提精度），
# L3 暴露 D_TOP 链 @ a0=segment 全面坍缩（21/24 零切换）——切换路径退化为死代码，anchor 仍锁核心、低级别空腿仍失血。
# 切断 D_TOP 区间套精确切换这一吃跌实现路径 @ a0=segment（有效域空集 → 净负，比 OFF/ANCHOR 都差）。
# scope=downstream：被切断的是 D_TOP completed 精确识别 → 核心翻空切换整条链。
depends_on:
  - '553'   # A 路设计开放轴——本号是 553 §五 A 路根本张力的 L3 裁决（区间套精度能否避强牛误触发）
  - '545'   # emergent_top 方向锚不稳——A 路 D_TOP 顶部识别依赖 emergent_top，方向锚不稳是链坍缩的上游成因之一
related:
  - '552'   # anchor 吃涨 L3——A 路在 anchor 之上加 D_TOP 切换；坍缩后退化为 anchor 锁核心 + 低级别空腿失血混合态
  - '547'   # cascade 级别错配——547 级别归属修对「翻哪一级」，A 路试图修「切换时机精度」，@ a0=segment 仍坍缩
  - '539'   # 清仓判据 regime 门控——D_TOP 切换 = 顶部该翻空的精确识别，@ a0=segment 无法操作
  - '【memory】project_t_cross_level_coupling_falsified'   # 低级别空腿强牛失血同构——A 路退化态的失血腿与之同族
  - '【memory】project_pcf_1s_a0_source_collapse'          # 区间套链坍缩同构——pcf 高层 source 坍缩 = D_TOP 高层链坍缩同一结构性现象
  - '【memory】project_pcf_pending_locate_collapse_fix'    # pending_locate 坍缩修复——坍缩的结构性根因诊断（前景不乐观的依据）
  - '【memory】feedback_filter_bank_metaphor_prove'        # a0=stroke 更细尺度开放轴——滤波器比喻更细尺度可能让区间套链贯通更多层
tensions_with: []
---

# 554 号：A 路 D_TOP 区间套精确切换 L3 否证——链坍缩，有效域 ≈ 空集 @ a0=segment

**认识论**：A 路本体 L0（实装成功，113 单测绿 bit-exact）；**A 路 D_TOP L3 否证 = 有效域 ≈ 空集 @ a0=segment**（区间套链坍缩，n_a_path_switches 21/24 = 0）；这是 553 §五 A 路开放轴的 L3 裁决。**否证的是『区间套 D_TOP 精确识别 @ a0=segment』这一形式**，不是『对称双吃目标本身不可达』。a0=stroke 更细尺度未试（开放轴，前景不乐观）。

## 一、背景：A 路本体实装成功，L3 回测否证

A 路本体（最高级别区间套 D_TOP 精确切换 + anchor 条件门控 + 平空对称）实装成功：worktree `3cc8c49b7a`，commit，113 单测绿，bit-exact。但 **L3 回测否证**。本号是 553 号 A 路开放轴（§五 A 路根本张力）的 L3 裁决。

L3 数据：worktree `wf_2f91805d-f6f-1`，a0=segment，env `T_A_PATH_NESTED`，743s 全绿。

## 二、L3 矩阵（A_PATH ON，Structural）+ 对照

> 列：A_PATH S（A 路开启）/ OFF S（关闭）/ ANCHOR S（仅 552 anchor）/ BH。单位均为 %。

| 标的 | regime | A_PATH S | OFF S | ANCHOR S | BH |
|---|---|---:|---:|---:|---:|
| OKLO | 强牛 | +173.8 | +173.8 | **+458.9** | +307.1 |
| BTC | 强牛 | +50.4 | +37.3 | **+1136.9** | +1380.4 |
| ES | 强牛 | −11.9 | −8.3 | **+398.2** | +594.3 |
| GC | 强牛 | −11.3 | −28.3 | **+151.3** | +257.3 |
| QQQ | 强牛 | −21.6 | −0.7 | **+108.6** | +174.6 |
| CL | 震荡 | −3.9 | +22.2 | −71.7 | +28.2 |
| BRN | 震荡 | −25.2 | −9.8 | −30.8 | +87.4 |
| DX | 震荡 | +0.8 | −1.0 | −10.5 | +4.1 |

**n_a_path_switches：21/24 cell = 0，其余 ≤1。D_TOP 区间套链全面坍缩。**

## 三、A 路 D_TOP L3 否证（四判据）

| 判据 | 内容 | L3 裁决 |
|---|---|---|
| 判据1 | 识别精度（D_TOP 能精确识别真顶部、强牛零误触发） | **否证**——强牛 A_PATH ≪ ANCHOR：BTC −1086pp / OKLO −285pp / ES −410pp / GC −163pp / QQQ −130pp |
| 判据2 | 顶部翻空吃跌（cc 级别 short_pnl 显著为正） | **否证**——cc 级别 short_pnl ≈ 0，switches ≈ 0，无顶部翻空吃跌 |
| 判据3 | 对称双吃（吃每级别涨跌幅绝对值） | **否证**——同判据2，跌那一半未吃到 |
| 判据4 | 链贯通（D_TOP completed 切换链能在真实数据触发） | **确认坍缩**——21/24 零切换 |

**结论**：A 路 D_TOP（区间套精确切换 @ a0=segment）有效域 ≈ 空集（formalization-validity-domain：有效域 ≪ 定义域）。判据1 是关键——强牛标的 A_PATH 远不及 ANCHOR，证明 D_TOP 切换不仅没吃到跌，反而比纯 anchor 死扣还差。

## 四、退化态修正（对抗审查关键）

**A 路退化不是干净的 ANCHOR 死扣。** 这是对抗审查的关键论点——若 A 路退化为干净 ANCHOR（链坍缩 = 切换从不触发 = 等价 ANCHOR），则 A_PATH 应 bit-exact 等于 ANCHOR。但矩阵显示 A_PATH ≠ ANCHOR 且远差于 ANCHOR。

A 路退化态 = **「anchor 锁核心 + 低级别空腿照常失血」混合态**：
- D_TOP 顶部切换链坍缩（21/24 零切换）→ 核心翻空从不触发 → 核心被 anchor 锁死（enter=1 / flip=0，瘫痪）；
- 但低级别空腿（539/547 短差腿）**照常运作、照常失血**——强牛标的被低级别空腿放血。

证据（A_PATH vs OFF/ANCHOR 的偏离）：
- **仅 4/24 巧合 = OFF**（若纯瘫痪应等于某基线）；
- **15/24 偏离 >10pp**——A_PATH 既不等于 OFF 也不等于 ANCHOR，是第三种混合态；
- 强牛 A_PATH 普遍劣于 ANCHOR（BTC +50.4 vs +1136.9、ES −11.9 vs +398.2、GC −11.3 vs +151.3、QQQ −21.6 vs +108.6）——anchor 的吃涨被低级别空腿失血侵蚀，而 D_TOP 切换又没补上吃跌。

**安全（无 OKLO 穿仓 = 避 553 cascade −106.6💥）来自瘫痪，不是精度。** A 路确实避开了 cascade 的强牛穿仓（D_TOP 切换从不触发 → 不会像 cascade 那样误翻空毁底仓），但这一「安全」是核心被锁死（enter=1 / flip=0）的副产物，不是 D_TOP 精确识别的功劳。瘫痪的核心不会误翻空，也不会精确翻空——它根本不动。

## 五、根因（重大 L3 发现）：第27课区间套三必要条件 @ a0=segment 几乎从不同时满足

缠论第27课区间套定位（精确大转折点寻找程序）要求三个必要条件**同时满足**才确认「走势完成」：

1. **链贯通**：区间套自上而下逐级收窄，背驰段逐级确认贯通到精确点；
2. **本级趋势**：被定位级别本身处于趋势（非盘整）；
3. **力度衰减**：背驰（力度衰减）在被定位级别成立。

**在 1min a0=segment 真实数据上，这三个条件几乎从不同时满足。** n_a_path_switches 21/24 = 0 是这一坍缩的可观测后果：D_TOP completed（区间套链贯通到最高级别顶部）这一事件在该分辨率下几乎不发生。**精确识别「走势完成」@ a0=segment 在 1min 分辨率不可操作。D_TOP 有效域 ≈ 空集。**

这与 [[project_pcf_1s_a0_source_collapse]]（pcf 高层 source 坍缩——高层 candidate 在时间上罕见，链坍缩 91-96%）和 [[project_pcf_pending_locate_collapse_fix]]（pending_locate 坍缩根因 = segment 瞬时 nf 抢先武装）是**同一结构性现象**：区间套链在高层 @ 当前分辨率坍缩。

## 六、形式 vs 目标（声明边界，formalization-validity-domain）

**否证的是形式，不是目标。** 编排者已在 553 纠正过「对称双吃不可达」的声明膨胀（formalization-validity-domain 模式 3：定义域=有效域假设）。本号严格遵守该边界：

- **被否证（确证）**：「区间套 D_TOP 精确识别 @ a0=segment」这一**实现形式**的有效域 ≈ 空集。这是与该形式定义域等大的 L3 验证（8 标的 ×3，21/24 零切换）。
- **未被否证**：「对称双吃目标本身（吃每级别涨跌幅绝对值）」——区分形式（@ a0=segment 区间套精度）vs 目标。声称目标不可达需与目标定义域等大的验证，而目标在更细分辨率（a0=stroke）从未实装。

故本号声明：**A 路 D_TOP @ a0=segment 否证（有效域 ≈ 空集），不声称对称双吃目标在全分辨率不可达。**

## 七、吃跌三条路全否证（吃涨 anchor 仍是死锁解确证部分）

对称双吃「跌那一半」的三种实现形式，至此全部 L3 否证：

| 路线 | 实现形式 | L3 否证 |
|---|---|---|
| B 递归 anchor | 深层空腿全程持空不切换 | 552 §八（BTC −87.4% / −45% 崩涨） |
| cascade flip | emergent_top.completed 粗识别触发核心翻空 | 553（THREE-HALF 8/8 ≤ anchor，OKLO −106.6💥 穿仓） |
| A 路 D_TOP | 最高级别区间套精确切换 @ a0=segment | **本号**（链坍缩 21/24 = 0，退化混合态比 OFF/ANCHOR 都差） |

**吃涨侧 anchor 强牛 5/8（552）仍是死锁解的确证部分。** 三条吃跌路全否证不否定 552 anchor 吃涨的真解地位——吃涨侧（核心死扣持多）确证有效，吃跌侧（每级别精确骑下跌走势切换）@ 当前已试形式与分辨率全部否证。

## 八、开放轴（吃跌最后未测）：a0=stroke 更细尺度

唯一未测的吃跌路径 = **a0=stroke**（[[feedback_filter_bank_metaphor_prove]] / 526 号 A0Source，笔级底座）：

- 笔级底座递归更深（recL 从 5-6 → 8-10 层），区间套链可能贯通更多层 → D_TOP 切换链可能不坍缩；
- 呼应滤波器比喻：更细尺度 = 更高奈奎斯特观测分辨率，第27课三必要条件可能在更细尺度同时满足。

**但前景不乐观**：pcf 内存诊断显示高层链坍缩可能是**结构性**的（[[project_pcf_1s_a0_source_collapse]]：1s 救不了 source 坍缩，ES 1s 11.77M bar seg_root 85.6% vs 同窗 1min 91.3%，85-91% 带；尺度不变性第二例——塔变高但高层 candidate 时间罕见不变）。若坍缩是结构性的（尺度不变），a0=stroke 也救不了 D_TOP 链。**未测，不下结论**——这是吃跌侧最后一根未测的轴。

## 九、张力检查结论

**无不可分层解决的矛盾，不新建张力记录。** A 路 D_TOP 否证与 552 anchor、553 cascade、cross_level 全部分层一致：

- 与 553 cascade 分层：cascade 粗识别强牛**误翻空毁底仓**（穿仓），A 路精确识别尝试反而**链坍缩从不切换**（瘫痪）——两个相反的失败模式，但同指向「@ a0=segment 切换时机识别不可操作」。cascade 太敏感（任何 completed 即翻），A 路太严格（三必要条件几乎从不同时满足）；中间没有可操作的精度窗口 @ a0=segment。
- 与 552 anchor 分层：A 路退化态保留 anchor 锁核心，但叠加低级别空腿失血 → 比纯 anchor 差（sever：D_TOP 切换路径被切除，anchor 路径保留）。
- 与 [[project_t_cross_level_coupling_falsified]] 同族：A 路退化态的低级别空腿失血 = 逆 regime 固定方向腿在强牛被套，regime 结构税。

对称双吃目标可达性（a0=stroke，§八）是**开放轴**（未试），非已结算矛盾，不上浮。

## 十、影响声明

- **A 路 D_TOP（T_A_PATH_NESTED，最高级别区间套精确切换 @ a0=segment）有效域 L3 否证 = 空集**（n_a_path_switches 21/24 = 0，退化混合态比 OFF/ANCHOR 都差，强牛 A_PATH ≪ ANCHOR）。**确证。**
- **A 路退化态 = 「anchor 锁核心 + 低级别空腿照常失血」混合态**（非干净 ANCHOR 死扣，4/24 = OFF，15/24 偏离 >10pp）；安全来自瘫痪（enter=1 / flip=0）不是精度。
- **根因（重大 L3 发现）**：第27课区间套三必要条件（链贯通 ∧ 本级趋势 ∧ 力度衰减）在 1min a0=segment 真实数据几乎从不同时满足——精确识别「走势完成」@ a0=segment 不可操作。
- **否证的是形式（@ a0=segment 区间套精度），不是目标（对称双吃）**——区分形式 vs 目标，遵守 553 编排者已确立的声明边界（formalization-validity-domain）。
- **吃跌三条路全否证**：B 递归 anchor（552 §八）/ cascade flip（553）/ A 路 D_TOP（本号）。吃涨 anchor 强牛 5/8（552）仍是死锁解确证部分。
- **开放轴**：a0=stroke（526 A0Source）更细尺度，呼应滤波器比喻——区间套链可能贯通更多层；但 pcf 高层链坍缩可能结构性（尺度不变），前景不乐观，未测。
- 未改主树生产路径（全 worktree 分支 + flag，113 单测绿 bit-exact，worktree 3cc8c49b7a 本体 / wf_2f91805d-f6f-1 L3）。
- **更新 553**：§五.3 成败判据「A 路 = 对称双吃可达性判决场，未实证」→「A 路 D_TOP @ a0=segment 已 L3 否证（链坍缩，有效域空集，见 554）；可达性判决场移至 a0=stroke 更细尺度」。
- **新增**：`.chanlun/genealogy/settled/554-a-path-d-top-nested-precision-falsified-empty-domain.md`（本文件）。
