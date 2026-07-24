# #214 背书失败原因测量装置——实装说明

- 日期：2026-07-24 ｜ 票据：issue #214（wayfinder map #126 子票；spec = tracker #215
  `chanlun/review-results/spec-endorsement-failure-instrument-20260724.md`）
- 工作面：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717）；
  git 零 mutation；改动全部在未提交面上叠加
- 验收状态：TDD 主接缝 12 新测试全绿（62 既有零改写零删除）；三窗重放红线对照
  逐字节一致（72 项断言退出码 0）

## 1. 改动面（3 文件 + 产物）

| 文件 | 改动 | 性质 |
|---|---|---|
| `rust/src/theta_v0/classifier/nest.rs` | `EndorsementProbe` 结构新增；`terminal_bits_in_book` 判定逻辑收进私有核 `terminal_bits_in_book_core`，原函数变薄包装；`terminal_bits_at_event` 变薄包装 + 新增测量入口 `terminal_bits_at_event_measured`；`mod tests` 纯追加 12 测试 | 核化（ID-1C）+ 测试 |
| `rust/src/theta_v0/classifier/nest_index.rs` | `NestEndorsementInstrument` sidecar 结构 + `observe_base` 归集；`NestCertificateIndex` 加 `instrument` 字段与访问器；`build_nest_certificate_index` 循环改经测量入口每基例求值一次 | sidecar（ID-4） |
| `rust/src/theta_v0/backtest/fill.rs` | NEST_GATE_INDEX 行同侧新增两条纯增量 stderr 行（NEST_GATE_FAIL / NEST_GATE_LEVEL） | 输出载体（ID-5，spec 指定落账点） |

注：fill.rs 接线属 spec ID-5 明定的落账点（「落账点在既有 NEST_GATE_INDEX 行同侧，
接线最小」），是一个注释块 + 两条新增 `eprintln!` 行，不触任何既有行。

## 2. 核化结构（ID-1C 私有核 + 双薄入口）

```text
terminal_bits_in_book_core（私有核）——判定逻辑唯一来源
  ├─ terminal_bits_in_book（生产薄包装，签名/语义逐字保留，丢探针）
  │    └─ 全部研究 bin 消费点（p92/p116/p117/p123/p124/p125）零改动
  └─ terminal_bits_at_event_measured（测量薄入口，留探针）
       └─ 唯一消费方：build_nest_certificate_index
            （terminal_bits_at_event 同为薄包装 = measured(...).0）
```

- 核内 `CWindow` 臂单次账本遍历逐点复算生产合取分量（窗口 → `confirm_side` →
  点类分域 owner），判定候选集与计数同源产出；`best` 严格更小 `source_index` 才替换
  ⟺ 生产 `min_by_key` 首个平局保留——逐 bit 等价（ID-6.1 同函数语义内重构）。
- `Exact` 诊断臂逐字保留原 `find` 查法，探针零值（不产计数）。
- 构建器循环原「每 top 尝试重算背书」改为「每基例测量入口求值一次 + 闭包回返
  预计算 bits」——`assemble_typed_certificate` 只对 base 调 `terminal_of`（nest.rs:645
  区域），纯函数预计算等价；背书结果、装配结果、去重、rungs 构成一个 bit 不变。

## 3. 计数口径（spec ID-2/ID-3/ID-4 + 2026-07-24 编排者钉口径①-⑥）

- **事件级四桶**（Trend 域，穿透序首因，互斥完备）：
  1. 窗内点数 = 0 → 账本有点（不限方向，宽口径③）归 `out_of_window`，账本空归
     `no_valid_point`（子计数 `book_empty`）；账本级别缺失/越界归 `no_valid_point`
     （子计数 `book_missing` 单列）。
  2. 窗内有点、同向点数 = 0 → `opposite_side`。
  3. 同向点 > 0、owner 等值 = 0 → `owner_start_neq`；`center=None`（身份缺失）与
     `start_index` 实不等同桶、点子计数分列④（`owner_identity_missing_pts` /
     `owner_real_neq_pts`）。
  4. owner 等值 > 0 → 判定必命中 → `trend_success`（不计桶）。
- **点级二维**（ID-3，C1 读数）：Trend 域窗内同向点按 `[一/二/三类][owner 等值/不等]`
  计数，事件窗内逐窗各计①（同一物理点落多事件窗逐窗各计一次）；成功事件点同样
  入计（分母覆盖成功+失败全体）；一位多类点按所属类逐行各计。
- **按级 × kind 两维分解**（ID-4，钉口径②）：`base/assembled/indexed_by_level_kind`，
  行 = 级别槽（槽 0 恒零，nest 不听 L0），列 `[Trend, Consolidation]`；
  kind 合计 = C2 分母（`base_trend()` / `base_consolidation()`）。
- **Pan 域**⑥：只计 `pan_success` / `pan_fail` 总数，四桶/点级不适用。
- 完备性（测试与 replay 校验共用断言）：`trend_total() == base_trend()`，
  `no_valid_point == book_missing + book_empty`，
  `Σbase_lk == stats.base_events`（assembled/indexed 同理）。
- v3 纪律：全部读数为全量计数（无抽样、无概率推断）；owner 相等 = `start_index`
  精确等值，无容差概念。

## 4. 零干预保证（ID-6 落实）

1. 共核单一来源：生产合取序/窗口/方向/owner 判定式逐字保留于核（见 §2 diff）；
2. 既有测试锁定：nest 模块 62 个既有单测零改写零删除，12 新测试纯追加
   （scoped 74 passed / 0 failed）；
3. 重放红线对照：三窗 NEST_GATE_STATS/CHAIN/T3/INDEX 整行、tower_events MD5、
   trades MD5、候选流（t5a_chain_dump）MD5 与改码前基线**逐字节一致**
   （校验脚本 E 组，72 项断言退出码 0）；
4. digest 面：sidecar 为新增结构，不进任何既有 GOLDEN/digest 面；全量
   `cargo test --lib` = 1789 passed / 1 failed，唯一失败
   `extract_signals_bit_exact_digest_guard` 为 #110 工作线既线失败
   （signal.rs 未提交 3+/3- 改动在案，与本装置无关，未修未归因）；
5. 090：本说明每条结论均有测试或 replay 证据；未验证项在 §6 如实列。

## 5. 既有 doc 的如实改写（钉口径⑤ 灰区，仅核化触及处）

- `terminal_bits_in_book` doc：Trend 域描述由旧「`buy1|sell1 ∧ owner=B`」如实改为
  「同向一/二/三类 ∧ owner=B（P1 重议 2026-07-21）」——核化触及处，090 声明=能力。
- `terminal_bits_at_event` doc：同步改写 + 登记双薄入口结构。
- 未触及但同样过时的 doc（留归 #210 后续，本票不顺手清）：`TerminalMatch::CWindow`
  枚举变体注释（nest.rs:503-511 区域「Trend = 破 B 一类点」）、
  `assemble_typed_certificate` doc（nest.rs:626-629 区域同口径）、审计 bin
  p125_endorsement_audit.rs:392 的 P125_RULE 串。已在返回报告中列为 spec 外观察。

## 6. 未验证项（090 如实）

- 事件级 `b_center_start=None` 路径在生产不可达（`NestCandidateEvent.b_center_start`
  为 `usize` 恒 Some）；核签名保留该臂（与生产 `is_some_and` 短路同构），三窗
  `owner_pts_id_missing=0` 佐证生产点 center 恒 Some。该臂只有合成测试覆盖。
- ` Exact` 臂探针零值只有单测覆盖（生产重放不走 Exact）。
- 物理点去重口径（ID-3 开放问题的另一口径）未实装——spec 推荐事件窗内口径为主读数，
  去重口径是否另列归编排者后续裁定。

## 7. 复算入口

- scoped 测试：`cargo test --lib theta_v0::classifier::nest`（74 passed）。
- 三窗重放：`THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=<dir>/opsem T5A_CHAIN_DUMP_DIR=<dir>/dump
  M8_WIN_FILTER=<tag> cargo test --release --lib
  theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`。
- 校验：`python3 chanlun/review-results/endorsement-failure-instrument-verify-20260724.py`
  （默认读 /tmp/nest214/{baseline,after}；输出留档
  `endorsement-failure-instrument-verify-output-20260724.txt`，退出码 0）。
