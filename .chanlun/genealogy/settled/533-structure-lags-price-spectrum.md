---
id: '533'
number: 533
title: "结构滞后于价格的三层判据时效谱型——confirmed（不可达）→ candidate（滞后）→ 价格事件（即时）"
type: 概念分离
status: 已结算
date: 2026-06-12
settled_date: 2026-06-12
settlement: H1 五标的 L3 判决确立——同一 49课禁令窗口语义在三个事件层上的有效域严格递增；挂载层选择不是实现细节，是有效域声明
source: "analysis/h1_candidate_freeze_results.md §4 + analysis/osc_whitelist_elimination_research.md §2.1"
depends_on:
  - '231'   # 形式化有效域规则（有效域 ⊂ 定义域）
  - '532'   # 38课触发轴分裂（同一判据集在不同声部有效域分裂的先例）
affects:
  - "rust/src/trading/center_book.rs（pending_departure——candidate 层首个生产消费者）"
  - "rust/src/trading/config.rs（osc_candidate_freeze / V2oa25_ht_h1）"
  - "未来一切挂确认层或 candidate 层的操作判据（设计前置问题：该事件在单边 regime 中滞后多少）"
---

## 分离内容

缠论操作判据可以挂载在三个事件层上，三层在单边 regime 中的**时效严格递减**：

| 层 | 事件 | 单边 regime 中的时效 | 在册证据 |
|----|------|---------------------|---------|
| confirmed | 回抽完成确认（如 confirmed type3） | **结构性不可达**——回抽不发生 | osc僵尸腿诊断（type3 确认不可达，僵尸腿占总亏 62-82%）；SC 六位否证 |
| candidate | 次级别段结构形成（如 candidate type3 离开段） | **滞后**——价格越 ZG 在前、段结构确认在后，僵尸腿在间隙开出 | H1 五标的判决：负域缩减仅 1.7-6.8%（僵尸主体开在窗口前）；正域深回调中窗口及时（OKLO 最大僵尸腿 bar 60978 精确拦截） |
| 价格事件 | 价格与结构边界的当下比较（如新中枢 ZD>锚 ZG） | **即时** | sc（osc_shift_close）8/10 跨域有效——负域唯一有效的出口形态 |

同一个 49课"中枢完成后的向上移动"语义，挂 confirmed 层有效域为空（在册缺口），
挂 candidate 层有效域 = 深回调 regime（H1 判决），挂价格事件层有效域跨正负域（sc）。
**挂载层的选择不是实现细节，是有效域声明**——231号"有效域 ⊂ 定义域"在事件
时效维度上的具体化。

## 发生史

- osc僵尸腿诊断：confirmed type3 不可达机制定位（第一层）
- osc_whitelist_elimination_research §2.1：发现 is_frozen 挂 confirmed 而原文
  判据（49课行68）在 candidate 层——确认层/结构层错位的首次显式化
- H1 实装判决（h1_candidate_freeze_results.md）：candidate 层修复后负域仍失效
  ——逐腿核验揭示第二层（candidate）自身也滞后于价格，谱型补全为三层
- sc 在册判决回溯定位为第三层（价格事件）的存在性证明

## 推论

1. 设计任何新操作判据时的前置问题："该事件在单边 regime 中滞后价格多少"——
   滞后量决定有效域，且滞后量本身是 regime 函数（深回调 vs 浅回调单边）。
2. H1 收缩为正域增强轴（OKLO +636pp）；负域僵尸消灭需要价格事件层或
   开腿前判据（H2 力度收敛门——下一工位）。
3. 白名单消除若最终需要 per-regime 判据切换，切换量应为"回调深度/中枢振幅
   比"（研究 §3——per-center 可测结构量），不是回测盈亏符号。
