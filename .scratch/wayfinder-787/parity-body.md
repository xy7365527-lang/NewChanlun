## 现象

`rust/tests/theta_v0_classifier_parity.rs` 有 **2 条 Lean↔Rust 对拍断言在当前 `main` 线上是红的**，稳定复现：

```
$ cd rust && cargo test --test theta_v0_classifier_parity
17 passed; 2 failed

✗ type1_buy_broke_and_diverge_matches_lean_istype1  (:528)
    assertion `left == right` failed:
    Lean IsType1（brokeCenter ∧ IsDivergence）⟺ rust buy1 置位
      left: 0   （rust 没置位）
      right: 1  （Lean 判成立）

✗ signal_extraction_emits_no_second_class_only      (:629)
    panicked: 产第一类（趋势背驰：C 段破最后中枢 ∧ C<A 面积）
```

**发现路径**：[#810](https://github.com/xy7365527-lang/NewChanlun/issues/810)（CI 接 `cargo test`）的产出复核。派出的子代理在 `origin/main-rewritten`（**落后 `main` 545 个提交**的 CI 镜像分支）上跑得 0 failed；主控在当前线复跑即红。**非本次改动引入**——当前分支相对 `main` 的 12 个提交 `git diff --stat main..HEAD -- rust/` 为空，全是文档。

## ⚠ 这不是普通 bug，不要直接「修绿」

按 `AGENTS.md`「缠论教义正本」的权威分层：**原文管语义 / Lean 管边界 / 生产代码只有否决权**。Lean 与 Rust 对「一类买点」判得不一样，性质是**教义分歧**，不是实现漂移——**「谁对」本身就是要裁的东西**。

**⟹ 禁止的处置**：改 Rust 去迎合 Lean、改 Lean 去迎合 Rust、或把测试 `#[ignore]` 掉让 CI 变绿。任何一种都是[#321](https://github.com/xy7365527-lang/NewChanlun/issues/321) 那次「**裁错了层**」的重演。

## 去向

**归 [map #787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 第二批的「买卖点概念票」的必答项**（该票尚未开出，待 [#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 的清点结果定第二批范围）。[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 已独立判出买卖点概念下有 3 组真分歧待裁，并点名「Lean 明文拒绝次级别递归 vs 生产走定律一」的冲突——**本票这两条红断言是那组分歧的活证据**。

本票的作用是**别让它在票外丢掉**：在买卖点概念票裁定落地之前，这两条保持红、不修、不 ignore。

## 待办（裁定之后才做）

- [ ] 买卖点概念票裁定「一类买点的定义以哪一侧为准」
- [ ] 按裁定改对应一侧，两条断言转绿
- [ ] 若裁定认为断言本身写错了（对拍写反/witness 取错），单独说明并改断言
