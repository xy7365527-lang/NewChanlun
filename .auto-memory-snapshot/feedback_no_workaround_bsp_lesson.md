---
name: no-workaround-bsp-lesson
description: 惨痛教训：不消费BSP导致整个操作层是workaround堆叠，promote/spawn/flip/chain全是替代品，消费BSP后全部消失
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 2026-06-20 BSP不消费=一切workaround的源头

整个session追到底的教训：递归引擎（rec_engine.rs）计算了11281个缠论买卖点（type1/2/3 买卖各数千个），然后**全部丢弃**，操作层靠走势方向变化（chain[0]反转）做决策。

结果：
- 843个买点摆在那里没消费 → 空头骑120万bar穿越牛市 → -184%
- 50%空头不回补 → 不是信号问题，是消费问题
- promote/spawn/flip/emergent门控/clear_root/走势跟随 → 全是因为不消费BSP而造的workaround

**教训**：每多加一层间接机制（promote、spawn、flip、chain、extract_chain），就多一层可能出错的地方。如果直接消费BSP，操作跟BSP一一对应，不需要任何间接机制。

**正确形式**：每个T实例消费自己级别的BSP，买点→做多/平空，卖点→做空/平多。多级别同时运行=多重赋格。不需要中间层。

**Why:** 编排者反复强调"绝不能做任何workaround"。这次是最大的实例——整个操作层都是workaround堆叠，因为最底层（BSP消费）没做。
**How to apply:** 任何设计先问"这是不是在直接消费BSP？"如果需要间接机制（promote/spawn/flip/chain），先质疑：为什么不能直接消费BSP做操作？间接机制=潜在workaround。

Related: [[no-asset-specialization]], [[bidirectional-always-in]], [[recursive-nested-fugue]]
