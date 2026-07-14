# P0 复议裁决登记：J_parent 口径改判（2026-07-10）

- 裁决人：用户（本轮会话口头批准"让 gpt-5.6 执行"）
- 依据材料：`chanlun/review-results/nest-lag-deep-research-20260709.md` §对 P0 复议的建议；codex(gpt-5.6-sol) 只读复核意见（/tmp/codex-ruling-opinion.log）
- supersede：#28 adopted-default 中"J_parent = 父级确认区间"的读法（"闭口径 ⊆"关系本身保留）

## 裁决内容

1. **J_parent := 父级背驰段 D_parent**（进入背驰段起点 → 该级别背驰对应转折端点），不再取父级确认区间。
2. 嵌套判据取 **I(A) ⊆ D_parent**（闭包含，与 nest.rs Sub 契约同构）。
3. **λ_C（子级确认时刻）不参与包含否决**：仅要求存在，登记 λ_C − right(D_parent) 延迟分布（诊断用）。
4. **ε_conf 降级为纯诊断参数**，不进判据。
5. **D_parent 左端判定与 D_child 字段映射必须在重放前冻结**，不得见产量后调整。
6. **L0→L1 ≥ 3 对仅作同基线回归预期**，不作定义正确性的唯一硬门。

## 附带（本次未裁）

- cert F-01 的 A/B 节归属：未裁；裁定前 bughunt-fix-v2 不得宣告 merge-ready、不合主线。
- BUG-10：按"宽中间类型 + checked 转换"口径待批，P2 不阻塞重放。

## 实施

- 实验基线：`1dcdb95a09`（bughunt-fix-v2 头）派生独立复议分支，纳入 confirm_src 拆绑（`7a571d1a38` 或等价）。
- 重放必须显式过滤 `cand_delta=true`（隔离 F-02 新增盘整诊断事件）。
- 判别实验：按 nest-lag-deep-research §必答子问题5 分支逻辑归因 L1→L2 的 0/0。
