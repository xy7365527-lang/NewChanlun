# 38课位置分支主腿移植 × Seq1 子腿组合 — 五标的 L3 判决

任务（2026-06-11）：① seq38-submode 并 main（rebase 零冲突）；② 38课位置分支
（"不跌破第一段低点 × 次级别确认 → 买回"，审计 §4e 唯一缺失项）移植到主 REV
腿闭腿集（`rev_seq_nobreak`，reason=11）；③ 2×2 因子（nb × Seq1 子腿）在
OKLO/BRN/CL/BTC/PINS 上判决。

脚本：`analysis/seq38_main_backtest.py`
数据：`analysis/data_cache/seq38_main_backtest.json`（git-ignored，本文档为判决记录）

## 1. 结论

| 标的 | ht 基线复利 | nb Δ(base) | seq1 Δ(base) | nbseq1 Δ(base) | nb 次数 | sub 开/闭 |
|------|-----------|-----------|-------------|---------------|--------|----------|
| OKLO | +1800.0% | **−245.2pp** | +16.0pp | −248.6pp | 296 | 3/2 |
| BRN  | +598.8%  | **−210.1pp** | **+39.0pp** | −215.3pp | 1367 | 255/230 |
| CL   | +65.8%   | **−47.8pp**  | +1.2pp  | −48.3pp | 2547 | 309/277 |
| BTC  | +6.6%    | +4.3pp   | **−20.0pp** | +3.7pp | 3726 | 723/665 |
| PINS | +52.3%   | **−42.7pp**  | +3.8pp  | −43.0pp | 684 | 42/38 |

**双轴判决**：

1. **nb（主腿位置分支）4/5 否证**。"不跌破第一段低点 × 次级别确认"作为主
   REV 腿闭腿路径在 OKLO/BRN/CL/PINS 全负（−43 ~ −245pp），且 MDD 一致恶化
   （OKLO −43.9→−47.6 / BRN −47.0→−51.9）。机制读数：nb 是**提前兑现出口**
   ——在 ZD 触线/同锚 Buy1 之前截断 REV 空腿，损失兑现深度（ZD兑现×存活率
   公式的存活率被人为截短）。唯一正域 BTC（+4.3pp，MDD −74.4→−72.4）：
   REV 循环本身无 alpha 的标的（在册判决），早闭腿 = 少亏。
2. **seq1（Seq1 子腿上 ht 基线）4/5 正**。OKLO+16.0 / BRN+39.0 / CL+1.2 /
   PINS+3.8，唯 BTC −20.0pp。与 V2of 基线判决（OKLO+10.2/BRN+4.1）方向一致
   且 BRN 增量放大——ht 出场（持仓更长）给子腿更多 REV 窗口。BTC 负域与
   nb 正域是同一枚硬币：REV 窗口在 BTC 无结构 alpha，窗口内再嵌套子腿放大亏损。
3. **组合无交互拯救**：nbseq1 ≈ nb 主导（nb 截短 REV 窗口连带压缩子腿域：
   BRN sub 开 255→24）。
4. **新在册最优候选**：V2oa25_ht_seq1（BTC 除外的 REV 正域标的）。

## 2. 定义依据

- 38课:36（向下段镜像）"1、不跌破第一段低点，重新买入"；答疑:296"这个
  '不跌破'是靠次级别判断吗？——对，需要该段内部结构的确认"⇒ 判据 =
  `low_since_open > seg1_low ∧ sub_confirm(k, Buy)`（非纯几何触线）。
- seg1_low = 开腿时冻结的 `rev_run_low`（上次配对闭腿以来 close 运行最低）
  ——Sequence38 子腿 `seg1_low` 的主腿同构，close 分辨率是诚实近似残留。
- 优先序：事件证据（T7/T6/T5/T2c）之后、几何触线（ZG/ZD）之前。

## 3. 边界条件

- nb 否证有效域 = **主 REV 腿**（V2oa25_ht 基线、floor=2、零摩擦磁带）。
  同一判据在 Sequence38 **子腿**三岔中是正贡献成分（L2 全正）——子腿≠主腿
  的范畴边界，正是审计 §6 预注册的边界条件（"若严格形式优于近似不可泛化
  （子腿≠主腿/master），替换方向预期失效"）——本判决触发该边界。
- 若 D3 磁带升级为段端点事件流（seg1_low 从 close 分辨率变为段低点本体），
  nb 的否证需复验——当前判据含 close 分辨率近似。
- BTC 双反向依赖"BTC REV 循环无 alpha"的在册判决——若 BTC 数据域变化
  （如剔除 2017/18 单边），两轴符号可能翻转。
- seq1 的正增量在 CL 仅 +1.2pp（≈噪声级）——L3 正结论的强支撑是
  OKLO/BRN/PINS 三标的。

## 4. 下游推论

- 审计 §4e 缺失项关闭：38课位置分支已实装并判决——主腿形式否证，缺失不再
  是缺口而是**有判决的否定**。主 REV 腿闭腿集（T7/T6/T5/T2c/ZG/ZD）经受住
  38课程式位置分支的挑战，在册集合的穷举性获得新证据。
- "在 confirmed 事件之上加层"失败家族扩员：SC 六位、R1、R3、SubAny 之后，
  nb 是第五例——但机制不同（前四例是**等待/过滤**，nb 是**提前出口**），
  统一形状修正为：外加择时层（无论延迟还是提前）破坏买卖点自带的 regime 信息。
- V2oa25_ht_seq1 成为 REV 正域标的的新默认候选——后续变体应基于它基线化
  （BTC 等 REV 负域标的维持 V2oa25_ht）。
- nb 截短 REV 窗口 ⇒ 压缩子腿域（BRN 255→24）——任何主腿闭腿集变更都会
  连带改变子腿的存在域，未来主腿实验必须带 sub 计数器可观测面。

## 5. 谱系引用

- Sequence38 判决（2026-06-11，seq38-submode→main，本次 rebase 并入）：子腿
  严格形式 L2 全正——本判决确认其有效域**不含主腿**。
- 严格形式审计（strict_form_audit.md §4e/§6）：缺失项来源 + 边界条件预注册。
- 次级别确认完整递归否证（SC 六位）/R1/R3 盘背消融：外加层失败家族。
- V2r 逐笔行为分解：ZD兑现×存活率公式——nb 否证的机制依据。
- master 出场模式消融（HoldTrend 四标的全正）：ht 基线来源。
- 090号（严格性）：nb 不静默 no-op（runner guard：rev_paired×Single×D3 行）。

## 6. 影响声明

- **代码**：`rev_seq_nobreak` 配置位（默认 false，O0≡P5 零接触——五标的
  V2oa25_ht 基线与在册 strict_form cell 逐键一致 PASS）+ `RevLeg::seg1_low` +
  `VoiceUnit::rev_run_low`（无条件状态跟踪，仅 nb 消费）+ reason=11 +
  `n_rev_seq_nobreak_close` 计数器 + runner capability guard + 4 变体注册
  （V2oa25_ht_nb / V2oa25_ht_seq1 / V2oa25_ht_nbseq1；V2ofSeq1 随 rebase 并入）。
- **测试**：`seq_nobreak_closes_on_held_low_with_sub_confirm` +
  `seq_nobreak_holds_when_seg1_low_broken_or_gate_off`（138 全过）。
- **分支**：seq38-submode rebase 到 main（零冲突），ff 合入，分支可删。

**认识论等级**：L3（五标的跨 regime 真实数据；nb 否证 4/5、seq1 确认 4/5）。
