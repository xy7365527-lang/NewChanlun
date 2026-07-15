# 裁定：严格 C2 pair 进入 N^δ 证书域（#91）

**状态：已裁定生效（2026-07-16，人裁口头批准："可以 按你的来" = 1批 2codex 3codex 4是）**
**依据**：双模型深度调研交叉对账
- codex gpt-5.6-sol：`chanlun/review-results/nest-ruling-deep-research-20260716.md`
- Claude：`chanlun/review-results/nest-ruling-deep-research-claude-20260716.md`

---

## 一、双模型一致项（待打包批准）

| # | 裁定 | 内容 |
|---|---|---|
| ② 主判据 | **B：背驰段口径** | J^δ_ℓ 的嵌套对象是"构成背驰的那段走势类型"（027:22,44-46 逐字），不含回试段，不是 leave→retest 全跨度 |
| ③ Conf | **维持 `BspBits::confirm_side`** | Λ≠∅（至少一类买卖点、不要求唯一）语义保持；D5 CompletedFreeze 只作完成/持久性见证，不替换 Conf |
| ⑤ 背驰域 | **盘整背驰入链** | N^δ 链背驰段域 = 趋势 ∪ 盘整（027 整课 + 万科旗舰例顶层即盘背）；typed 分流保留，盘背证书不冒充同级 B1/S1（#145 承接门不动） |
| ④ 钟 | **首次因果可证时点 + snapshot 纪律** | 禁止终态回填；`CompletedFreezeEvent.created_at`（工程持久化时戳）禁用 |

## 二、分歧项（需人裁二选一）

### D1（裁定点①）：Cand^δ_ℓ 要不要含背驰谓词
- **codex（主线推荐）**：不含。Cand = 纯结构宽候选，但须承载 `dir ∧ Comparable ∧ Extreme`（严格订正版 formal-criteria-20260705.md:60-70）；力度判据属于②的背驰段定位，塞进 Cand 是提前征税。P83 raw 相邻 pair 不够格。
- **Claude**：含。Cand = C2 pair ∧ 背驰命中，语义更贴"候选"字面。
- 互锁说明：②已取 B 时两者最终合取语义几乎等价，差别在中间量可观测性与计数口径。

### D3（裁定点④子点）：时序递降是硬门还是 sidecar
- **codex（主线推荐）**：sidecar。原文与递归式均无 `parent.confirm ≤ child.confirm` 此门，硬加属过度约束；只记录违反率，异常高再升级人裁。
- **Claude**：保留硬门（用背驰确认钟），更保守，产量可能进一步收缩。

## 三、D2（不裁，探针实证）——已拆两问

②细节对齐后发现两报告对 B 几何表述不一致，D2 拆为：

- **D2a 对象身份**：C2 域的 `leave`（CompletedMove）是否恒等于完整背驰段 c_ℓ？
  - Claude 断言恒等（leave.start 天然为 c 结构起点，旧塔 episode 局部化病灶在 C2 域消失）；
  - codex 认为未证明——须经 `DivergencePair.seg_c`（level_view.rs:384-398）与 CompletedMove 身份逐项对账，"找到 seg_c 坐标 ≠ 找到完整 c 的起止身份"。
- **D2b 链数单调性**：即便逐节点 A ⊇ B，链计数也**不自动**单调——父子两端同时变宽，B 链可能在 A 口径破包含、反之亦然（codex 反例论证成立，Claude"157 是 B 真上界"的推理不闭合）。
  - 探针：B 口径重算链集，与 157 条 A 链做双向差集对账；差集为空才可把 157 称上界。
- **附带发现（⑤落地缺口）**：当前 C2 provider 只从有方向 Trend block 生成 `DivergencePair`，Consolidation `dir=None` 被跳过（level_view.rs:400-415）——⑤裁"盘背入链"后此处需要扩 provider，属实现缺口非裁定项。

## 四、重放前只读探针清单（#92 前置）

1. D2a：`seg_c` ↔ CompletedMove 身份对账（差异计数）
2. D2b：A/B 双口径链集双向差集
3. 盘背反事实：Consolidation 若入 provider 的候选增量计数
4. 基例：链尾 `BspBits::confirm_side` 命中率
5. 钟：三钟（首次可证 / retest.end / freeze）通过率矩阵
6. 双核阴影对账（#89 静默双核 3,169 窗口在 C2 域是否残留）

## 五、签字位（2026-07-16 已签）

- [x] 一致项四条批准（②B 背驰段口径 / ③维持 BspBits / ⑤盘背入链 typed 分流 / ④首次可证钟 + snapshot + created_at 禁用）
- [x] D1：**codex 不含**——Cand = 纯结构宽候选（dir ∧ Comparable ∧ Extreme），力度判据留在②
- [x] D3：**codex sidecar**——时序递降只记录违反率不拦截，异常高再升级人裁
- [x] 零产量签字条件（R7）**接受**：B 重放若为 0，须"provider 完整 + 定义忠实 + snapshot 无前视"三项通过后方可写成"正确市场答案"，否则只能写能力边界或未决
