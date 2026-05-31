# Codex 异质审计 — pending-015 P1/P2/P3 — 调用失败记录

- **日期**：2026-05-30
- **模式**：diagnose（定理级对抗审计）
- **审计对象**：pending-015 §19 三命题 P1/P2/P3（缠论形态学 ≅ T_merge^τ / ker(R)）
- **触发**：challenge 模式（pending-015 提交后），结算条件#1 的执行
- **结果**：**真异质源不可达——审计未执行**

## 调用尝试与失败

| 通道 | 模型 | 结果 |
|------|------|------|
| `python -m newchan.codex diagnose` | gpt-5.3-codex | 不可用，自动降级 |
| 同上 fallback | gpt-5.2-codex | **429 insufficient_quota** |
| 直接 OpenAI Responses API | gpt-5.1 | 429 insufficient_quota |
| 直接 OpenAI Responses API | gpt-5 | 429 insufficient_quota |
| 直接 OpenAI Responses API | gpt-4o / gpt-4o-mini / o4-mini | 429 insufficient_quota |
| Gemini generativelanguage v1beta | gemini-2.0-flash | **403 Forbidden** |

**诊断**：OpenAI 账户层 `insufficient_quota`（账户级，非单模型限流，无 retry-after），GOOGLE_API_KEY 返回 403。**两个真异质源（OpenAI + Google）均硬不可达。**

## 为何不降级为 Claude 自审

§18.8 先例（2026-05-29）：Gemini API 403 时降级由 Claude 执行被**明确标注为非异质**（同模型族），其结论"作为 Claude 自审记录，不计入异质收敛"。

本工位（155号谱系）的存在论位置：**我是调用外部异质模型的代理，不是异质模型本身**。在无真异质源可达时，由 Claude 产出"P1/P2/P3 定理级判决"会是：
- **声明膨胀（090号）**：声称获得了异质否定，实际未获得；
- **异质性伪造**：把同模型族的自审伪装为异质审计；
- 违反工位铁律"不在没有上下文/没有真异质源的情况下伪造审计结论"。

故本工位**不产出 P1/P2/P3 的定理级判决**，如实汇报真异质源不可达。

## 已完成的审计前准备（上下文已构建，待异质源恢复即可执行）

完整审计 prompt 已构建并持久化于 `/tmp/codex-diagnose-pending015-ctx.md`，包含：
1. §18.8 三条已确立否证（前提，不重审）+ Codex "fiber 比 τ 等价类大" 诊断；
2. §19 选择框架全文（R = Π_分型∘Π_包含∘Π_跨度，时间增强树定义，§19.5.1 sub/super 切换，§19.5.2 四理由，§19.5.3 settle 桥梁）；
3. P1/P2/P3 完整定义 + 审查点；
4. **实现侧硬事实**（来自 a_segment_v1.py / a_online_persistence.py 实读）——这是本工位相对纯理论审计的增量。

## 实现侧硬事实（本工位读码所得，异质源恢复后作为定理级约束输入）

1. **包含处理 `_apply_inclusion` 携带 `dir_state`（序列级方向状态）**：包含合并方向（向上 max/max，向下 min/min）依赖序列历史，非分量局部属性 → 序列级非局部性在代码层确证（理由 B + §18.8 #1）。

2. **Case2 缺口触发前向 `_second_seq_has_fractal`**：检测缺口时从分型中心**向后（前向）**构建第二特征序列，仅其出现分型才确认断段 → 前向依赖在代码层确证（理由 D + §19.5.3 延迟 settle）。

3. **L78"顶高于底"不是 merge tree 不变量，而是被标准化 `_standardize_endpoints`**：结构端点违反顶高于底时，代码用**段内实际 high/low** 替换。**关键**：标准化端点可能不是任何 merge tree 临界点（是段内 argmax/argmin），这对 P1 "persistence>0 几何强制顶高于底" 的声明 + P3 可见极值对应构成定理级挑战——R 输出端点与 T_merge^τ 临界点未必一一对应。

4. **裸 merge tree 只用 sublevel（a_online_persistence.py：valley birth + saddle death，无 superlevel 峰配对）**：与 §19.5.1 "上段找峰=superlevel" 的增强需求 γ 直接冲突——当前实现的裸树缺一半信息，§19 声称纳入 γ 但 a_online_persistence.py 实装未含 superlevel。

> 这四条是本工位对审计的实质贡献（即使异质源不可达），它们把 §19 的三个声称转化为**可对照实装的定理级约束**，供异质源恢复后直接消费，或供 Lead 据此走矛盾上浮。

## 四分法分类

真异质源不可达 = **行动类阻塞**（缺少外部资源且无法自行获取），非定理/选择/语法记录。
按 no-unnecessary-escalation："缺少外部数据/权限且无法自行获取" = 允许上报的合法情形。
