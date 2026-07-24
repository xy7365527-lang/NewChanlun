# 正式 supersede 文书：N2 rung 力度门 supersede #97 D1（程序补正）

- 日期：2026-07-21
- 性质：**编排者令补正**（grill 会话：「①需要补文书」）。被 supersede 方：`p97-*` 裁定 D1「rung 初筛只看结构、力度留在基例门」。
- 背景：V2 N2 力度门实装时（v2-rung-force-gate-impl-20260720.md §7.1）以「实装卡登记」程序 supersede #97 D1，未经正式裁定文书。code-review Standards #3 指认为程序 judgement call；编排者令补正式文书。

## 1. SUPERSEDE 条款（显式列明被替代条款）

**本文替代 #97 裁定 D1**（「rung 初筛只看结构、力度留在基例门」口径）。替代范围：rung 级筛选谓词；#97 其余条款不受影响。

## 2. 替代内容（已落码事实的裁定追认）

- `rust/src/theta_v0/classifier/nest.rs:775`：`if event.side != side || !event.divergence_confirmed { continue; }`——rung 级筛选从「只看结构包含」升格为「结构包含 ∧ 同向 ∧ 背驰确认」。
- 注释改写 nest.rs:769-774；#97 唯一旧断言改写 nest.rs:1752-1756（父级 false→true，覆盖后退已登记）。
- 教义锚：027:38/027:46（「背驰段的背驰段」力度要求——子背驰段严格在父背驰段内且力度确认）；`doctrine-vs-code-nest-recursion-20260720.md` G-2。
- 实证：全量 4.6M pre/post 双跑 I1-I5 全过（v2-rung-force-gate-impl-20260720.md §6.3）——门后 2,964 张证书 confirmed_vec 恒全 '1'；基例层 855/855 逐格相等（基例门零改）；XiaozhuandaCandidate 150→0（后续 R-2 裁定另文处置）。

## 3. 程序追认声明

V2 实装时的「实装卡登记」程序不符裁定链惯例（裁定 supersede 裁定）。本文将 R-1 登记升格为正式裁定，追溯生效至 V2 落码时点；此后一切 supersede 必须经正式文书，实装卡自封「裁定登记」不再构成 supersede 效力。

## 签字位

- [x] #97 D1 supersede 追认（rung 级力度门为唯一合法口径）——编排者令补正（2026-07-21）
- [x] 程序规则：supersede 必须经正式文书（实装卡登记无 supersede 效力）
- [ ] 编排者复议（空位）
