# #106 实装卡：V4 臂C typed miss 构成归因（#105 前置调研，只读不写代码）

- 日期：2026-07-21
- 票据：issue #106（wayfinder:research，AFK）；上游 #105（wf7 ✗ 后续方向 grilling）；map #59 子票。
- 性质：纯调研票——只读分析既有 dump/源码，零代码改动、禁 git mutation、主仓禁写、不改 rust/Cargo.toml。产物 = 调研报告（落 chanlun/review-results/），不写一行 rust。
- 090：归因到构成数字为止，不推测机制之外的东西；查不出的格子标「未确证」。

## 背景（全部带锚）

#77 V4 复验（v4-three-window-typed-chain-acceptance-20260721.md §2.4/§5）：

- wf7 臂C：typed_found=71 / typed_none=1653（命中率 4.1%）；准入坍缩到 Xzd 回退（xzd_fallback=1118）；装配侧 single_level_share=0.9605（INDEX rungs 73/1/2）；typed 门 wf7 仍有害（Long 翻负 7.0× 于 v0 门）。
- p3fold / wf8 同格局：命中率 80/1633=4.9% / 51/1518=3.4%；single_level_share 0.9579 / 1.0000。
- #75 身份桥：级别桥 ℓ=lvl+1、值桥 source_index==seg_c_full.1（turn_source 等值桥已证否）。
- 跨级链稀薄是既有事实：sidecar 链 72.7% 单级（nest-chain-existing-inventory-20260720.md:97-98）、95.36% rungs 空；密度调研裁定 ~300× 密度差是教义结构非漏检（cert-density-doctrine-research-20260721.md）。

## 核心问题

typed_none 的三窗 4,673 个 miss（1653+1553+1467）里，每一个属于哪一类：

- **A 类（桥键 bug）**：该 bar 前缀的 typed 索引里**存在**满足级别桥+值桥+side 的证书，但 `typed_lookup` 没查到（键构造/索引水线/喂法 bug）——可便宜修。
- **B 类（链不存在·候选无链）**：候选事件的 source_index 位置上**根本没有** typed 证书产出（装配没长出链）——真稀疏。
- **C 类（链存在但键域不匹配）**：索引里有证书，但证书的 (level, source_index, side) 与候选的查询键系统性错位（例如级别桥 ℓ=lvl+1 在这批候选上不成立、或 source_index 与 seg_c_full.1 不重合）——键域假设证伪。

## 数据与工具（全部在案，零新跑批）

- dump：`/tmp/v4_C/{p3fold,wf7,wf8}/trades.jsonl`（准入后）；日志 `/tmp/v4_C.out`（STATS :280/:285/:290、CHAIN :281/:286/:291、INDEX :282/:287/:292 行）。
- OPSEM dump 目录 `/tmp/v4_C/` 全量（含事件流/pending/views，按既有 opsem 格式）。
- 源码：typed_lookup 键构造（runner.rs NestChainGate :1137-1468）、索引构建（classifier/nest_index.rs）、身份桥确证记录（#75 交付报告 §1 + 记分卡 N3 段身份桥行）。
- **零新跑批**：本票不跑 cargo test 以外的任何重放；如需小探针（例如解析 opsem dump 的脚本），用 python 直接读 jsonl，不写 rust、不进 crate。cargo 只允许在 crate 空闲时跑既有测试，本票原则上不需要。

## 方法（建议，可按实测调整）

1. 从 opsem dump 复原 wf7 臂C 的候选序列（STATS 分账 cert_none=56 / 余者进查询的构成先拆开——cert_none 与 typed_none 是两类，先分账再归因）。
2. 对每个 typed_none 候选：取 (level, source_index, side) 查询键，在同 bar 前缀的索引快照里查——三分（A/B/C）。索引快照可从 dump 事件流重放 `sync_events` 序列重建（python 侧），或对每个候选记录 INDEX 行 counter 差分。
3. B 类内部再分：该位置为什么没长链——是跨级包含关系根本没出现（对照密度调研「教义结构」判定），还是出现了但装配未产出（装配缺口）。
4. 输出构成表：三窗 × {A/B/C + B 的子类} 计数 + 每类 3-5 个具体案例（bar/side/级别/键值/索引快照摘录）。

## 验收线

- 构成表三类计数齐全，总和 == typed_none 总数（三窗分别平账）；
- 每类至少 3 个带锚案例；
- A/C 类若存在：给出键错位的精确形态（哪个键、差什么），可直接导向修复票；
- B 类若为主：回答「跨级链稀薄是教义结构（数据里就没有）还是装配没造」——与密度调研「~300× 是教义结构」裁定对照，一致/矛盾都照实写；
- 报告落盘 `chanlun/review-results/typed-miss-attribution-20260721.md`（或开工日日期）。

## 交付后

#105 由编排者按归因结果裁定方向（A/C 主导 → 修复票；B·装配缺口主导 → 覆盖率治理票；B·教义结构主导 → 存档或门形态重议）。本票不替编排者选方向。
