# 亏损研究完整总结报告（2026-07-05）

> 从「为什么亏钱」到「双根因 + 唯一合法修复」的完整诊断链。本报告整合今日全部产出，每一步都经过严格核实，多个初步结论被后续核实修正——这是 no-patch-mentality 的实践记录。

---

## 〇、最终结论（一行）

**亏损双根因**：① signal 层无方向 alpha（μ 双门不过）+ ② execution 层 typed exit 触发质量缺陷（Stop 太远 / ReduceCore 未发 / 三类点时机错）。**INCONCLUSIVE 不变**。唯一合法修复 = q_Θ 升级消费 σ_higher（v0→v1 扩展）。

---

## 一、诊断链（自我修正过程）

### 步骤 1：opsem 逐笔重放（R5 修复后，commit e300242340）

**前提**：先修 R5（唯一真 bug——3 语义字段 lex_argmin_top3/divergence_input/nest_depth 未暴露，导致之前 6 个语法故事族全否决）。

**产出**：BTC 4653 笔真实交易（不编造），4 族聚类：
- 族 A 超大持仓跨大级别（3 笔，Σ-50564）
- 族 B 中枢震荡择时错（19 笔，Σ-70139）
- 族 C 背驰对但持仓久（1 笔 Type1）
- 族 D ReduceCore 减仓时机错（7 笔）

**初步结论**：亏损=持仓管理失败（持仓>5000bar + 逆势）。两个充分特征：正收益全≤666bar+0逆势；亏损含长持仓+10笔逆势。

### 步骤 2：ws-solresearch 严格调研 S1/S2/S3（commit 77006b82dc）

**核实**：对初步结论的三个修复方向逐一对照 formal-chain 原文 + 代码。

**三方向全否定**：
- **S1**（J_Θ 加趋势约束）：否定——J_Θ 定义封闭（§15 三项作用于净持仓 p）；ShortDiff 定义上即逆势（§7.5），加趋势项⟹抑制全部 ShortDiff⟹违反 §7.5
- **S2**（K_Θ 逆势门控）：否定——K_Θ 全资本约束不含趋势；AncOK 已实装裸逆势门控（coverage.rs:1753）
- **S3**（持仓时长上限）：否定——§9 typed exit 五枚举无时长型

**关键修正**：初步诊断的"逆势"**全是 ShortDiff 对冲腿**（§7.5 合法对冲，非裸投机）；trade 3765"J_Θ 选 Short"是对 J_Θ 作用层的误读（J_Θ 是净持仓函数非方向选择器）。

**精化结论**：真正根因=**typed exit 触发质量**（族 A Stop 缺陷/族 C ReduceCore 未发/族 D 三类点时机），属 exit/signal 侧。

### 步骤 3：quality-guard 对象域核实

**核实**：M8「execution 非缺陷」vs opsem「持仓管理根因」是否矛盾。

**结论**：同对象 Π_max-full。M8「execution 非缺陷」**声明膨胀**（超出 μ estimand 有效域，231/090），被 opsem 对照组 L2 反证（正收益 vs 亏损在 bsp/tw 同分布，分水岭在持仓管理⟹独立第二根因非 signal 传导）。

**立 698 号谱系**（commit a8cc38b2ed）：M8 §2.2 归因双修正——「单一因果链」→「主+execution 独立第二根因」；「execution 非缺陷」→「非 M6 成本记账缺陷」（策略退出行为层有独立缺陷）。INCONCLUSIVE 不变。

---

## 二、最终精化图景

```
亏损双根因（不同 estimand 域，231）：

signal 层（μ 统计）              execution 层（操作语义）
    │                                │
    ▼                                ▼
μ 双门无方向 alpha          typed exit 触发质量缺陷
（M8 统计层成立）           （Stop 太远/ReduceCore 未发/三类点时机）
    │                                │
    └────────── 共同导致 ────────────┘
                    │
                    ▼
        INCONCLUSIVE（不翻转）
        完整策略 OOS 全负
```

**关键澄清**：
- signal 层无 alpha 是**统计事实**（μ 双门不过，唯一正桶=beta 漂移）
- execution 层 typed exit 触发质量是**操作事实**（同 signal 下持仓管理决定盈亏）
- 两者不同 estimand（μ ⊥ 操作语义），互补非矛盾
- "逆势"是 ShortDiff 对冲腿（合法），单腿亏损=对冲成本，不蕴含 campaign 亏损

---

## 三、唯一合法修复方向

**q_Θ 升级消费 σ_higher（v0→v1 扩展）**

- **现状**：v0 q_Θ（leg_target, coverage.rs:1316）仅消费 depth（s_e=base_units×w_depth），未消费 σ_higher
- **formal-chain 要求**：σ_higher 是 q_Θ/SizeΘ 第 5 参数（§6 line 867 `SizeΘ(ℓ,δ,Iγ,N^depth,σ_higher,role,TStage,RiskMode,CostBucket,MarginState)`）
- **修复**：升级 q_Θ 消费 σ_higher，经 q_Θ 通道（p̃ 构造层），非 J_Θ/K_Θ
- **约束**：保 ShortDiff 合法性（§7.5）+ 遵守 §6 line 468-469「不能用全局顺/逆上级一刀切」
- **需新预注册**（135 号冻结先于跑数）+ 分级权重表

**为什么 S1/S2/S3 不合法**：
- S1（J_Θ 加趋势）：J_Θ 作用于净持仓 p 不选方向；加趋势项消灭 ShortDiff（§7.5 违反）
- S2（K_Θ 逆势门控）：K_Θ 是资本约束不含趋势；排除逆势消灭 ShortDiff + 破坏 X^cover（§16）
- S3（持仓时长上限）：§9 无时长出场型；"持仓过久"根因是 typed exit 未触发，非缺时长 cap；T_max 是数据窥探参数

---

## 四、今日产出清单（commit 链）

| commit | 内容 | 性质 |
|---|---|---|
| 5db6362093 | R5 语义插桩修复（3 字段暴露，env gated bit-exact） | 真 bug 修复 |
| e300242340 | opsem 重做诊断（R5 修复后真实可观测语义） | 初步诊断 |
| 77006b82dc | S1/S2/S3 严格调研（三方向全否定 + J_Θ 作用层澄清） | 否定性修正 |
| 7c97fae1c5 | R2 prereg-rev3（descend 已含 div_cand，R2 无需修订） | 否定性修正 |
| a8cc38b2ed | 698 号谱系（M8 归因双修正） | 语法记录 |
| a839c14c0f | R2 goal CLOSED（实装已满足裁决） | goal 闭合 |

---

## 五、待编排者裁决

1. **q_Θ v0→v1 扩展**（唯一合法修复）——是否推进？需新预注册
2. **M8 §2.2 措辞修正**（698 号已记录，是否改 M8 报告正文）——已交付报告，最小持久化原则下 698 谱系已够
3. **13+ pending /ritual**（长期选择类积压）

---

## 六、结果包六要素

1. **结论**：亏损双根因（signal 无 alpha + typed exit 触发质量缺陷），INCONCLUSIVE 不变，唯一合法修复=q_Θ 升级消费 σ_higher。
2. **定义依据**：formal-chain（完整的策略.pdf §6/§9/§15/§16、买卖点.pdf §7.5、关于背驰.pdf §4.3）+ 代码（interp.rs/coverage.rs/exit.rs/econ_positive.rs）+ R5 修复后真实 dump。
3. **边界条件**：①q_Θ 扩展后若 signal 仍无 alpha=理论边界；②typed exit 触发质量修复（Stop/ReduceCore/三类点时机）是独立维度，需 separate 预注册；③opsem 诊断有限效域（BTC/L2，dump 跑全量 48.7%）。
4. **下游推论**：q_Θ v0→v1 扩展是下一里程碑；M8 归因措辞已由 698 持久化；deep research workflow 在 glm 后端不可用（后续用自定义工位）。
5. **谱系引用**：231（有效域<定义域）、090（声明膨胀）、161（否定性照实）、698（M8 归因双修正）、694（高低配扬弃，第四方向=typed exit 触发质量）。
6. **影响声明**：新增本文件（纯研究合成，零代码改动）。整合今日 6 个 commit 的产出。不改任何 settled 谱系。
