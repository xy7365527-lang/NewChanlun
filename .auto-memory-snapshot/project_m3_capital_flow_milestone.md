---
name: m3-capital-flow-milestone
description: M3阶段性目标：资本流转流量/流速的OU估计+宏观持仓表设计
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## M3 宏观持仓表（阶段性目标，不阻塞M1/M2）

### 已完成的基础
- 方向 ✅：残差符号（强持续性，L2验证）
- 加速度 ⚠：残差MACD背驰（聚合无边际但高幅背驰命中4大regime）
- 流量 ❌：成交量proxy否证 + COT净持仓否证（7/7无一领先）
- 持续性/流速 ✅：残差缠论递归涌现L3多级别结构

### 待探索（M3时做）
1. **OU流量估计**：flow = θ×r，θ从残差自相关估计，不需外部数据
   - θ大=资本流动快（市场高效），θ小=有摩擦/管制
   - θ的regime变化点是否对应已知危机/转折
2. **持仓表设计**（Soros/Druckenmiller框架）：
   - 汇率腿（6E, DX）：残差方向决定
   - 折叠通道腿（GC, CL/BRN spread）：全局C↔M / C→P方向
   - 指数/个股腿（ES + M1+M2选出的标的）
3. **跨国总拓扑**：
   - 各国K4不是孤立的，是总拓扑的局部切片
   - 金/油是全局折叠（连接所有经济体），汇率是局部连接
   - R(不动产)是唯一局部顶点（锚定点）
   - 金计价下K4→K3（自由度3→2，自测不可能性）
4. **COT精细口径复验**：Legacy Non-Commercial已否证，Managed Money/Leveraged Funds待测

### 已完成的研究文档
- docs/architecture/capital_flow_ontology.md（548行）
- analysis/cross_national_fx_deep_dive.md
- analysis/verify_capital_flow_4d.py
- analysis/cot_capital_flow_verification.md
- /tmp/chanlun-soros/（用户之前的研究包，需对齐到当前体系）

**Why:** M3是M1+M2之上的宏观层，不阻塞但需要提前记录方向
**How to apply:** M1严格收尾 → M2选股验证 → M3宏观持仓表

Related: [[project_omega_research]], [[project_k4_fold_channel_model]]
