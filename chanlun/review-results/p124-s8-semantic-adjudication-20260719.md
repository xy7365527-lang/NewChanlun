# 编排者裁定：关② S=8 语义收口（对 C-3 semantic_verdict=FAIL 的复裁）

**裁定人**：编排者（claude-fable-5 session 438f5dc7）· 2026-07-19
**对象**：p124-s8-final-recon-c3-20260719.md 之 `semantic_verdict=FAIL`（20 个非一类 Trend 终端存续、锚 121 CERT，违 C-3 任务书 P1）

## 证据链（全部直读核对）
1. **存续掩码分布**（terminal-bridge tsv）：7×Long `010000`、12×Short `000010`、1×Short `000001`——每个存续终端恰以**自身类别位**（buy2/sell2/sell3）过门，无一例外。
2. **谓词源码**（rust/src/theta_v0/types.rs）：`conf_plus()=buy1∨buy2∨buy3`（:202-204）、`conf_minus()=sell1∨sell2∨sell3`（:210-212）、`confirm_side()`（:216-221）——字面析取。
3. **裁定权威**（escalate/bsp-terminal-endorsement-t1-review-20260718.md §5 行93）：`confirm_side` 宽于精确化背书语义系**在册缝隙**，编排者 (a) 定调：**非实装错误、不实装修改**，任何谓词收紧属新裁定文书域（090 禁模糊地带）；引用终端门实测读数的报告**须携带身份注记**。
4. C-3 沙箱禁入主仓/worktree（`requested_authority ... missing` 系沙箱产物）——其 P1 严格判定在视界内正确，但未见 T1 §5。

## 裁定
- 关② 语义收口改判 **PASS-with-registered-gap**：20 存续/121 CERT = 在册缝隙的合法表达，保留并携 T1 §5 身份注记（本文即注记载体）。
- C-3 机械账三项 PASS（terminal_cert_bridge / turn_class_bridge / pan_877_identity）照单采认。
- 关③ 硬界同步采认：`terminal_key 1213→855`、`pan_terminal 748 零漂移`、877 池 `110链/122事件/272基例(31背书)` 与 escalate 裁定全等。

## 残项登记（非阻塞）
- 8 个 type1 失证的 owner=B 归因系**推断**（p125_audit.out:2 owner readback 未做）——属 drop 侧保守方向，登记待 owner 读回抽验。
- 台账第 3 行「并发场景验证」已由 C-4 让位实证 ✅，回写待主仓禁写解除。
