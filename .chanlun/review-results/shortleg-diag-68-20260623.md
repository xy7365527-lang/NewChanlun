# #68 诊断：CL 做空腿巨亏真根因（Lead 务实诊断，工位卡死绕过）

> 编排者两洞察落地诊断（睡前授权自主）。读 rec_engine.rs g_pair/close_short_leg/pair_stop_loss_step。
> 认识论等级：L0（源码逻辑诊断）+ L3（#57 8标的回测数据）。

## 背景：#57 L3=7/8净正，CL−66.6%唯一异常
CL short_pnl=−71932（3653 opens / 2867 stops）vs GC short_pnl=−2597（同~4000次但几乎不亏）。

## 编排者洞察2（止损现金归还）= 会计正确，已验证
- open_short_leg（line 1196-1197）：`free += m*c`（卖空得现金）
- close_short_leg（line 1224）：`free -= units*c`（买回付现金）
- short pnl（line 1215）：`units*(basis - c)`（basis=开空价，跌则赚）
- 净 free 流 = m*c − units*c = units*(basis−c) = pnl（units=m），**守恒正确**。
- ⟹ **short_pnl −71932 是真亏，非会计 bug**（编排者洞察2 排除会计问题）。

## 平空级别匹配 = 已实装（非 CL 根因）
g_pair line 1458-1468：买点 `view.buy[k]` fire → 平级别 k 空腿（同级别反向买卖点平，对称）。**平空级别匹配已正确实装**。

## ★CL 巨亏真根因 = 开空腿判据太松（无真回调/链破坏门控）
g_pair line 1470-1488 卖点分支：
- 核心多腿平仓（line 1472-1482）：**有 churn 门控**（`may_close_core = !is_core_long_level || chain_break`，链破坏才动核心）✓
- **开空腿（line 1483-1486）：`卖点 fire ∧ 空腿空 → open_short_leg`，无门控**——每个卖点都开空腿 ✗

CL 震荡 → 卖点频繁（3653 次）→ 空腿开在**假突破/假回调**（次级别小下跌非真转折）→ 假回调无真底 → 买点不 fire → 涨破进场 ZG 止损（2867 次，pair_stop_loss_step line 1426-1432）→ 累积 −71932。

**对比趋势标的（GC short −2597）**：回调浅且少 → 卖点少 → 开空腿少 → 假突破少 → 自然不亏。**趋势 alpha 成功是 regime 碰巧（少开空），掩盖了开空腿判据太松的 bug；CL 震荡暴露之。**

## 修复方向（#68 实装，编排者洞察1"判定快且准"+ 递归自相似 no-hardcode）
**开空腿 line 1483 加区间套链破坏/真回调判定门控**——假突破不开空腿，如核心多腿 line 1473 的 chain_break 门控对称：
- 真回调/链破坏（次级别突破中枢+回试不回=第三类卖点形态，真下跌涌现）→ 开空腿（会有买点平，赚回调跌）
- 假突破（次级别小下跌未破中枢/回试回）→ 不开空腿（避免涨破止损）
- 自相似：开空腿门控与核心多腿 churn 门控同构（chain_break 驱动），跨级别同构，零 if regime/level==N。

**验证判据**：CL short_pnl 从 −71932 改善（假突破不开→止损↓）→ CL 转正 → 8/8 净正（做空腿赚=编排者"基本稳"）；趋势标的保持；churn↓零强平守住；bit-exact OFF。

## 谱系/影响
- 567（冻结空腿，否定线零强平已验）；[[project_long_short_dual_open_eat_both]] 纲领。
- 影响：g_pair 开空腿分支（line 1483）加门控；不改平空级别（已对）+ 止损会计（已对）。
- no-escalate：无概念矛盾，开空门控是 line 1473 churn 门控的对称扩展，可处理。
