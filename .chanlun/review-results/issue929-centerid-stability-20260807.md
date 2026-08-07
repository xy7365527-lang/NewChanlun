# #929 探针：`CenterId` 三分量在中枢延伸 / 升级重切下的稳定性

- 票：#929（`wayfinder:task`，挂 map #787），喂 [ADR 0019](../../docs/adr/0019-limit-order-state-flow-interface.md) 裁定三实装线
- 类型：AFK 只读勘察（未改任何 `.rs`）
- 基线：本地 `main` = `1a07ebeb73`（worktree 初始 HEAD 是祖传废线 `19b4015927`，开工前已 `git reset --hard main` 对齐）
- 日期：2026-08-07

---

## 0. 三选一结论

**只能用 `start_index`**——但**不是**因为「延伸改核心」，而是因为**生产从不以「单次扫描内的延伸」实现延伸**。

两条承重句：

1. **单次扫描内的 Step2 延伸确实冻结核心**：`rust/src/theta_v0/classifier/recursive_tower.rs:745-752` 的延伸循环只写 `c.end_index` / `c.dd` / `c.gg`，`zd`/`zg`/`start_index` 一个字节不动。#927 的 doc 旁证（`recursive_tower.rs:264-267`）与实现一致，**未漂移**。
2. **但生产的「延伸」= 每 bar pop 开放窗 + 以同 seed 重扫**：`recursive_tower.rs:709-711` 的 bit-exact 充要条件第 1 条逐字——「frontier 协议天然正确：**pop 开放中枢 + 从其 seed 起点重扫** ⟹ 延伸在重扫中吸收新单元，bit-exact」。重扫时 seed 三单元里的 frontier 源外缘可能已被修订 ⟹ `zd = max(lo₁,lo₂,lo₃)` / `zg = min(hi₁,hi₂,hi₃)`（`center.rs:223-230` / `:260-267`）重新求交 ⟹ **中枢还活着，`zd`/`zg` 却变了**。

这条不是推理，是**已落盘的生产实测**：`.chanlun/review-results/rebase-identity-d1-research-20260728.md:80-87` 的八条 wf8 案例中 **7 条**是「同 seed、`start_index` 不变、父 `zd` 或 `zg` 漂移、归因为**连续候选非分裂**」。逐数演算见该文 §3.4（`:220-236`，seq=3，L2）：

```
start_index = 24614           （不变）
old zd = max(2545760000000, 2556555000000, 2580000000000) = 2580000000000
new zd = max(2545760000000, 2556555000000, 2571704000000) = 2571704000000
```

即 **`start_index` 相同、`zd` 变 ⟹ 全三元组键会把「中枢还在」误判成「中枢失效 ⟹ 撤单」**，正是 #929 票面判据里「会变 ⟹ 只能用 `start_index`」那一支。

同一结论已被仓内代码独立坐实：`rust/src/theta_v0/lineage_book.rs:5-8` 模块 doc 逐字——

> 开放 frontier 每 bar 被 pop 后重扫，`CenterId=(start_index, zd, zg)` 是当次前三个构造单元的派生值：**第三源外缘一改，父核心就重新求交，旧三元组消失、新三元组出现**……这是**工程丢身份**，不是教义生死。

`LineageBook`（#679 D1b）是**生产默认开**的（`lineage_book.rs:27-29`：建簿只看 `consumer_enabled`，默认开，不受 `OPSEM_DUMP_DIR` 门控），接线点 `rust/src/theta_v0/backtest/fill.rs:1017-1022`。**这条修补回路存在本身，就是「全三元组不稳」的在产证明。**

### 三个分量各自的裁定

| 分量 | 单次扫描内延伸 | 九段升级重切 | frontier pop+重扫（生产实际路径） |
|---|---|---|---|
| `start_index` | **不变**（`recursive_tower.rs:745-752` 不触） | 头子**不变**、后 k−1 子为新身份 | **7/8 实测不变**；2 例（seq=66/59）右移，但那两例中枢确已重建（D1 裁「拒绝过继，保持核销」）⟹ 撤单是**正确**行为 |
| `zd` | 不变 | 不变（`:783` 继承 `c.zd`） | **会变**（seq=3/20 实测） |
| `zg` | 不变 | 不变（`:784` 继承 `c.zg`） | **会变**（seq=47/53/56 实测） |

`start_index` 自己**没有**「中枢还在但键变」的实测反例 ⟹ 不触发第三选项（三分量都不可用）。

---

## 1. `Center.zd` / `Center.zg` / `Center.start_index` 全部写入点清单

### 1.1 穷举方法（检索式与覆盖域，逐条可复跑）

覆盖目录 = 仓根全量，排除 `.git` / `target` / `node_modules`；即 `rust/`（`src/`+`tests/`+`bin/`）、`formal/`、`src/`、`scripts/`、`tests/`、`prototypes/`、`docs/`、`.chanlun/`、`.rtas/`。

| # | 检索式 | 命中 | 用途 |
|---|---|---|---|
| A | `grep -rn -E "\.(zd\|zg)\s*(=[^=]\|[-+*/]=)" .`（全仓、不限后缀） | 见 §1.3 | 捕获**一切**字段赋值式改写，与变量名无关 |
| B | `grep -rn --include='*.rs' -E "\bCenter\s*\{" rust/src/` | 见 §1.4 | 捕获**一切**结构体字面量构造（含 `..base` 更新语法） |
| C | `grep -rn --include='*.rs' -E "&mut\s+(super::)?(types::)?Center\b\|&'?[a-z]*\s*mut Center\b" rust/src/` | **0** | 排除「借出可变引用到别处改」 |
| D | `grep -rn --include='*.rs' -E "centers\s*\.\s*(iter_mut\|last_mut\|get_mut\|first_mut)" rust/src/` | **0** | 排除「就地遍历改写中枢表」 |
| E | `grep -rn --include='*.rs' -E "(\*\s*c(enter)?[a-z_0-9]*\s*=\|centers\[[^]]*\]\s*=[^=]\|\bmem::(swap\|replace)\b)" rust/src/theta_v0/` | 无 `Center` 命中 | 排除「整体赋值 / swap / replace 换掉整个 Center」 |
| F | `grep -rn --include='*.rs' -E "^impl .*\bCenter\b" rust/src/` | 仅 `impl From<&Center> for PanCenterIdentity`（`level_view/pan.rs:69`，只读） | 排除「`&mut self` 方法改自身」 |
| G | `grep -rn --include='*.rs' "CenterId::of" rust/` | §5 用 | 定位身份键消费面 |

**A/C/D/E/F 合起来穷尽了 Rust 里改写一个已有 `Center` 实例的全部语法形式**（字段赋值、可变引用外借、容器可变迭代、整体覆写、方法内改）。`Center` 的 derive 只有 `Debug, Clone, Copy, PartialEq, Eq`（`rust/src/theta_v0/types.rs:121`）——**无 `Deserialize`**，故不存在 serde 反序列化写入路径；`Copy` 语义也意味着「传出去被改」不会回写原件。

⚠ **未覆盖的形式（照实标注）**：宏展开产生的赋值（本仓 `theta_v0` 未见自定义 derive/宏生成 `Center` 代码，但我没有跑 `cargo expand` 逐 crate 核对）；`unsafe`/FFI 指针写（未作专项核对）。

### 1.2 同名异物排除（票面纪律第 5 条）

- 目标类型唯一：`rust/src/theta_v0/types.rs:122` 的 `pub struct Center`，字段 `zd: Tick` / `zg: Tick`（`:124,126`），`pub type Tick = i64`（`types.rs:15`）。这正是 `CenterId::of` 读的那个——`center_lifecycle.rs:145` `use super::super::types::{BspBits, Center, Side, Tick};`，`CenterId::of(c: &Center)` 在 `center_lifecycle.rs:173-179`。
- **排除**：`rust/src/trading/**` 的 `zd`/`zg` 是 `Option<f64>` / `f64`（`trading/types.rs:251,733`、`trading/center_book.rs:66`、`trading/unified_osc.rs:112`、`trading/level_operating_unit.rs:73`）——**不同结构**。检索式 A 在 `rust/src/trading/` 下命中的 30 余处 `rows[i][j].zd = Some(50.0)` 全部属于它们，且全部在 `#[cfg(test)]` 内。
- **排除**：`rust/src/recursive_t/rec_stream.rs:472-473` 的 `view.zd/zg` 是列视图字段（`Vec`），非 `Center`。
- **排除**：Python（`src/newchan/`、`scripts/`、`tests/`）与 Lean（`formal/`）侧的同名 `zd`/`zg`，非本裁定对象。
- **排除**：检索式 A 在注释/断言里的 `c.zd=100`、`zs.zg` 等**比较式**（`buysellpoint.rs:429,442`、`signal.rs:286,590,609`、`nest.rs:751` 等）——`==`/`<`/`>` 不是赋值。

### 1.3 对已存在 `Center` 实例的字段改写：**已查到 3 处，全部在同一循环体内，全部只碰非身份字段**

| # | 位置 | 字段 | 路径 | 打开确认 |
|---|---|---|---|---|
| 1 | `recursive_tower.rs:748` | `c.end_index` | Step2 延伸吸收 | ✅ |
| 2 | `recursive_tower.rs:749` | `c.dd` | Step2 延伸吸收 | ✅ |
| 3 | `recursive_tower.rs:750` | `c.gg` | Step2 延伸吸收 | ✅ |

**在 `rust/src/theta_v0/` 全域，`.zd = ` / `.zg = ` / `.start_index = ` 的赋值命中数 = 0**（检索式 A）。故 `CenterId` 的三分量**从不被就地改写**——它们只可能在**重新构造**一个 `Center` 时取到不同值。

### 1.4 `Center` 结构体构造点（决定 `zd`/`zg`/`start_index` 取值的全部位置）

检索式 B 在 `rust/src/` 命中 190 余处；按各文件 `#[cfg(test)]` 起始行切分后，**生产（非 test）构造点共 8 处**，逐处已打开确认：

| # | 位置 | `zd`/`zg` 来源 | `start_index` 来源 | 路径归属 |
|---|---|---|---|---|
| 1 | `classifier/center.rs:223-230`（`center_from_segments`） | `compute_zd/zg(a,b,c)` 全三段交 | `a.start_index` | **构造**（L0 完整判据，canonical seed） |
| 2 | `classifier/center.rs:260-267`（`center_from_window`） | 同上（几何判据） | `a.start_index` | **构造**（L≥1 上级 seed） |
| 3 | `classifier/recursive_tower.rs:782-789` | `c.zd` / `c.zg`（**继承**被延伸中枢的核心 = seed 三段交） | `units[s].start_index`，`s = i + 3t` | **升级重切**（#148，见 §3） |
| 4 | `classifier/level_view/projection.rs:157-161` | `carried.zd` / `carried.zg`（**继承塔携带核**，覆盖自算核） | `..own_center` 继承 | 旁路：ExactThree 投影（`level_view`），非塔主链 |
| 5 | `classifier/level_view/projection.rs:170-177` | `carried.zd` / `carried.zg` | `a.start_index` | 同上（自核不成立分支，#95 V3 独占） |
| 6 | `strategy/mod.rs:588-595` | 全 `0` | `0` | **零占位**（`Type1Anchor` 载体、无 3 类 bit，center 不被读） |
| 7 | `strategy/mod.rs:597-604` | 全 `0` | `0` | **零占位**（`None` 载体、无 3 类 bit） |
| 8 | `backtest/signal.rs:129-136` | 全 `0` | `0` | **零占位**（同 6/7 口径） |

补记（不进裁定，照实记）：

- `backtest/wverify_run.rs:2953,2969` 两处虽在 `#[cfg(test)]`（起于 `:3644`）**之外**、会编译进生产，但其 doc `:2941` 明写「★旧弱口径（#321 **前**）完整判据副本——反事实臂，**不是**生产路径」——是 #327 的双口径对照函数，不喂塔链。
- `classifier/mod.rs:3703` 在 `#[cfg(test)] mod tests`（起于 `mod.rs:2134`）内，是「口径 R vs 口径 I」的 BTC 实测审计探针（`:3596` 起 `load_by_symbol("BTC", ...)`）。它构造的 `Center` 用**重算核** `zd_r/zg_r` 配**母中枢 `start_index`**——若哪天上生产，会造出「同 `start_index` 不同核心」的对象，是本裁定的潜在破坏源。目前不是。

⚠ **#4/#5 值得下游注意**（本票范围外）：`projection.rs:157-161` 会在自算核与携带核不等时**用携带核覆盖自算核**（`SeedCoreProvenance::InheritedRecut`）。这意味着「同一个窗口，读 `level_view` 投影 vs 读塔链，`(zd,zg)` 可以不同」。ADR 0019 的快照若同时消费这两个源，必须钉死读哪一个。

---

## 2. 延伸路径判定

**单次扫描内的 Step2 延伸：`CenterId` 三分量全部不变。**

承重：`rust/src/theta_v0/classifier/recursive_tower.rs:745-752`

```rust
let mut c = seed;
let mut j = i + 3;
while j < units.len() && units[j].lo <= c.zg && units[j].hi >= c.zd {
    c.end_index = units[j].end_index;
    c.dd = c.dd.min(units[j].lo);
    c.gg = c.gg.max(units[j].hi);
    j += 1;
}
```

- 只写 `end_index` / `dd` / `gg`，三者**都不在** `CenterId` 里（`center_lifecycle.rs:162-169`）。
- 延伸的**判据本身**读 `c.zg` / `c.zd`（`:747` 循环条件），即核心是延伸的不动锚——写它会自毁循环语义。doc `:264-267` 与实现一致，**未漂移**（这一条 #927 说对了）。
- 仓内已有机检锁：`center_lifecycle.rs:1371-1400` 测试 `center_id_ignores_envelope_and_end_index`，断言 `CenterId::of(&extended) == CenterId::of(&c0)`（`end_index` 改成 4444、外缘 `dd` 50 / `gg` 300 放大，核心与 `si` 不变）。

**但这不是生产实现延伸的方式。** 生产每 bar 走 frontier 协议：

- `recursive_tower.rs:707-711`（bit-exact 充要条件第 1 条）：「最后一个中枢在未出现 non-extension 单元前是**开放**的（尾部追加单元可延伸它）……frontier 协议天然正确：**pop 开放中枢 + 从其 seed 起点重扫** ⟹ 延伸在重扫中吸收新单元，bit-exact」；
- `recursive_tower.rs:712-713`（条件 2）：「`units[..start_i]` 在两次扫描间**不可变**（**只允许尾部追加或 frontier 段原地改写后重扫**）」——即 frontier 段的原地改写是**被允许**的输入；
- 接线在 `classifier/mod.rs:1937-2001`（pop 末个开放构造整窗、从旧 `win_start` 重扫），完整链路图见研究文 `rebase-identity-d1-research-20260728.md:134-185`（§3.1 代码级路径）。

重扫时 `build(&units[i], &units[i+1], &units[i+2])` 重新执行 `center_from_segments`（`center.rs:223-230`）。若 `units[i+2]`（frontier 源）的 `lo`/`hi` 已被下层修订，则核心重新求交出新值，而 `start_index = a.start_index` 不动。

**⟹ 生产口径下「中枢在延伸期间 `CenterId` 会变（`zd`/`zg` 分量），`start_index` 分量不变」。**

### 2.1 生产实测（wf8，八条全表）

源：`.chanlun/review-results/rebase-identity-d1-research-20260728.md:80-87`，现场四流在 `/tmp/main489-on.I7S288/`。

| seq / bar / L | 现象 | 归因 |
|---|---|---|
| 3 / 27124 / L2 | 同 seed `#12,#13,#14`，父 `zd` 漂移 | 连续候选，非分裂 |
| 20 / 55967 / L2 | 同 seed `#27,#28,#29`，父 `zd` 漂移 | 连续候选 |
| 24 / 85532 / L1 | 同 seed，3 源→5 源延伸吸收 | 连续候选（阈值未到 9） |
| 47 / 170041 / L1 | 同 seed，父 `zg` 漂移 | 连续候选 |
| 53 / 193252 / L1 | 同 seed，父 `zg` 漂移 | 连续候选 |
| 55 / 193486 / L1 | 同 seed，3→5 源延伸 | 连续候选 |
| 56 / 200401 / L1 | 同 seed，父 `zg` 漂移 | 连续候选 |
| **66 / 240310 / L1** | 旧 seed 交集失效，窗口右移一位，`start_index` 238178→238612 | **旧窗撤出 + 新窗建立，拒绝过继** |

7/8 = 中枢连续但 `zd`/`zg` 变；1/8 = `start_index` 变且中枢确已重建。

### 2.2 仓内代码层的独立佐证（机检，非报告）

`rust/src/theta_v0/classifier/rebase_txn.rs:1469-1487` 测试 `continued_center_pairs_only_admits_fully_bijective_continued_edges` 第①段——helper 参数序为 `center(start, end, zd, zg, dd, gg)`（`rebase_txn.rs:1109-1118`）：

```rust
let old_c = center(100, 130, 1080, 1100, 1000, 1180);  // start=100, zd=1080, zg=1100
let new_c = center(100, 136, 1060, 1100, 1000, 1200);  // start=100, zd=1060, zg=1100
...
assert_eq!(
    continued_center_pairs(&txn.old_nodes, &txn.new_nodes, &txn.edges),
    vec![(CenterId::of(&old_c), CenterId::of(&new_c))]
);
```

注释 `:1470` 逐字：「① **同 seed、第三源修订** ⟹ 一条可**过继**的连续边」。即：**同一个中枢（`continued_1to1` ∧ functional/injective/unique/bijective 四布尔全真），`start_index` 不变、`zd` 从 1080 变 1060**。这是仓内 CI 每次都跑的机检断言，不是文档声明。

`continued_center_pairs` 本体在 `rebase_txn.rs:744-773`，返回类型 `Vec<(CenterId, CenterId)>`——**「旧身份 → 新身份」这个签名本身就承认了身份会变**。

---

## 3. 升级重切路径判定（#927 完全未查的那条）

`recursive_tower.rs:767-800`：窗口总段数 `n ≥ UPGRADE_TOTAL_SEGMENTS`（9）时，整窗按每 3 段重切为 `k = n / 3` 个本级子中枢。

**逐分量裁定（承重 `recursive_tower.rs:782-789`）：**

| 分量 | 重切后 | 承重行 |
|---|---|---|
| `zd` | **不变**——`zd: c.zd`，`c` 是被延伸的母中枢，其核心 = seed 三段交 | `:783` |
| `zg` | **不变**——`zg: c.zg` | `:784` |
| `start_index` | **头子（`t=0`）不变**：`s = i + 0*3 = i`，`units[i].start_index` = seed 构造时的 `a.start_index`（`center.rs:228` / `:265`，`a = units[i]`）⟹ 与母中枢逐值相同。**后 `k−1` 个子（`t≥1`）取 `units[i+3t].start_index`**，是新坐标 | `:787` |

（`dd`/`gg`/`end_index` 会收缩到各自子窗——`:785-786,788`——但那三个字段不在键里。）

**含义（两条，都对裁定三承重）：**

1. **升级重切不会让「已挂单所绑的中枢」换身份**——头子完整继承母中枢的 `(start_index, zd, zg)` 三元组，`CenterId` 逐位不变。仓内独立佐证：`lineage_book.rs:35-36` 的宽读法裁定逐字——「**下级分裂头子继承父种子**——头子唯一保住窗口起点与核心，身份延续无歧义」（用户 2026-07-29 裁定，已转生产默认；严格读法留档不采用，`THETA_REBASE_MIGRATE_STRICT=1` 切回）。
2. **`(zd, zg)` 单独不足以做键**：重切后 `k` 个子中枢**共享同一 `(zd, zg)`**（`:783-784` 全部继承 `c`），只有 `start_index` 能区分它们。所以无论选哪个方案，**`start_index` 必须在键里**。

⚠ **未能实测确认**：§2.1 的 wf8 八条案例**没有一条触发九段重切**——研究文 `:185` 明写「九段升级是另一条可导致 1→N 的重切路径，代码在 `recursive_tower.rs:756-790`，但**本八条的探针没有触发它**」。所以「头子身份保持」是**从代码逐行读出的**，**不是实测**。下游若要把它当承重，建议补一条 wf8 实测（探针票）。

---

## 4. `start_index` 自身稳定性（票面第 3 件）

**不是恒稳，但没有触发第三选项（三分量都不可用）。**

已查到 2 个 `start_index` 漂移实例，均在同一份 wf8：

- **seq=66**（研究文 §2.3 `:96-124`，逐数演算）：旧 seed `#494,#495,#496` → `start=238178, zd=4311413000000, zg=4317400000000`；`#496` 外缘收缩为 `[4321576000000, 4347000000000]` 后旧三元交集变空（`max(dd)=4321576000000 > min(gg)=4317400000000`）⟹ 扫描 `i += 1`，新 seed `#495,#496,#497` → `start=238612, zd=4321576000000, zg=4347000000000`，`#498` 随后作延伸吸收。D1 裁定：**旧构造窗撤出 + 新构造窗建立，拒绝过继、保持核销**。
- **seq=59**（研究文 §3.3 `:203-218`）：旧 `#414,#415,#416` → `200900/4269983000000/4346171000000`；新 `#416,#417,#418` → `202536/4391001000000/4423309000000`。**没有任何逐值相等的 direct source edge**。

两例的共同点：**中枢本身确实换了**（seed 成员集合变了），撤单重挂是**正确**语义，不是误判。所以「`start_index` 会变」在这两例里**不构成**裁定三的反例。

⚠ **照 090 标注**：我**没有**找到「中枢连续（`continued_1to1` ∧ 四布尔全真）但 `start_index` 变」的实例；也**没有**找到证明它不可能的代码论证。`recursive_tower.rs:777-789` 的重切路径确实产出 `start_index` 不同的兄弟子中枢，但那些是**新中枢**、不是同一个。**这一项是「已查到的证据里没有反例」，不是「已证不可能」。**

---

## 5. 对裁定三的直接建议（不改裁，只报事实）

1. **身份键必须包含 `start_index`**——否则升级重切后 `k` 个子中枢撞键（§3 第 2 点）。
2. **身份键不能包含 `zd`/`zg`**——否则 7/8 的 frontier 修订会把活着的中枢误判成失效 ⟹ 与 ADR 0018 裁定四「寿命绑中枢存续」正相反（§0/§2）。
3. **⚠ 但「键相同」不再蕴含「价格不变」。** 这是从「全三元组」降到「只 `start_index`」的**代价**，ADR 0019 裁定三现在的表述（「键相同即续挂，**qty 冻结**、判 maker」）在这条上有个缺口：同一 `start_index` 下 `zd`/`zg` 可以漂移（seq=3 实测 `zd` 2580000000000 → 2571704000000，约 −0.32%），而限价单的挂价正是从 ZD/ZG 读的。键说「续挂」，价却该改了。

   本仓已有**同形先例**可直接抄：`center_oscillation_trade.rs:619-631` 的 `on_trigger_side_bound`（#487 生产接线入口）在**开局腿首次绑定时冻结完整中枢四边框**，后续清算按旧框判（ADR 补充十四），doc 逐字「重复触发只延续首次冻结框」。ADR 0019 若照抄，语义就闭合：**键管身份、冻结框管价格**。

4. **更强的载体已经在产**：`CenterOscillationBook::resolve()`（`center_oscillation_trade.rs:516-522`，doc 逐字「这是挂起表的**唯一 key 归一入口**」）把「当前链身份」折回「首次绑定的稳定锚」，背后是 `LineageBook`（`lineage_book.rs`，生产默认开，接线 `fill.rs:1017-1022`，证书缺失/歧义/多认领一律 **fail-closed**）。裁定三若直接以 `resolve()` 后的锚为键，能同时吃到「`zd` 漂移不误撤」与「seed 右移正确撤」两头，且不必自建第二套身份。**这属于改裁范围，本探针不自行裁定，交回主控。**

---

## 6. 未能确认的部分（照实，不省略）

1. **九段升级重切的「头子身份保持」是代码读出、非实测**——wf8 八条案例无一触发（研究文 `:185`）。
2. **`cargo expand` 未跑**：宏展开产生的 `Center` 字段写入未作机械排除（§1.1 A–F 是源码层穷举，不是展开后穷举）。
3. **`unsafe`/FFI 写入未作专项核对**。
4. **「中枢连续但 `start_index` 变」是否可能，未证亦未反证**（§4）。
5. **首个脏源来自 parser 线段重分还是下一级窗口重算，本探针未查**——研究文 §3.5 `:246` 同样标为「**未能判定**：四流没有 dirty 源及跨级 transform，代码两条路径都存在，不能按可能性冒充本案事实」。
6. **`projection.rs:157-161` 的「携带核覆盖自算核」是否落在 ADR 0019 快照的读路径上，未查**（§1.4 ⚠）。
7. **本探针未运行任何测试/回测**（AFK 只读口径）——全部结论来自源码逐行打开 + 仓内已落盘的 wf8 实测报告。§2.2 的机检断言我读了源码但**没有实际执行** `cargo test`。

---

## 附：援引清单（逐条已打开确认，非 grep 命中数）

| 援引 | 内容 |
|---|---|
| `rust/src/theta_v0/types.rs:15` | `pub type Tick = i64` |
| `rust/src/theta_v0/types.rs:121-133` | `struct Center`（derive 无 `Deserialize`） |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:145` | `use ... types::{BspBits, Center, Side, Tick}`（确认同一个 `Center`） |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:161-169` | `CenterId { start_index, zd, zg }` 定义 |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:171-180` | `CenterId::of(&Center)` |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:1371-1400` | 机检锁：延伸不改身份 |
| `rust/src/theta_v0/classifier/recursive_tower.rs:264-267` | doc：ZD/ZG 核心冻结（#927 旁证，实测未漂移） |
| `rust/src/theta_v0/classifier/recursive_tower.rs:707-713` | frontier 协议：pop 开放中枢 + 从 seed 起点重扫；frontier 段原地改写被允许 |
| `rust/src/theta_v0/classifier/recursive_tower.rs:745-752` | Step2 延伸：只写 `end_index`/`dd`/`gg` |
| `rust/src/theta_v0/classifier/recursive_tower.rs:767-800` | #148 九段升级重切 |
| `rust/src/theta_v0/classifier/recursive_tower.rs:783-784,787` | 子中枢 `zd`/`zg` 继承母核心；`start_index` 取 `units[s]` |
| `rust/src/theta_v0/classifier/center.rs:223-230` | `center_from_segments`：`start_index = a.start_index` |
| `rust/src/theta_v0/classifier/center.rs:260-267` | `center_from_window`：同上 |
| `rust/src/theta_v0/classifier/level_view/projection.rs:150-180` | 携带核覆盖自算核（旁路，`SeedCoreProvenance`） |
| `rust/src/theta_v0/classifier/rebase_txn.rs:1-30` | D1a 模块 doc：现行三流「旧 `CenterId` 三元组消失、新三元组出现」 |
| `rust/src/theta_v0/classifier/rebase_txn.rs:744-773` | `continued_center_pairs` → `Vec<(CenterId, CenterId)>` |
| `rust/src/theta_v0/classifier/rebase_txn.rs:1109-1118` | 测试 helper `center(start, end, zd, zg, dd, gg)` 参数序 |
| `rust/src/theta_v0/classifier/rebase_txn.rs:1469-1487` | 机检：同 seed 第三源修订 ⟹ `(100,1080,1100) → (100,1060,1100)` 判**连续** |
| `rust/src/theta_v0/lineage_book.rs:1-8` | 谱系簿 doc：`CenterId` 丢身份的机理（「工程丢身份，不是教义生死」） |
| `rust/src/theta_v0/lineage_book.rs:27-29` | 建簿生产默认开，与 `OPSEM_DUMP_DIR` 解耦 |
| `rust/src/theta_v0/lineage_book.rs:33-39` | 宽读法转生产默认；「下级分裂头子继承父种子」 |
| `rust/src/theta_v0/backtest/fill.rs:1013-1022` | 谱系簿生产接线 |
| `rust/src/theta_v0/strategy/center_oscillation_trade.rs:516-522` | `resolve()`：当前身份 → 稳定锚，挂起表唯一 key 归一入口 |
| `rust/src/theta_v0/strategy/center_oscillation_trade.rs:619-631` | `on_trigger_side_bound`：#487 首次绑定冻结四边框 |
| `.chanlun/review-results/rebase-identity-d1-research-20260728.md:12` | 「第三个源单元的外缘修订，`zd/zg` 就可能改变」 |
| 同上 `:80-87` | 八条 wf8 案例全表 |
| 同上 `:96-124` | seq=66 逐数演算（`start_index` 右移） |
| 同上 `:134-185` | 代码级路径图 + 关键公式 + 「七条同 start 案例不是延伸直接改 CenterId，而是先 pop、再以同 seed 重算」+ 九段重切未触发 |
| 同上 `:203-218` | seq=59 逐数演算（seed 右移两位） |
| 同上 `:220-236` | seq=3 逐数演算（同 seed、仅核心漂移） |
| 同上 `:238-247` | §3.5 裁定表（含「首个脏源」未能判定） |
