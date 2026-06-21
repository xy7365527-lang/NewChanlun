---
name: located-direction-discovery
description: 区间套located武装方向发现：当前自下而上(错)应为自上而下(对)，但1min a0下高级别candidate稀缺导致source坍缩segment
metadata:
  type: project
  originSessionId: cowork-2026-06-14
---

## 2026-06-14 session关键发现链

### 1. 信号丢失12机制诊断 → isolated_fugue.rs森林引擎
- 栈→森林、per-voice acted、Type2接入
- P1 1/8（森林放松约束反而更差——栈纪律是特征非bug）

### 2. why_not_profitable.md 五维度诊断
- 方向正确率@bar ≈ 随机 → 用户纠正测法 → 配对重测70-83%强有效
- **摩擦地板按级别切割**：segment幅度0.02-0.19%<摩擦，recL2+ 0.23-75%>摩擦
- 99%交易在segment = 摩擦地板下噪声
- 高级别type1独占利润

### 3. nested_interval_fugue.rs (nif3) 固定floor
- min_trade_ladder=3固定参数 → OKLO+922%/BTC+1287% 但P1 1/8
- 固定参数=regime函数（强趋势改善震荡劣化）= 不严格

### 4. positioning_chain_fugue.rs (pcf) 自下而上→自上而下
- **发现located武装方向反了**：每层独立检测candidate→检查全层对齐=自下而上
- 区间套应该是：高级别candidate→级联武装下面所有层→低级别confirm=自上而下
- 实装了自上而下级联（cascade_arm）→ source覆盖3-4层（改善10pp）
- **但source仍83-87%坍缩segment**：根因=1min a0下高级别candidate本身稀缺
- 自下而上和自上而下两版夹逼证明：坍缩是candidate频率结构决定的

### 5. 下一步：1秒a0
- 用户选择1s a0方向（更细粒度→更多递归层→高级别candidate可能不那么稀缺）
- 用NautilusTrader框架+Databento数据
- 之前有1s实验(`1s_a0_nest_coverage_results.md`)双否证，但那是旧模式(fusion_v+自下而上)

**Why:** 这条链是整个session的核心推进——从信号丢失→摩擦地板→固定floor→located方向→candidate稀缺→a0粒度
**How to apply:** 下个session从1s a0 + NautilusTrader + 自上而下pcf引擎开始

Related: [[recursive-nested-fugue]], [[concept-movement-chain]], [[session3-fusion]]
