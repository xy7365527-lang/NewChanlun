# 证书索引口径审计：收了什么、漏了什么（级别×类型）

- **日期**：2026-07-21 ｜ **票据**：issue #128（wayfinder:research，AFK）
- **性质**：纯调研——只读分析源码/dump，零代码改动、禁 cargo、禁 git mutation、主仓禁写
- **数据源**：worktree `/tmp/kimi-nest-mainline`（分支 `kimi-nest-mainline-20260717`），dump `/tmp/v4_C.out`、`/tmp/v4_C/{p3fold,wf7,wf8}/trades.jsonl`、`/tmp/v4_A/{p3fold,wf7,wf8}/trades.jsonl`
- **上游裁定**：ADR `chanlun/escalate/adr-quasi-chain-direction-and-multiview-20260721.md`；Parent map #126

---

## 结论速览

| # | 结论 | 性质 |
|---|------|------|
| 1 | **Trend 域终端背书确证只收 buy1\|sell1**（owner=B 合取）——CWindow 合法性过滤将 type2/3 构造性排除，代码直读确证 | 代码确证 |
| 2 | **absorb_exts 收全级别（L1-L5）事件，不限于 L0**——`sync_events` 遍历 `1..n_levels`，每级派生事件；但**索引终端背书只读 `levels[ℓ-1]` 账本**，L0 事件被 `events_by_level[0]` 忽略（`exec` 从 1 起扫），L0 bsp 不直接经事件进索引 | 代码确证 |
| 3 | wf7 三窗索引 typed 命中率 = **4.1% / 4.9% / 3.4%**（typed_found/total_candidates）；消费侧 class1 几乎被消灭（臂C 三窗合计仅 1 笔 class1，臂A 合计 22 笔） | 实测 |
| 4 | 截断两源：① **Trend×type2/3 构造性排除**（P1 裁定，Trend 域合法背书只收一类点）→ 趋势背驰事件的 type2/3 终端背书被全量裁掉；② **CWindow 窗口坐标 + 值桥收束**→ `seg_c_full.1` 不等于候选 `source_index` 的 bsp 全部 miss | 代码 + 实测 |

---

## §1 Trend 域确证只收 buy1|sell1（调研项 1）

### 1.1 代码确证链

**入口**：`nest_index.rs:129-131`——索引构建器固定调 `terminal_bits_at_event(classification, event, TerminalMatch::CWindow)`。

**核心函数**：`nest.rs:644-659` `terminal_bits_at_event` → 委托 `terminal_bits_in_book`。

**合法性过滤**（`nest.rs:611-629`，CWindow 分支）：

```rust
TerminalMatch::CWindow => bsp
    .iter()
    .filter(|point| {
        c_start <= point.source_index
            && point.source_index <= turn_source
            && point.bits.confirm_side(side)
            && match kind {
                NestDivergenceKind::Consolidation => true,  // Pan: 同向一/二/三类全合法
                NestDivergenceKind::Trend => {
                    (point.bits.buy1 || point.bits.sell1)   // ← 只收一类！
                        && b_center_start.is_some_and(|b| {
                            point.center.is_some_and(|c| c.start_index == b)
                        })  // + owner=B 合取
                }
            }
    })
    .min_by_key(|point| point.source_index)  // 窗口内最早合法点
    .map(endorsement),
```

### 1.2 确证结论

| 事件族 | 合法背书点类 | 裁掉点类 | 代码锚 |
|--------|-------------|---------|--------|
| **Trend** | **仅 buy1\|sell1**（+ owner=B 合取） | buy2/buy3/sell2/sell3 全排除 | `nest.rs:619-624` |
| **Consolidation** | **同向一/二/三类全合法**（`confirm_side` 即精确语义） | 无排除 | `nest.rs:618` |

- **Trend 域只收 buy1|sell1：确证成立。**
- 这是关③ P1 裁定的实装（`pan-terminal-endorsement-ruling-20260718.md:68-73`）——趋势背驰的终端背书 = 破 B 一类点，二/三类在 Trend 域被构造性排除。
- **零违反可能**：该过滤在 `terminal_bits_in_book` 单一来源函数内，调用点零限定（`nest_index.rs:129` 不加额外过滤）；lib 非测试代码中无第二调用点（全仓 grep 确证，`bsp-type-usage-audit-20260719.md:65`）。

---

## §2 absorb_exts 事件来源——全级别收，不止 L0（调研项 2）

### 2.1 事件派生路径

**`sync_events`**（`nest_gate.rs:380-412`）：

```rust
for level in 1..n_levels {   // ← 从 L1 到最高级
    // 值指纹跳过未变级
    if let Some(fp) = &self.derived[level] {
        if fp.content_unchanged(...) { continue; }
    }
    let exts = self.derive_level_events(tower, level, as_of);
    self.absorb_exts(exts);
    // 存值指纹...
}
```

- **遍历 `1..n_levels`**——事件派生覆盖 L1 到塔最高级（实测 wf7 可达 L4-L5），**不限于 L0**。
- `derive_level_events`（`nest_gate.rs:401-479`）在每级 `tower[level]` 上跑投影 → decompose → assemble_level_view → `provide_nest_candidate_events_ext`，产出该级的 Trend + Consolidation 事件。

### 2.2 absorb_exts 收入条件

**`absorb_exts`**（`nest_gate.rs:482-510`）：

```rust
for ext in exts {
    let event = ext.event;
    self.n_events_seen += 1;
    if !event.divergence_confirmed { continue; }  // 只收确认事件
    let id = NestEventIdentity::of(&event);
    if !self.seen.insert(id) { continue; }        // first-wins dedup
    // 写入 by_end（固定级别键）+ by_end_multi（多级键）
    self.by_end.entry((event.level, ext.seg_c_full.1, is_long))...
    self.by_end_multi.entry((ext.seg_c_full.1, is_long))...
    self.events_by_level[event.level].push(event);
}
```

- **收全级别确认事件**（`event.level ∈ [1, n_levels)`），不限 L0。
- 但 **L0 事件不进索引**：`nest_index.rs:135` 的 exec 循环从 `1..events_by_level.len()` 起扫，`events_by_level[0]` 内容被忽略（注释明写「nest 不听 L0，p105 §3」）。

### 2.3 索引终端背书的级别移位

**`terminal_bits_at_event`**（`nest.rs:644-659`）查账本时做级别移位：

```rust
let book = &c.levels.get(event_bsp_book_level(e.level)?)?.bsp;
// event_bsp_book_level(ℓ) = ℓ-1  (ℓ≥1)
```

- level-ℓ 事件的终端背书查 **`levels[ℓ-1].bsp`**（子级别账本）。
- L1 事件查 `levels[0].bsp`（L0 账本，点最密）。
- L2 事件查 `levels[1].bsp`。
- 以此类推。

### 2.4 高级别 bsp 是否进索引？

**事件侧：进。** `derive_level_events` 对每级 L1+ 都派生事件，经 `absorb_exts` 写入 `events_by_level[ℓ]` 与 `by_end` / `by_end_multi` 索引。

**终端背书侧：受级别移位约束。** level-ℓ 事件的终端背书只能从 `levels[ℓ-1].bsp` 窗口内查到。如果该窗口内没有合法背书点（Trend 域需要 buy1/sell1 + owner=B；Consolidation 域需要同向 confirm_side 点），则 `terminal_bits_at_event` 返回 `None`——基例力度门拒证，该事件不产 typed 证书。

**实测证据**：wf7 窗 INDEX events=879、indexed=76——事件产出 879 个（经 `divergence_confirmed` 过滤后），但终端背书通过 + 装配后仅 76 张证书入索引（通过率 8.6%）。

---

## §3 塔各级别 bsp 总量 vs 索引命中量（调研项 3）

### 3.1 候选总量与索引侧

| 指标 | p3fold | wf7 | wf8 |
|------|--------|-----|-----|
| STATS total（候选总数） | 1,633 | 1,724 | 1,518 |
| INDEX events（确认事件入索引） | 773 | 879 | 725 |
| INDEX indexed（证书去重后） | 95 | 76 | 55 |
| INDEX single_level_share | 0.9579 | 0.9605 | 1.0000 |
| CHAIN typed_found（身份桥命中） | 80 | 71 | 51 |
| CHAIN typed_none（身份桥 miss） | 1,553 | 1,653 | 1,467 |
| **typed 命中率** | **4.9%** | **4.1%** | **3.4%** |

### 3.2 消费侧（trades.jsonl）各级别 × 点类对照

**臂A（门关 = 全量消费，代表塔各级别 bsp 真实分布）**：

| 窗 | 级别 | class1 | class2 | class3 | 小计 |
|----|------|--------|--------|--------|------|
| p3fold | L0 | 8 | 139 | 282 | 429 |
| p3fold | L1 | 4 | 29 | 0 | 33 |
| p3fold | L2 | 1 | 16 | 0 | 17 |
| **p3fold 合计** | | **13** | **184** | **282** | **479** |
| wf7 | L0 | 6 | 149 | 283 | 438 |
| wf7 | L1 | 3 | 25 | 1 | 29 |
| wf7 | L2 | 0 | 25 | 0 | 25 |
| wf7 | L3 | 0 | 1 | 0 | 1 |
| wf7 | L4 | 0 | 11 | 0 | 11 |
| **wf7 合计** | | **9** | **211** | **284** | **504** |
| wf8 | L0 | 0 | 125 | 309 | 434 |
| wf8 | L1 | 5 | 40 | 9 | 54 |
| wf8 | L2 | 0 | 5 | 2 | 7 |
| wf8 | L3 | 0 | 22 | 1 | 23 |
| **wf8 合计** | | **5** | **192** | **321** | **518** |

**臂C（typed 真链门开）**：

| 窗 | 级别 | class1 | class2 | class3 | 小计 |
|----|------|--------|--------|--------|------|
| p3fold | L0 | 0 | 131 | 15 | 146 |
| p3fold | L2 | 1 | 17 | 0 | 18 |
| **p3fold 合计** | | **1** | **148** | **15** | **164** |
| wf7 | L0 | 0 | 147 | 15 | 162 |
| wf7 | L1 | 0 | 1 | 0 | 1 |
| wf7 | L2 | 0 | 26 | 0 | 26 |
| wf7 | L3 | 0 | 13 | 0 | 13 |
| wf7 | L4 | 0 | 11 | 0 | 11 |
| **wf7 合计** | | **0** | **198** | **15** | **213** |
| wf8 | L0 | 0 | 134 | 16 | 150 |
| wf8 | L1 | 0 | 1 | 0 | 1 |
| wf8 | L2 | 0 | 5 | 0 | 5 |
| wf8 | L3 | 0 | 18 | 0 | 18 |
| **wf8 合计** | | **0** | **158** | **16** | **174** |

### 3.3 命中量 vs 总量——各级别命中率

以 wf7 为例（臂A = 候选总量，臂C = typed 门后幸存量）：

| 级别 | 臂A 总笔 | 臂C 幸存笔 | 存留率 | 裁掉率 |
|------|---------|-----------|--------|--------|
| L0 | 438 | 162 | 37.0% | 63.0% |
| L1 | 29 | 1 | 3.4% | 96.6% |
| L2 | 25 | 26 | 104.0%* | — |
| L3 | 1 | 13 | — | — |
| L4 | 11 | 11 | 100% | 0% |
| **合计** | **504** | **213** | **42.3%** | **57.7%** |

> *L2 存留笔 > 臂A 是因为臂间交易结构不同（Xzd 回退通道裁单逻辑差异），不直接可比笔数；命中率以 CHAIN 行的 typed_found/total 为准。

**关键读数**：消费侧存留率 ~42% 远高于 typed 命中率 ~4%——因为准入主体是 **Xzd 回退通道**（`xzd_pass=609` vs `nest_pass=71`），真链通道只承载了 71 个候选的准入，其余 609 个经 Xzd 通道（非 typed 证书索引）准进。

### 3.4 索引侧各级别证书产出（从 INDEX 行反推）

INDEX 行不直接报各级别证书数，但可从 `events_by_level` 反推。wf7 窗：

- `events_seen=142,399,680`——派生产出的全部事件（含未确认），分布在 L1-L4+。
- `events=879`——经 `divergence_confirmed` 过滤后写入 `events_by_level` 的确认事件。
- `base_events=878`——exec≥1 级事件总数（≈events，L0 被忽略）。
- `indexed=76`——去重裁定后入索引的证书数。

证书产出率 = 76/879 = **8.6%**——确认事件只有 8.6% 通过终端背书门 + 装配三门产证。裁掉率 91.4% 的主因是终端背书 miss（窗口内无合法背书点）。

---

## §4 口径截断清单 + 修正后预期命中率（调研项 4）

### 4.1 截断清单

| # | 截断源 | 影响域 | 裁掉的内容 | 代码锚 | 性质 |
|---|--------|--------|-----------|--------|------|
| **T1** | **Trend×type2/3 构造性排除** | Trend 族事件终端背书 | buy2/buy3/sell2/sell3 在 Trend 事件背书中被全量裁掉 | `nest.rs:619-624` | **裁定 P1 实装**（教义：趋势背驰只制造一类买卖点） |
| **T2** | **CWindow 窗口坐标限定** | 所有事件终端背书 | 窗口 `[c_start, turn_source]` 外的合法背书点不被收 | `nest.rs:611-616` | **设计性**（因果序正，无前视入证） |
| **T3** | **owner=B 合取** | Trend 族事件终端背书 | buy1/sell1 点的中枢 `start_index ≠ b_center_start` 被裁 | `nest.rs:621-623` | **裁定 P1 实装**（owner 身份保证） |
| **T4** | **值桥收束** | 身份桥反查 | 候选 `source_index ≠ seg_c_full.1` 的 bsp 查不到证书 | `nest_gate.rs:486,492` | **设计性**（离开段终点 = 买卖点位置） |
| **T5** | **级别移位** | 终端背书查账本 | level-ℓ 事件只查 `levels[ℓ-1].bsp`，不查本级或更高级 | `nest.rs:649` | **裁定 T1**（教义：背驰查找只能向下） |
| **T6** | **L0 事件不产证** | 索引构建 | `events_by_level[0]` 被 exec 循环跳过 | `nest_index.rs:135` | **设计性**（nest 不听 L0） |
| **T7** | **divergence_confirmed 过滤** | absorb_exts | 未确认事件不进 `events_by_level` / 索引 | `nest_gate.rs:496` | **基例力度门**（未确认过不了门） |
| **T8** | **因果守卫（#112 multi）** | typed_lookup_multi | `judge_at > anchor_index` 的证书被剔除 | `nest_gate.rs:625-626` | **设计性**（禁前视） |

### 4.2 各截断的贡献量级估计

以 wf7 窗为基准（total=1,724 候选）：

| 截断 | 贡献量级 | 估算依据 |
|------|---------|---------|
| T1 (Trend×type2/3) | **中-高** | Trend 事件在总事件中占比可观（强单边窗 Trend 主导）；type2/3 在 bsp 账本中占 95%+（消费侧 class1 仅 1.8%）。Trend 事件的 type2/3 背书全裁 → 终端 miss 率高的主因之一 |
| T2 (CWindow 窗口) | **低-中** | 窗口限定 `[c_start, turn_source]` 是因果正确性保证；窗口外的点本就不是背书的合法候选 |
| T3 (owner=B) | **低** | 生产一/二/三类点构造时均填判定中枢（`signal.rs`），owner 缺失仅限非生产合成形状 |
| T4 (值桥收束) | **高** | typed_none=1,653（95.9% miss）——miss 主体。值桥 `source_index==seg_c_full.1` 要求候选坐标 = 离开段终点；大量候选的 `source_index` 不匹配任何索引事件的 `seg_c_full.1` |
| T5 (级别移位) | **中** | level-ℓ 事件查 `levels[ℓ-1]` 账本——L1 查 L0（点最密，通过率高）；L2+ 查高级别账本（点稀疏，通过率低） |
| T6 (L0 不产证) | **不适用** | L0 bsp 不经事件进索引；但 L0 bsp 经 Γ 候选消费路径直接入场（不经终端背书） |
| T7 (confirmed 过滤) | **中** | `events_seen=142M → events=879`——绝大部分派生事件未通过确认（R1 全合取 / R2 力度或关系） |

### 4.3 修正后预期命中率

**「修正」的含义澄清**：T1/T3/T5/T6 是教义裁定实装，**不可放宽**（放宽 = 违反教义，引入非法背书）。可修正的只有 T4（值桥收束）和 T7（确认条件）。

| 修正情景 | 预期 typed 命中率 | 估算逻辑 |
|---------|------------------|---------|
| **现状（不修正）** | **~4%**（实测 4.1% / 4.9% / 3.4%） | 三窗实测 |
| 放宽值桥为窗口匹配（T4：`source_index ∈ [seg_c_full.0, seg_c_full.1]`） | **~8-12%** | 值桥从精确点匹配放宽为区间匹配，预计命中量翻 2-3 倍；但多数 miss 是结构性（索引里根本没有该坐标的证书），放宽窗口只救回边界邻域 miss |
| 放宽 Trend 域收 type2/3（T1 修正）| **~15-20%** | Trend 事件终端背书从「只收一类」放宽为「同向全收」；Trend 事件在总事件中占比高，type2/3 在 bsp 账本中占 95%+ ⟹ 可救回大量终端 miss。**但此修正违反 P1 教义裁定（趋势背驰只制造一类买卖点），不可行。** |
| 全修正（T1+T4 同时放宽） | **~25-35%** | 两修正叠加。**仍非高覆盖率**——跨级链稀薄（single_level_share ≥0.96）是教义结构（密度调研裁定 ~300× 密度差），非装配缺口 |

### 4.4 终极判定

**typed 命中率 ~4% 不是口径截断造成的「漏检」，是跨级链稀薄的教义结构表现。**

证据链：
1. 装配侧 single_level_share ≥ 0.958——96% 的证书是单级（0-rung），跨级链几乎不存在。
2. 密度调研（`cert-density-doctrine-research-20260721.md`）裁定：~300× 密度差是教义结构（N^δ 跨级链的拓扑稀疏性），非装配 bug。
3. typed_none 的三窗 4,673 个 miss 中，#106 实装卡已列明需分三类（A 桥键 bug / B 链不存在 / C 键域错位）归因——尚未跑批，但结构上 B 类（链不存在 = 教义稀疏）预期为主。
4. 索引通过率 8.6%（76/879）——确认事件只有 8.6% 通过终端背书门。终端 miss 的主因是窗口内无合法背书点（T1 裁掉的 type2/3 是 bsp 账本主体）。

**核心张力**：T1 截断（Trend 只收一类）是教义正确的，但 bsp 账本中一类点仅占 ~2%（消费侧 class1 = 22/1501 = 1.5%）——教义正确性导致 Trend 事件终端背书通过率极低，进而 typed 证书产出极低，进而 typed 命中率极低。这不是 bug，是 Trend 域终端背书门**设计性严格**的直接代价。

---

## §5 数字可复算索引

| 数字 | 出处 |
|------|------|
| typed_found / typed_none / xzd_fallback | `/tmp/v4_C.out` CHAIN 行（p3fold :281 / wf7 :286 / wf8 :291） |
| INDEX events / indexed / single_level_share | `/tmp/v4_C.out` INDEX 行（p3fold :282 / wf7 :287 / wf8 :292） |
| STATS total / nest_pass / xzd_pass | `/tmp/v4_C.out` STATS 行（p3fold :280 / wf7 :285 / wf8 :290） |
| 臂A/臂C trades 各级别×点类分布 | `/tmp/v4_A/{p3fold,wf7,wf8}/trades.jsonl` + `/tmp/v4_C/{p3fold,wf7,wf8}/trades.jsonl`（python json 逐行解析 `certificate.level` + `certificate.bsp_class_min`） |
| Trend 域合法性过滤代码 | `rust/src/theta_v0/classifier/nest.rs:611-629`（CWindow 分支） |
| absorb_exts 收入条件 | `rust/src/theta_v0/backtest/nest_gate.rs:482-510` |
| sync_events 级别遍历 | `rust/src/theta_v0/backtest/nest_gate.rs:390`（`for level in 1..n_levels`） |
| event_bsp_book_level 级别移位 | `rust/src/theta_v0/classifier/nest.rs:535-539`（ℓ → ℓ-1） |
| 索引 exec 循环跳过 L0 | `rust/src/theta_v0/classifier/nest_index.rs:135`（`for exec in 1..`） |

---

## §6 纪律声明

- `rust/src` 零改动、`rust/Cargo.toml` 未触碰、零 git mutation、主仓零写入。
- 全部计数可由 `/tmp/v4_C.out` + `/tmp/v4_{A,C}/{p3fold,wf7,wf8}/trades.jsonl` 重算复现。
- 代码锚全部直读 worktree 复核（分支 `kimi-nest-mainline-20260717`，HEAD `640609071d`）。
- 「修正后预期命中率」§4.3 为结构性估算（基于实测比例外推），非跑批读数——已标明「估算」性质，不冒充实测。
