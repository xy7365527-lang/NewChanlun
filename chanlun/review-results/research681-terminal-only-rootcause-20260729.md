# #681 终态投影驱动 terminal-only 链身份：逐条根因归因（2026-07-29）

**结论（一句话）**：11 条 terminal-only 链身份**全部是 (a) 定义后果，零 (b) 增量发射缺口**——
且不是一枚新的定义后果，而是 **#551 裁定甲已裁的那两个可观测面**（面 A 终态载荷冻结、面 B 身份
消失）经链层的「Hasse 覆盖关系 + 极大路径」口径传导到链身份集合上的结果。判 (b) 不成立的直接机器
证据：**候选身份层面 120 步累计 `fresh_only = 0`**（终态投影侧从未产出因果簿没有的候选 key），
即 #551 甲明文保留的那条**禁止方向**（`fork_fresh_only`，「fresh 凭空多出身份 = 增量宿主漏记」）
在本实测面为零；真实 BTC 20k 面同项亦为 0。

| 桶 | 计数 | 机制 | 候选层定义锚 |
|---|---|---|---|
| **A** 父端点区间在因果侧被终态冻结 ⟹ 链边 `C⊆C` 谓词判假 | **7** | 因果簿里已 `Confirmed` 的候选 C 段右端钉死在确认时刻；fresh 每步新簿以**当步**几何首次落簿 ⟹ 父区间更长 ⟹ 吃进更晚的子候选 | #551 §五 **面 A（终态载荷冻结）**；`cand_event.rs:265-278` 终态挡回 |
| **B** 中间级留存候选在因果侧细分该边 ⟹ 非 Hasse 覆盖边 | **4** | 因果簿 append-only 保留了终态侧已消失的 L1 身份，它 `c ⊆ m ⊆ p` 居中 ⟹ 因果侧那条 L2→L0 skip 边不是覆盖边，短路径不成链（改由 L2→L1→L0 成链） | #551 §五 **面 B（身份消失/回长）**；#551 裁定甲 `fork_causal_only` |
| ★未解释 | **0** | — | — |

---

## 一、实测面与复现配方

- 面：`chain_fixture(120)`（`rust/src/theta_v0/classifier/mod.rs` 测试族同款；
  `candidate_rich_segments(120)` + 合成 closes），两驱动逐前缀推进链簿，与
  `causal_and_terminal_projection_drives_agree_on_common_chain_keys`（mod.rs:4998）逐字同路。
- 交叉面：真实 BTC 20k（既有诊断 bin，只读，未改）：
  `cargo run --release --bin issue550_event_battery -- <btc_1m_full.json> 20000 5000`。
- **生产面零改动**：探针为临时 `#[cfg(test)]` 文件 `rust/src/theta_v0/classifier/probe681.rs`
  + mod.rs 一行 `#[cfg(test)] mod probe681;`，**跑完即撤**（本报告提交前已 `git checkout` +
  `rm`，工作区仅含本报告）。探针全文见 §七（配方，可原样重放）。
- 运行方式：`cargo test --release --lib theta_v0::classifier::probe681 -- --ignored --nocapture`
  （实测 0.04 s，靶向、非全量重放）。原始输出留在 `/tmp/research-681/probe-run4.txt`（仓外）。

---

## 二、候选层：增量侧没有漏发任何身份（判 (b) 不成立）

逐步（n = 1..120）比对两驱动的候选 `latest`（每 key 最新 revision）：

| 读数 | 值 |
|---|---|
| `fresh_only` 候选 key（终态投影有 / 因果簿无）累计出现次数 | **0** |
| 同上，去重 | **0** |
| 共有 key 上 `state` 或 `interval` 分歧的步数 | 25 |
| 其中 `state` 分歧 | **0**（25 条全是 `Confirmed ↔ Confirmed`） |
| 25 条分歧的唯一差项 | `interval.1`（C 段右端），因果侧恒 **≤** 终态侧 |

分歧样本（原样）：

```
step=71  L1 Pan/Short c_start=232  causal(Confirmed, iv=(232,280))  terminal(Confirmed, iv=(232,288))
step=106 L2 Pan/Short c_start=384  causal(Confirmed, iv=(384,420))  terminal(Confirmed, iv=(384,424))
step=112 L1 Pan/Short c_start=432  causal(Confirmed, iv=(432,444))  terminal(Confirmed, iv=(432,448))
```

⟹ 候选**身份**（key）层面因果簿是终态投影的超集（真超集：causal 侧另有终态投影已丢弃的身份），
**没有一个 key 只在终态投影侧出现**。故 terminal-only 链身份不可能由「增量侧漏发某个候选事件」
造成——它只能来自共有候选的**载荷**（C 段右端）与**存活构成**在两驱动上的差异，经链层口径放大。

候选簿机器计数（`cand_event::event_probe`，两驱动各单独跑一遍全 120 步）：

| 驱动 | `terminal_block` | `birth_confirmed` | `growth_revision` | `to_confirmed` | `invalidated_absent` |
|---|---|---|---|---|---|
| 因果簿（共享 cache） | **3167** | 64 | 0 | 0 | 0 |
| 终态投影（每步 fresh） | 345 | 2886 | 0 | 0 | 0 |

读法：因果侧 3167 次后续观察被「终态不复活」挡回（`cand_event.rs:274-278`
`event_probe::on_terminal_block` + `return None`）——**这就是桶 A 的载荷冻结在本面被走过的次数**；
`growth_revision = 0` 说明本面候选**一出生即 `Confirmed`** 终态，冻结在首次入簿的那一刻立即生效。
终态侧 `birth_confirmed = 2886` = 每步新簿各自重新出生（120 步累计）。

---

## 三、11 条逐条归因（全枚举，无省略）

`terminal_only` 全表（链 key 缩写：`L级·Kind Side·c{c_start}`；同名两条的区别在 `seg_a` /
`parent` 指纹，见 [7]/[8] 备注）：

| # | 链身份（root→leaf） | 终态侧 status / observed_at | 终态侧作为极大路径出现的步 | 桶 |
|---|---|---|---|---|
| 1 | L1·PanShort·c232 → L0·PanLong·c284 | Closed / 288 | 72–79 | **A** |
| 2 | L1·PanShort·c232 → L0·PanShort·c288 | Closed / 292 | 73–79 | **A** |
| 3 | L2·PanShort·c232 → L0·PanShort·c232 | Closed / 396 | 99–120 | **B** |
| 4 | L2·PanShort·c232 → L0·PanShort·c248 | Closed / 396 | 99–120 | **B** |
| 5 | L2·PanShort·c232 → L0·PanShort·c256 | Closed / 396 | 99–120 | **B** |
| 6 | L2·PanShort·c232 → L0·PanShort·c264 | Closed / 396 | 99–120 | **B** |
| 7 | L2·PanShort·c384 → L0·PanLong·c420（leaf `seg_a=(412,416)` / `parent.cs=396`） | Closed / 424 | 106–107 | **A** |
| 8 | L2·PanShort·c384 → L0·PanLong·c420（leaf `seg_a=(404,408)` / `parent.cs=408`） | Closed / 432 | 108–115 | **A** |
| 9 | L2·PanShort·c384 → L0·PanShort·c432 | Closed / 436 | 109–110 | **A** |
| 10 | L2·PanShort·c384 → L1·PanShort·c432 → L0·PanShort·c432 | Closed / 444 | 111–115 | **A** |
| 11 | L2·PanShort·c384 → L1·PanShort·c432 → L0·PanShort·c440 | Closed / 444 | 111–115 | **A** |

全 11 条的节点在终态侧**全部 `Alive`**、全部边 `predicate_holds = true`、`status = Closed`
（即：这些链在终态投影侧是正常成型的链，不是抖动残渣）。

归因方法：对每条链、在它于**终态侧真作为极大路径出现的每一步**上，在**因果侧**同一步的候选视图
里按 `chain_cert` 的构造口径逐项追问为何该路径不成链（节点缺席 → 节点证伪 → 边谓词 → 中间级细分
→ 极大性）。11 条 × 各自出现步 = 全枚举，无采样。

### 桶 A（7 条）：父端点 C 段右端在因果侧被终态冻结

样本（条 [1]，终态首现步 s0 = 72）：

```
L1 Pan/Short c_start=232
  causal  : state=Confirmed iv=(232, 280) observed_at=280 confirmed_at=Some(280) rev=0
  terminal: state=Confirmed iv=(232, 288) observed_at=288 confirmed_at=Some(288) rev=0
L0 Pan/Long  c_start=284
  causal  : state=Confirmed iv=(284, 288) observed_at=288 confirmed_at=Some(288) rev=0
  terminal: state=Confirmed iv=(284, 288) observed_at=288 confirmed_at=Some(288) rev=0
因果侧阻断：边 L1→L0 谓词假（causal parent_iv=(232,280) child_iv=(284,288)）
```

子区间 `(284,288)` 落在终态侧父区间 `(232,288)` 内、**不**落在因果侧父区间 `(232,280)` 内
（`288 > 280`）⟹ `cand_sub::candidate_is_sub` 在因果侧判假 ⟹ 该 (父,子) 对在因果侧根本不是边，
链不存在。

**反事实（机器验证，非推断）**：只把父区间换成终态投影侧在同一步读到的那个值、子区间不动，
谓词是否翻真？桶 A 的 7 条 × 各自出现步 = **21 个 (条, 步) 组合，21/21 全部 `true`**。
⟹ 差异的**唯一**来源就是父候选 C 段右端的冻结，不掺杂其他因素。

条 [10]/[11] 是三节点链，阻断点在 L2→L1 这条边（`causal parent_iv=(384,420)` vs
`child_iv=(432,444)`；终态侧 `parent_iv=(384,444)` 吃得下），L1→L0 两侧都成立。

### 桶 B（4 条）：中间级留存候选在因果侧细分该边

样本（条 [3]，s0 = 99）：

```
L2 Pan/Short c_start=232  causal iv=(232,336) == terminal iv=(232,336)   ← 父区间两侧相同
L0 Pan/Short c_start=232  causal iv=(232,236) == terminal iv=(232,236)   ← 子区间两侧相同
因果侧阻断：边 L2→L0 被中间候选细分×1（非 Hasse 覆盖边）
  见证 = [ L1·PanShort·c232·iv(232,280)·Confirmed·terminal=None ]
```

见证候选 `L1·PanShort·c232`（区间 `(232,280)`）满足 `L0(232,236) ⊆ L1(232,280) ⊆ L2(232,336)`
且级别严格居中 ⟹ 按 `chain_cert` 的覆盖关系（Hasse）口径，L2→L0 在因果侧**不是**覆盖边
（`chain_cert/mod.rs:45-56`：把传递闭包边当链边会「把真实存在且套得住的中间级候选说成跳过」）。
而它在**终态投影侧查无**（`terminal = None`）——即 #551 面 B 的消失身份。四条见证完全相同
（4/4 同一个 L1 身份）。

**配对证据**：因果侧对应产出的是经该 L1 的三节点长链，正落在 `causal_only`（18 条）里：
`L2·PanShort·c232 → L1·PanShort·c232 → L0·{c232, c236, c248, c256, c264}`。
桶 B 不是「因果侧漏了链」，是**同一批候选在因果侧被拼成了更长的链**。

---

## 四、定义锚：哪条条款使增量（因果）侧「合理地」看不见这 11 条

1. **候选层终态不复活（桶 A 的锚）**——`cand_event.rs:201`「每级 append-only 修订簿。终态不复活；
   同投影重跑零 Delta」+ `advance_observation`（`cand_event.rs:265-278`）：`prior.state.is_terminal()`
   ⟹ `on_terminal_block()` + `return None`。故 `Confirmed` 候选的 C 段右端此后永不更新。这与是否
   增量**无关**（同一个簿在全量重算下同样挡回），是簿层语义，不是 cache 层实现。
2. **#551 裁定甲（编排者 2026-07-28）已把这两个面裁为定义后果**——
   `chanlun/review-results/issue551-t2-impl-20260728.md` §五：
   - **面 A 终态载荷冻结**：「Pan 域候选一入簿即 `Confirmed` ⟹ C 段区间冻结在确认时刻；而
     `PanDivCert.seg_c` 此后仍随 bar 推进而变，fresh-full 对同 key 重判得到当前区间」；
   - **面 B 上游塌空/身份消失**：因果簿保留、fresh 侧查无；
   - 裁决：「终态域分叉（载荷差异 **以及 key 集合差异**）不再是违规，而是定义后果，只计量」。
   - 同一裁决保留的**唯一禁止方向**是 `fork_fresh_only`（fresh 有而因果簿无该 key）——本票实测
     该项在 120 段夹具上 **= 0**、BTC 20k 上 **= 0**。
3. **链层口径对上述两者的敏感性（传导机制的锚）**——`chain_cert/mod.rs`：
   - 边 = 包含序的**覆盖关系**（Hasse，`:45-56`）⟹ 链边集合是**候选存活全集**的函数（面 B 一变即变）；
   - 链 = 覆盖图中的**极大路径**（`:58-62`）⟹ 链身份是路径整体，父区间伸缩即改路径（面 A 一变即变）；
   - 边的唯一判定谓词 = `cand_sub::candidate_is_sub`（`:191`），直接读两端点 `interval`
     ⟹ 载荷冻结直接改判边的真值；
   - 模块已明文声明两驱动都是支持的输入语义（`ChainNodeStatus::Absent` 文档 `:147-158`）。

⟹ **增量（因果）侧看不见这 11 条是合理的**：它看到的是「父区间冻结在确认时刻 + 中间级身份全史留存」
这一历史相关视图，在该视图下这 11 条路径要么边谓词不成立、要么不是覆盖路径。反之终态投影侧看不见
`causal_only` 那 18 条，是 #551 甲同一枚裁定的另一面。

---

## 五、真实数据面交叉验证（BTC 20k，既有 bin，只读）

```
ISSUE551_LIFECYCLE states={"Confirmed": 46} growth_revisions=0 payload_revisions=0 transitions=0 invalidations=0
ISSUE551_FORK fresh_identities=41 causal_identities=46 payload_equal=36 payload_differ=5
              payload_differ_live=0 fork_causal_only=5 fork_causal_only_terminal=5
              fork_fresh_only=0 revived=0
ISSUE641_CHAIN bars=20000 chains=18 statuses={"Closed":18} path_lens={2:13, 3:5} extends_some=0
ISSUE641_CHAIN_TRACE nodes_alive=41 nodes_falsified=0 nodes_absent=0
```

- 候选**出生即 `Confirmed`**（46/46、`growth_revisions=0`）⟹ 桶 A 的前提在真实面同样普遍成立，
  **不是 120 段夹具的特性**；
- 载荷差 `payload_differ=5`、其中活假设域 `payload_differ_live=0` ⟹ 差异 100% 落在终态域，
  正是桶 A 机制的真实面读数；
- **禁止方向 `fork_fresh_only=0`**（与本票候选层 `fresh_only=0` 同向）⟹ 真实面同样无「增量漏记」；
- 生产两个消费 bin 均因果簿驱动 ⟹ 真实面 `nodes_absent=0`，本票的 terminal-only 现象在**当前生产
  读数上不可达**（它只在 fresh-book 驱动下出现）。

---

## 六、给编排的裁定候选与后续建议

### 裁定候选（登记用，本票不自裁）

> **链身份双向差集均为定义后果**：`causal_only` 与 `terminal_only` 都不是 `chain_cert` 的缺陷，
> 而是 #551 裁定甲已裁的终态域两面（面 A 终态载荷冻结 / 面 B 身份消失）经链层「Hasse 覆盖 + 极大
> 路径」口径传导的结果。链层因此**只计量、不作集合关系断言**（既不断言超集，也不断言子集）；
> 唯一保留的硬锁 = 共有 key 的证书逐字段相同（豁免候选全集计数 `alive_at_level` /
> `inside_parent`，#676-5 已收窄）+ 双向差集计数 golden 锚（38/31/20、18/11，漂移即改证据）。
> 链层继承 #551 甲的**唯一禁止方向**：候选身份层 `fresh_only`（终态投影有而因果簿无该**候选 key**）
> 非零即红；本票实测 120 段 = 0、BTC 20k = 0。

可选的机器载体（若编排要把本票结论固化，属实施票范围，本票未落）：在
`causal_and_terminal_projection_drives_agree_on_common_chain_keys` 旁增一条**候选层**断言
「逐步 `fresh_only == 0`」——它是 (b) 的直接判据，比链层计数 golden 更贴近禁止方向，且非真空
（causal_only 侧非零证明该比对面有内容）。

### 不转实施票

无 (b)：**不建议**为本票开 classifier 修复票。11/11 有定义锚，无一条落「增量发射缺口」。

### 照实登记的 fog（不在本票裁定范围，供编排判断是否另开票）

桶 A 的传导有一个可交易语义方向的后果：**因果驱动下，父候选 C 段右端冻结在确认时刻 ⟹ 后续才成型
的子候选永远进不了它的链**（本面 7/11 条链因此只存在于 fresh 侧）。这不是 bug，是 #551 甲已知并
接受的代价（出路乙「Pan 首入判 `Provisional`」当时因依赖 E2E-D4 未采纳）。若将来 D4 落地、
或链层要作为消费面，这条代价的量级需要单独评估——本票不重开该裁定，只把它照实登记，并指出重开的
前提条件 = E2E-D4 终局判据到位。

---

## 七、探针配方（跑完即撤，此处留全文以便原样重放）

放置：`rust/src/theta_v0/classifier/probe681.rs`（新文件）+ `mod.rs` 加一行
`#[cfg(test)] mod probe681;`（紧邻 `pub mod chain_cert;` 之后）。
跑：`cargo test --release --lib theta_v0::classifier::probe681 -- --ignored --nocapture`。
跑完撤：`git checkout rust/src/theta_v0/classifier/mod.rs && rm rust/src/theta_v0/classifier/probe681.rs`。

```rust
//! ★临时探针（#681 research，跑完即撤，不入 merge）：terminal-only 链身份逐条归因。

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::super::config::ThetaConfig;
use super::super::parser::ParseLayer;
use super::super::types::{Bar, Direction, Segment};
use super::cand_event::{CandidateEvent, CandidateKey, CandidateState, CandidateStreams};
use super::cand_sub::candidate_is_sub;
use super::chain_cert;
use super::{classify_with_tower_events_incremental, TowerCache};

fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
    Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
}

fn bars_from_closes(vals: &[i64]) -> Vec<Bar> {
    vals.iter().enumerate().map(|(i, &v)| Bar {
        source_index: i, timestamp: i as i64, open: v, high: v, low: v, close: v,
        volume: 1, untradable: false,
    }).collect()
}

/// 与 mod.rs tests 的 `candidate_rich_segments` 逐字同构（复制，非调用——tests 模块私有）。
fn candidate_rich_segments(count: usize) -> Vec<Segment> {
    let mut price = 100_i64;
    let mut state = 0x9e3779b97f4a7c15_u64;
    (0..count).map(|i| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let swing = 20 + ((state >> 32) % 90) as i64;
        let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
        let start = price;
        price += match dir { Direction::Up => swing, Direction::Down => -swing };
        seg(dir, i * 4, i * 4 + 4, start, price)
    }).collect()
}

fn chain_fixture(count: usize) -> (Vec<Segment>, Vec<i64>) {
    let segments = candidate_rich_segments(count);
    let closes: Vec<i64> = (0..=segments.last().expect("非空").end_index)
        .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
        .collect();
    (segments, closes)
}

fn key_str(key: &CandidateKey) -> String {
    format!("L{} {:?}/{:?} c_start={} seg_a={:?} parent(cs={},zd={},zg={}) prev_c={:?}",
        key.level, key.kind, key.side, key.c_start, key.seg_a,
        key.parent.center_start, key.parent.zd, key.parent.zg, key.previous_center_start)
}

fn chain_key_str(key: &chain_cert::ChainKey) -> String {
    let path: Vec<String> = key.path.iter()
        .map(|n| format!("L{}·{:?}{:?}·c{}", n.level, n.kind, n.side, n.c_start))
        .collect();
    format!("rv{} [{}]", key.rule_version, path.join(" → "))
}

/// 每 key 最新 revision（与 `chain_cert::AliveIndex` 同口径）。
fn latest_by_key(streams: &CandidateStreams) -> BTreeMap<CandidateKey, CandidateEvent> {
    let mut latest = BTreeMap::new();
    for stream in streams.iter() {
        for event in stream.iter() { latest.insert(event.key, event.clone()); }
    }
    latest
}

fn alive_events(latest: &BTreeMap<CandidateKey, CandidateEvent>) -> Vec<CandidateEvent> {
    latest.values().filter(|e| e.state != CandidateState::Invalidated).cloned().collect()
}

#[test]
#[ignore = "研究探针（#681），手动 --ignored 跑"]
fn probe681_terminal_only_rootcause() {
    let cfg = ThetaConfig::default();
    let (segments, closes) = chain_fixture(120);

    // ── 两驱动逐步事件流（causal = 共享 cache；terminal = 每步 fresh cache）
    let mut causal_cache = TowerCache::new();
    let mut causal_steps: Vec<(CandidateStreams, usize)> = Vec::new();
    let mut terminal_steps: Vec<(CandidateStreams, usize)> = Vec::new();
    for n in 1..=segments.len() {
        let end = segments[n - 1].end_index.min(closes.len() - 1);
        let layer = ParseLayer {
            segments: Rc::new(segments[..n].to_vec()),
            merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
            ..Default::default()
        };
        let causal = classify_with_tower_events_incremental(&layer, &cfg, &mut causal_cache).2;
        let mut fresh = TowerCache::new();
        let terminal = classify_with_tower_events_incremental(&layer, &cfg, &mut fresh).2;
        causal_steps.push((causal, end));
        terminal_steps.push((terminal, end));
    }

    // ── §1 候选层（判 (b) 的关键面）：每步 fresh 有 / causal 无的候选 key
    println!("\n===== §1 候选层逐步差集（fresh_only = 增量侧漏发的必要条件）=====");
    let mut fresh_only_total = 0usize;
    let mut fresh_only_keys: BTreeSet<CandidateKey> = BTreeSet::new();
    let mut state_mismatch_steps: Vec<(usize, String)> = Vec::new();
    for (i, ((causal, _), (terminal, _))) in causal_steps.iter().zip(terminal_steps.iter()).enumerate() {
        let step = i + 1;
        let causal_latest = latest_by_key(causal);
        let terminal_latest = latest_by_key(terminal);
        let fresh_only: Vec<&CandidateKey> = terminal_latest.keys()
            .filter(|k| !causal_latest.contains_key(k)).collect();
        if !fresh_only.is_empty() {
            fresh_only_total += fresh_only.len();
            for key in &fresh_only {
                fresh_only_keys.insert(**key);
                println!("  step={step} fresh_only 候选 key: {}", key_str(key));
            }
        }
        for (key, t) in &terminal_latest {
            if let Some(c) = causal_latest.get(key) {
                if c.state != t.state || c.interval != t.interval {
                    state_mismatch_steps.push((step, format!(
                        "{} causal(state={:?},iv={:?}) terminal(state={:?},iv={:?})",
                        key_str(key), c.state, c.interval, t.state, t.interval)));
                }
            }
        }
    }
    println!("  fresh_only 候选 key 累计出现次数={fresh_only_total}，去重={}", fresh_only_keys.len());
    println!("  共有 key 上 state/interval 分歧步数={}", state_mismatch_steps.len());
    for (step, text) in state_mismatch_steps.iter().take(30) { println!("    step={step} {text}"); }

    // ── 链层：两侧簿
    let mut causal_book = chain_cert::ChainCertificateBook::default();
    for (streams, end) in &causal_steps { causal_book.advance(streams, *end); }
    let mut terminal_book = chain_cert::ChainCertificateBook::default();
    for (streams, end) in &terminal_steps { terminal_book.advance(streams, *end); }
    let causal_heads = causal_book.heads();
    let terminal_heads = terminal_book.heads();
    let causal_keys: BTreeSet<chain_cert::ChainKey> = causal_heads.iter().map(|c| c.key.clone()).collect();
    let terminal_only: Vec<&&chain_cert::TowerChainCertificate> = terminal_heads.iter()
        .filter(|t| !causal_keys.contains(&t.key)).collect();
    println!("\n===== §2 链层差集：causal={} terminal={} terminal_only={} =====",
        causal_heads.len(), terminal_heads.len(), terminal_only.len());

    // ── 逐条归因
    let mut buckets: Vec<(usize, String, String)> = Vec::new();
    for (idx, cert) in terminal_only.iter().enumerate() {
        println!("\n--- [{}] {}", idx + 1, chain_key_str(&cert.key));
        println!("    terminal 侧：status={:?} observed_at={} revision_at={} revision={} \
                  extendable={} edges={} nodes_status={:?}",
            cert.status, cert.observed_at, cert.revision_at, cert.revision, cert.extendable,
            cert.edges.len(), cert.nodes.iter().map(|n| (n.level, n.status)).collect::<Vec<_>>());
        for edge in &cert.edges {
            println!("      edge {:?} L{}→L{} predicate={} skipped={:?}",
                edge.kind, edge.parent.level, edge.child.level, edge.predicate_holds, edge.skipped_levels);
        }
        let mut terminal_present_steps: Vec<usize> = Vec::new();
        let mut causal_reasons: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, ((causal, _), (terminal, _))) in causal_steps.iter().zip(terminal_steps.iter()).enumerate() {
            let step = i + 1;
            let terminal_paths: BTreeSet<Vec<CandidateKey>> =
                chain_cert::chain_paths(terminal).into_iter().collect();
            if terminal_paths.contains(&cert.key.path) { terminal_present_steps.push(step); }
            let causal_latest = latest_by_key(causal);
            let alive = alive_events(&causal_latest);
            let alive_by_key: BTreeMap<CandidateKey, CandidateEvent> =
                alive.iter().map(|e| (e.key, e.clone())).collect();
            let mut reasons: Vec<String> = Vec::new();
            let mut missing = Vec::new();
            let mut falsified = Vec::new();
            for node in &cert.key.path {
                match causal_latest.get(node) {
                    None => missing.push(*node),
                    Some(e) if e.state == CandidateState::Invalidated => falsified.push(*node),
                    Some(_) => {}
                }
            }
            if !missing.is_empty() { reasons.push(format!("节点未在因果侧出现×{}", missing.len())); }
            if !falsified.is_empty() { reasons.push(format!("节点在因果侧已 Invalidated×{}", falsified.len())); }
            if reasons.is_empty() {
                let root = &alive_by_key[&cert.key.path[0]];
                let leaf = &alive_by_key[cert.key.path.last().unwrap()];
                let terminal_latest = latest_by_key(terminal);
                for pair in cert.key.path.windows(2) {
                    let parent = &alive_by_key[&pair[0]];
                    let child = &alive_by_key[&pair[1]];
                    if !candidate_is_sub(child, parent) {
                        let t_parent = terminal_latest.get(&pair[0]).map(|e| e.interval);
                        let t_child = terminal_latest.get(&pair[1]).map(|e| e.interval);
                        // 反事实：只把父区间换成终态投影侧读到的那个（子区间不变），谓词是否翻真？
                        let counterfactual = t_parent.map(|t| t.0 <= child.interval.0
                            && child.interval.1 <= t.1 && t.0 <= t.1
                            && child.interval.0 <= child.interval.1);
                        reasons.push(format!("边 L{}→L{} 在因果侧谓词假：causal parent_iv={:?} \
                            child_iv={:?} ｜terminal parent_iv={:?} child_iv={:?}\
                            ｜反事实(换父区间为终态侧)={:?}",
                            parent.event_level, child.event_level, parent.interval, child.interval,
                            t_parent, t_child, counterfactual));
                    }
                }
                if alive.iter().any(|o| candidate_is_sub(root, o)) {
                    reasons.push("root 在因果侧有存活父（路径非极大：可上延）".to_string());
                }
                if alive.iter().any(|o| candidate_is_sub(o, leaf)) {
                    reasons.push("leaf 在因果侧有存活子（路径非极大：可下延）".to_string());
                }
                for pair in cert.key.path.windows(2) {
                    let parent = &alive_by_key[&pair[0]];
                    let child = &alive_by_key[&pair[1]];
                    let inserted: Vec<&CandidateEvent> = alive.iter().filter(|m|
                        m.event_level > child.event_level && m.event_level < parent.event_level
                        && candidate_is_sub(child, m) && candidate_is_sub(m, parent)).collect();
                    if !inserted.is_empty() {
                        let witnesses: Vec<String> = inserted.iter().map(|m| format!(
                            "L{}·{:?}{:?}·c{}·iv{:?}·{:?}·terminal={:?}",
                            m.key.level, m.key.kind, m.key.side, m.key.c_start, m.interval, m.state,
                            terminal_latest.get(&m.key).map(|e| (e.state, e.interval)))).collect();
                        reasons.push(format!("边 L{}→L{} 在因果侧被中间候选细分×{}（非 Hasse 覆盖边）\
                            见证=[{}]", parent.event_level, child.event_level, inserted.len(),
                            witnesses.join(", ")));
                    }
                }
                if reasons.is_empty() {
                    reasons.push("★该 path 在因果侧同样应为极大路径（未解释）".to_string());
                }
            }
            causal_reasons.entry(reasons.join(" | ")).or_default().push(step);
        }
        println!("    terminal 侧作为极大路径出现的步：{:?}", terminal_present_steps);
        if let Some(&s0) = terminal_present_steps.first() {
            let causal_latest = latest_by_key(&causal_steps[s0 - 1].0);
            let terminal_latest = latest_by_key(&terminal_steps[s0 - 1].0);
            println!("    节点两侧读数 @terminal 首现步 s0={s0}：");
            for node in &cert.key.path {
                let fmt = |e: Option<&CandidateEvent>| match e {
                    None => "查无".to_string(),
                    Some(e) => format!("state={:?} iv={:?} observed_at={} confirmed_at={:?} rev={}",
                        e.state, e.interval, e.observed_at, e.confirmed_at, e.revision),
                };
                println!("      {}\n        causal  : {}\n        terminal: {}",
                    key_str(node), fmt(causal_latest.get(node)), fmt(terminal_latest.get(node)));
            }
        }
        println!("    因果侧阻断理由分档（理由 → 步区间）：");
        for (reason, steps) in &causal_reasons {
            println!("      [{}步 {}..{}] {reason}", steps.len(),
                steps.first().copied().unwrap_or(0), steps.last().copied().unwrap_or(0));
        }
        let mut focus: BTreeMap<String, usize> = BTreeMap::new();
        for step in &terminal_present_steps {
            for (reason, steps) in &causal_reasons {
                if steps.contains(step) { *focus.entry(reason.clone()).or_default() += 1; }
            }
        }
        println!("    ★仅在 terminal 出现的步上的因果侧理由：");
        for (reason, count) in &focus { println!("      ×{count} {reason}"); }
        let has_predicate = focus.keys().any(|r| r.contains("谓词假"));
        let has_subdivide = focus.keys().any(|r| r.contains("细分"));
        let has_extremal = focus.keys().any(|r| r.contains("存活父") || r.contains("存活子"));
        let has_unexplained = focus.keys().any(|r| r.contains("未解释"));
        let bucket = match (has_predicate, has_subdivide, has_extremal, has_unexplained) {
            (_, _, _, true) => "★未解释（需继续查）",
            (true, false, false, false) => "A：父端点区间在因果侧被终态冻结 ⟹ 边谓词假",
            (false, true, false, false) => "B：中间级存活候选在因果侧细分该边 ⟹ 非 Hasse 覆盖",
            (false, false, true, false) => "C：路径在因果侧非极大（可上/下延）",
            _ => "混合",
        };
        println!("    ⇒ 归属桶：{bucket}");
        buckets.push((idx + 1, chain_key_str(&cert.key), bucket.to_string()));
    }

    println!("\n===== §4 桶汇总 =====");
    let mut bucket_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (idx, key, bucket) in &buckets {
        println!("  [{idx}] {bucket}  ←  {key}");
        *bucket_counts.entry(bucket.clone()).or_default() += 1;
    }
    for (bucket, count) in &bucket_counts { println!("  桶计数：{bucket} = {count}"); }

    // ── §5 机器计数：候选簿的「终态挡回」在两驱动上各被走过多少次
    {
        use super::cand_event::event_probe;
        event_probe::reset();
        let mut cache = TowerCache::new();
        for n in 1..=segments.len() {
            let end = segments[n - 1].end_index.min(closes.len() - 1);
            let layer = ParseLayer {
                segments: Rc::new(segments[..n].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
                ..Default::default()
            };
            let _ = classify_with_tower_events_incremental(&layer, &cfg, &mut cache);
        }
        let causal_probe = event_probe::snapshot();
        event_probe::reset();
        for n in 1..=segments.len() {
            let end = segments[n - 1].end_index.min(closes.len() - 1);
            let layer = ParseLayer {
                segments: Rc::new(segments[..n].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
                ..Default::default()
            };
            let mut fresh = TowerCache::new();
            let _ = classify_with_tower_events_incremental(&layer, &cfg, &mut fresh);
        }
        let terminal_probe = event_probe::snapshot();
        println!("\n===== §5 候选簿 event_probe（causal vs terminal）=====");
        println!("  causal  : terminal_block={} growth_revision={} birth_confirmed={} \
            to_confirmed={} invalidated_absent={} invalidated_shrink={} idempotent_skip={}",
            causal_probe.terminal_block, causal_probe.growth_revision, causal_probe.birth_confirmed,
            causal_probe.to_confirmed, causal_probe.invalidated_absent,
            causal_probe.invalidated_shrink, causal_probe.idempotent_skip);
        println!("  terminal: terminal_block={} growth_revision={} birth_confirmed={} \
            to_confirmed={} invalidated_absent={} invalidated_shrink={} idempotent_skip={}",
            terminal_probe.terminal_block, terminal_probe.growth_revision,
            terminal_probe.birth_confirmed, terminal_probe.to_confirmed,
            terminal_probe.invalidated_absent, terminal_probe.invalidated_shrink,
            terminal_probe.idempotent_skip);
    }

    // ── 反向对照：causal_only 也打印一份 key 概览（#551 甲管方向，只作对照）
    let terminal_keys: BTreeSet<chain_cert::ChainKey> = terminal_heads.iter().map(|c| c.key.clone()).collect();
    let causal_only: Vec<&&chain_cert::TowerChainCertificate> = causal_heads.iter()
        .filter(|c| !terminal_keys.contains(&c.key)).collect();
    println!("\n===== §3 causal_only={} （对照）=====", causal_only.len());
    for cert in &causal_only {
        println!("  {} status={:?} observed_at={}", chain_key_str(&cert.key), cert.status, cert.observed_at);
    }
}
```

---

## 八、未做项与诚实边界

- **未改任何生产文件**：`rust/src/` 在本报告提交时与 `main`（23a1869849）逐字相同（探针已撤）；
  本票不落任何断言、不改 golden、不放宽任何锁。
- **未跑 BTC 面的逐前缀双驱动链簿对拍**：链的覆盖边计算是 O(n²)，逐 bar 推进在 20k 上不可行
  （既有 bin 也用 `chain_every=5000` 节拍）。真实面结论仅到「桶 A 的候选层前提普遍成立 +
  禁止方向为 0」这一层，**没有**声称真实面上 terminal-only 也恰好 11 条或同样分桶。
- **桶 A/B 的比例是本夹具的形状，不是规律**：7/4 随夹具与窗口变。定性结论（两桶都是定义后果、
  零 (b)）由候选层 `fresh_only=0` + 反事实 21/21 + 见证 4/4 支撑，与比例无关。
- **`chain_key_str` 缩写不含 `seg_a`/`parent` 指纹**：表中 [7]/[8] 打印相同但是两个不同
  `CandidateKey`（备注已标出差异字段）；差集判定用的是完整 `ChainKey`，不受缩写影响。
- 本票**不关 issue、不登记裁定**——裁定候选（§六）交编排裁决。
