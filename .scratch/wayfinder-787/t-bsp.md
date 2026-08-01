Part of #787

## Question

**买卖点的教义正本**——第二批概念票第 5 张。清点见 [总缝规则过筛 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)。

**排在背驰票之后**：B-1 直接问「一类点判据含不含背驰」，背驰的力度口径没定就答不了。

### 要裁的 3 组（B-1…B-3）

| 组 | 问题 |
|---|---|
| **B-1** | 一类点判据含不含背驰——含（`BspClassification.lean:99` `IsType1 = brokeCenter ∧ IsDivergence`）↔ 不含（`theta_v0/classifier/bsp.rs:78`，`:159` 明说与 `macd_c_lt_a` 无关）↔ 背驰是生成源且用比值阈值（`buysellpoint.rs:195`，`force_c/force_a ≤ 0.9`）。 |
| **B-2** | 二类点的锚是什么——一类点的价（2 处）↔ 次级别第一类 + 回拉不创新低（`rmove_compose.rs:144` + `descend.rs:183`，真下钻）↔ `afterTypeOne ∧ ¬brokeCenter`（`BspClassification.lean:110`，且 `SellClosedLoop.lean` **明文拒绝冒充次级别递归**）↔ `after_first_buy ∧ is_pullback_end`（`bsp.rs:82`）。**四种。** |
| **B-3** | 三类点「第一次回抽」在不在判据——在（`BspClassification.lean:113` `firstRetrace`）↔ 不在（`bsp.rs:86`）↔ 隐式承担（`buysellpoint.rs:418`）。 |

### ⚠ 本票有两条红的对拍断言在等着（必答项）

[Lean↔Rust 一类买点对拍两条断言在 main 上为红 #811](https://github.com/xy7365527-lang/NewChanlun/issues/811)：

```
rust/tests/theta_v0_classifier_parity.rs
  ✗ type1_buy_broke_and_diverge_matches_lean_istype1  (:528)  left 0 / right 1
  ✗ signal_extraction_emits_no_second_class_only      (:629)
```

**这两条就是 B-1 的活证据**——Lean 判成立、Rust 没置位。按 [#811](https://github.com/xy7365527-lang/NewChanlun/issues/811) 的规定，**在本票裁定之前不许修绿、不许 `#[ignore]`、不许踢出 CI**。本票裁完之后按裁定改对应一侧，断言随之转绿。

**另**：B-2 的 Lean 侧「明文拒绝冒充次级别递归」vs 生产走真下钻，是同一冲突的第二个面，本票须正面回答。

### 关票判据

正本落 `.chanlun/definitions/maimaidian.md` + 受影响代码清单；并在 [#811](https://github.com/xy7365527-lang/NewChanlun/issues/811) 留处置指针。

### 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。落完文档才关票。
