---
name: alpha-diagnosis
description: M1三层alpha归因：纯信号≤BH，降成本是负alpha源，MACD门控治标不治本。提款机bug修复后全面无alpha
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 2026-06-06 三层alpha归因（L2决定性结果）

### 提款机bug修复
`if profit > 0` 截断在20个文件39处被修复。修复后所有"亮眼"回测数字崩塌。

### 三层归因结果

| 标的 | 纯进出场(A) | 含降成本(V0) | 精修门控(B) | BH |
|------|------------|-------------|------------|-----|
| QQQ | +40.59% | +23.69% | +26.03% | +174.64% |
| OKLO | +242.61% | +114.08% | +179.36% | +251.30% |

1. **纯进出场层无显著alpha** — OKLO接近BH(超额-8.69%)，QQQ差距大(-134%)
2. **降成本是负alpha源** — OKLO降成本亏128个百分点，QQQ亏17个百分点
3. **MACD门控有改善但治标不治本** — 通过率55-63%，减少失血但不翻正

### 挣股数阶段
FSM EARNING_SHARES状态已实现（金额守恒），但有效域为空——cost_basis降不到0，触发条件从未满足。

### 下一步
需先修进出场信号层（让纯信号跑赢BH），然后再叠降成本。

**Why:** 提款机bug掩盖了整个M1回测基线的虚假性
**How to apply:** M1所有历史回测数字不可信。新基线必须从零降成本版开始

Related: [[quant-system-progress]], [[costreduction-moneyprinter-bug]]
