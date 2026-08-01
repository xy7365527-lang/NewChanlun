Part of #787

## Question

**背驰的教义正本**——第二批概念票第 3 张。清点见 [总缝规则过筛 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)。

本票是违规重灾区：[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 判出的 **6 条违规里 4 条落在「力度」上**（V1/V3/V4/V6）。违规定性已立，**收敛到哪一套是本票的事**。

### 要裁的 5 组（D-1…D-5）

| 组 | 问题 |
|---|---|
| **D-1** | 「力度」是什么——MACD 段面积 ↔ 三维度 OR + T4 零轴穿越 ↔ 振幅×时长 ↔ 中枢嵌套深度 ↔ persistence Wasserstein-1。**五种互不相容。** 源码自陈「与缠师第 17 课原文相悖」（`theta_v0/classifier/divergence.rs:256`）。后两种的**兜底接法**已判 V3/V4 违规，但**作为力度候选口径本身仍待裁**。 |
| **D-2** | 次级别背驰要不要 `Extreme` 条件——`div_cand` 要（`cand_predicate.rs:107`）↔ `sublevel_diverges` 不要（`classifier/mod.rs:1929`）。**已记账**（[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十③）。⚠ 收敛后信号量可能直接归零——[#796](https://github.com/xy7365527-lang/NewChanlun/issues/796) 实测 52 个候选中 **51 个**靠宽松档放行。 |
| **D-3** | 盘整背驰比较哪两段——最后两个同向离开段 ↔ 进入段 vs 离开段 ↔ 只定位不判力度。 |
| **D-4** | 判据形状：严格面积 `<` 还是三维 OR——`segments_diverge` ↔ `segments_diverge_or`（**后者是生产件**，`signal.rs:1285` 盘整背驰真值路径，[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 订正了 [#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 的低估）。可援引限定词①故不判违规，但「同一概念两种判据形状」须收口。 |
| **D-5** | 力度判据能不能常驻可配置四档——`DivergenceGauge{MacdArea,ThetaDom,Conjunction,ThetaLex}` 进生产热路径。**灰区**：明写「不 fallback、no-workaround」⟹ 不踩兜底那半条；但踩了「宽严两档」的字面，且三条限定词都不完全覆盖。**这一刀同时管住线段 G-4 与笔 S-2 的配置档，两票口径须一致。** |

### 例外上限预警

[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 点名：V6（`a_nested_divergence` 力度按级别换判据）与中枢票的 E1 **同型**。若本票选择给它开举证书而不是收敛，[#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 的**全仓 3 条上限**会在中枢票+本票之内用光。

### 关票判据

正本落 `.chanlun/definitions/beichi.md` + 受影响代码清单（点名到行号）。

### 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。落完文档才关票。
