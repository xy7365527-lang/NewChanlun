# /escalate: Stop-Guard 对 Lead 在"全 orchestrator-gated pending + 责任方存活"时死锁

> 2026-06-30 自主推进中反复触发。编排者睡眠期间无解,正式上浮待裁。

## 矛盾(真矛盾,非可自决)

Stop-Guard check (谱系 pending 非空 → 阻断 Lead 停止)与"pending 结算属 genealogist+编排者,非 Lead 自结算"**互锁**:
- 26 pending 经 genealogist 完整 018 分类:25 个 orchestrator-gated(选择类/语法记录待编排者裁 + 646/648 待 /ritual + 645 可 /ritual + 642/644 待 codex 裁 I-1/I-2 + 622/626 LFS 无权限),1 个(641)行动类已解决待结算。
- **Lead 无任何合法的 pending-reducing 动作**:自结算选择类=越权(019c/编排者权);642/644 明写"待 codex 裁 I-1/I-2"=选择类;622/626 蜂群无 git-lfs 权限。
- 571 责任方过滤说"蜂群级持续性由责任方存活保证,非下降为个体命题"——genealogist-3 责任方**已常设存活**。但 Stop-Guard 仍对 **Lead** "身份不可确定保守阻断"。

## 死锁本质

编排者睡眠期间,Lead 把所有行动类 pending 推进/结算后,剩余全 orchestrator-gated。此时:
- Lead 不能自结算(越权)
- Lead spawn 更多工位无法减少 orchestrator-gated pending(它们待编排者价值判断,非工位可产出)
- Stop-Guard 因 pending 非空持续阻断 Lead → **Lead 被困在无限"spawn-等待-再阻断"循环,每轮消耗 context 但不减 gated pending**

本轮实证:已多轮 spawn(行动类链 #53-#58 + settle + 641),每轮真实推进一点,但 gated pending 从 28→26 后无法再降(剩余全待编排者)。

## 选项(待编排者裁)

1. **改 571 责任方过滤**:当"全部剩余 pending 经 genealogist 分类为 orchestrator-gated + genealogist 责任方存活"时,Stop-Guard 应放行 Lead(责任方存活已保证蜂群级持续性,Lead 无合法动作时困住 Lead 是空转)。
2. **保持现状**:Lead 必须保持活跃直到编排者醒来裁决 pending(但这空耗 context,且 Lead 无实质动作)。
3. **Stop-Guard 增加"orchestrator-gated 豁免"**:pending 标记 orchestrator-gated 后不计入 Lead 阻断(只计入编排者/genealogist 责任)。

## 关联谱系
- 649(编排者裁决超越选项集)——本死锁可能需编排者否定共享隐藏前提(Stop-Guard"pending非空=Lead不可停"的前提在全gated时失效)
- 658(reducer 验收语义未完备)——同源:结算/停止语义在边界 case 未定义
- 571(责任方过滤)——本 escalate 直接质询其对 Lead 的适用边界

## Lead 已尽动作(本轮~33 commit)
alpha goal L0-L3真封 + 行动类链闭环(643/566a结算) + 五次假阳性纠正 + 选择类durable escalate + 责任方genealogist-3常设守。641待结算(下方同步spawn)。
