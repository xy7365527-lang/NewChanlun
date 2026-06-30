# Codex 异质审 — E1 a0=Stroke 否证结论（任务 #69）

生成：2026-06-30  模型：codex (gpt-5.5) diagnose read-only  调用：无 --json（566a 规避）
被审：commit 8f9be6f0ff backtest-e1-a0-stroke-20260630.md（E1 否证 = a0=Stroke 灾难性劣化）
源：/tmp/out.txt（817KB 完整）。**codex 子进程完成但工位未 mailbox 回传，Lead 从 /tmp 直接消费并落盘（防 verdict stranded 无限等待）。**

## 总判定：(B) FALSIFICATION DEFECTIVE（否证有缺陷 → 降级为窄结论）

**保留窄结论**：a0=Stroke 在 long-only apply_bsp 引擎下（BTC/CL 1min）灾难性劣化。
**禁止外推**：不得提升为"a0=Stroke 无 alpha"；不得仅凭此否决 E2/1s+笔路线。
**承重纠正**：−100.0% 本身不是会计 bug 的证据（攻击点3 robust），但结论仍被操作层敏感性 + 对象漂移混淆。

## 4 攻击点逐条裁定

| 攻击点 | 裁定 | 等级 | 理由 |
|--------|------|------|------|
| 1 引擎混淆 | **defective** | L1/L2 | a0=segment→stroke 同时改变整个涌现树结构 + BSP 数 + 每事件仓位翻动——三者非独立。单变量隔离未真正隔离 a0，混入"引擎对信号密度的敏感性"第二变量。是 apto_bsp×高密度信号 的否证，非纯 a0=笔 的否证 |
| 2 基线断裂 | **defective** | L1 | filter-spec +664% vs 本引擎 +70.9% 差一量级 = 对象漂移信号（filter-spec 实验设计与当前引擎非同一对象）。"−100%低于任何正基线故否证不依赖阈值"的 dismissal 不稳健 |
| 3 −100%归零 | **robust** | L0 | 针对"会计bug致人为归零"的反驳，−100% 是真实交易亏损，非 sizing bug。这一条 backtest-e1 站得住 |
| 4 可外推性 | **defective** | L2 | 2标的×1引擎只支撑"stroke a0 在此 apply_bsp 执行层在 BTC/CL 1m 劣化"，不支撑否决整个 E2/1s+笔路线。下游推论需匹配的带门控引擎测试 + 成本感知执行 + 更广标的/窗口 |

## 下游推论（Lead 据此决策）

1. **barspec 1s 子树（#63/#65-#67）不停**：codex 明确"不得仅凭 E1 否决 E2/1s 路线"。E1 否证有效域 = long-only apply_bsp 引擎，barspec 的 1s 数据通路升级独立于此（数据粒度参数化对任何引擎都是前提）。275 局部依赖：barspec 不消费 E1 的否证输出。
2. **E2 设计需修正**：若续推 a0=笔，必须用带级别/方向门控引擎（rec_engine LongEntry::CrossLevel 族）而非 long-only apply_bsp——否则重复同一混淆。这是 E2 的前置设计约束（待编排者方向确认或 filter-spec 重设计）。
3. **filter-spec 对象漂移**（攻击点2）= 已记录 pending 659 的同源问题（filter-spec 报告与当前引擎非同一对象）。codex 攻击点2 独立印证 659。

## 认识论等级

整体 L2（双标的真实数据，codex 源码级对抗审查 = L1 管线 + L0 逻辑）。codex 未跑测试（只读沙箱）——其裁定是源码级 + 逻辑级对抗，非独立重跑 L2。

## 影响声明

E1 否证从"E2/1s 路线失去依据"（过度外推）降级为"a0=Stroke 在 long-only apply_bsp 劣化"（窄有效域）。barspec 1s 子树继续。E2 若推进须换带门控引擎。关联 pending 659（filter-spec 对象漂移/范畴错误）。
