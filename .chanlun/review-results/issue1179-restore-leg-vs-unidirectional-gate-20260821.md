# issue #1179：restore 祖先腿是否绕过重内单向门

- 日期：2026-08-21
- main 基线：`2c840640f6b80fb880561e9349e88514f26866e4`（`docs(workflows): #1159 …`）
- 工作区：`ticket-919-final`（不含 S1 代码，本报告全部证据取自 `git show main:<path>` / `git grep main`）
- 方法：只读静态走查 + 生产代码与既有测试证据对位；未改任何源码、未跑回测。

---

## 〇、一句话结论

**旁路不成立。** `restore_ancestor_chain_from_registry` 推入 `work/raw` 的恢复祖先腿，在现役
main 中**没有第二条出口**：`raw → next_idx → strategy_target_legs → enforce_chong_unidirectional`
是必经路径（`step.rs:598-622`），门在净额折叠 `net_target_units` 与 `sep_legs` 打包之前对
**全部** `LegTarget`（含 restore 腿）逐腿生效。候选段相反信号方向也进不了恢复腿：
`#446` 门禁排除了候选拷贝复用，恢复腿方向取 registry 持久方向 / base 树方向 / 持仓腿方向，
而不是候选 eps。反向恢复腿被**零化单位**（不是钳向、不是跳过、不是报错），随后
`q<=0` 过滤使反向声部根本进不了 overlay/LEE 账本；`Chong.position_lots` 只承载净额成交后的
单一有符号 lot，不可能经 restore 腿重新出现跨级反向**持仓**。

---

## 一、恢复腿去向链（逐站）

### 站 1：restore 把祖先元素 push 进 `work` 并 `raw.push`

调用点只有两处生产路径：

- **held 路**（A_t 段）：`HeldLegState::LiveDetached` 且 `leg.op_parent=Some` 时，
  `step.rs:259-274` 调用 restore；随后 `step.rs:281-292` 重注册触发腿自身入 raw。
- **open 路**（ℬ_x 段）：开仓候选父 carrier 不在 raw 且 `registry.registry_live` 时，
  `step.rs:414-431` 调用 restore。

restore 函数本体在 `held.rs:282-391`，关键 push：

```rust
// held.rs:366-385
let parent_pid = pe.structural_parent_id;
let op_idx = work.len();
work.push(CoverageElement {
    lambda: pe.lambda,
    rho: pe.rho,
    eps: pe.dir,                 // ← 方向源：registry 持久方向
    level: pe.level,
    parent: None,
    attached_dir: None,
    id: pe.pid,
    parent_id: pe.structural_parent_id,
});
overlay_seen.entry(pe.pid).or_insert(op_idx);
pending_parent_fixup.push(op_idx);
raw.push(op_idx);                // ← 恢复祖先直接进 raw
cur = parent_pid;
```

若祖先已在 `work`（base 树前缀或本 bar restore 已 push 的 overlay 元素）但不在 raw，
走复用分支 `held.rs:335-343`：`id_idx`（base 树）优先，`overlay_seen` 仅放行
`idx >= overlay_cand_end`（restore push，持久身份），然后 `raw.push(existing_idx)`。

### 站 2：raw → 活动集 → next_active（下 bar 的 prev_active）

- `step.rs:324-334`：`raw[0..a_t_end]` 为 A_t 段（含 held 路 restore），`raw[a_t_end..]`
  为 ℬ_x 段（含 open 路 restore）。
- `step.rs:481-488`：两段分别 `element_as_leg(&work[i])` 转成 `ActiveLeg`；
  `held.rs:111-127` 中 `dir: e.eps`。
- `step.rs:497-501`：`exit::step_active_set_with_subtree_close` 对 A_t 段做 `𝒟_x^†`
  子树清仓 + 对两段做声部层 AncOK；restore 腿只要父链齐全就存活（`step.rs:454-468`
  明确「restore 注入的扩集语义保留」）。
- `step.rs:527-536`：存活腿按 id 映射回 work idx，产出 `next_idx`。
- `step.rs:660-663`：`next_idx → element_as_leg`，产出 `next_active`。
- 回测闭环：`fill.rs:6339 prev_active = next_active;`——restore 腿下 bar 确实以持仓
  身份进入 `prev_active`。

### 站 3：next_idx 全部进入 LegTarget 合成层

```rust
// step.rs:598-599
let mut legs = strategy_target_legs(&work, &next_idx, base_units, config);
```

`strategy_target_legs` 对 active 的**每个** idx 产一条 `LegTarget`
（`leg.rs:239-282`），方向即元素 eps：

```rust
// leg.rs:184-188（leg_target_two_segment；leg_target 在 leg.rs:160-164 同式）
LegTarget {
    e_idx,
    side: e.eps,           // ← restore 腿的方向就是 work 元素方向
    units: base_units * w,
    role,
}
```

### 站 4：重内单向门在净额折叠前无条件生效

```rust
// step.rs:613-622
// 重内单向（ADR 0014 裁定一，SPEC #847 S1 / 实施票 #879）…
let _uni = super::leg::enforce_chong_unidirectional(&mut legs, chong_pos);
// step.rs:634
let p_tilde = net_target_units(&legs);
```

门实现 `leg.rs:356-408`：先求 long_sum/short_sum；`chong_pos≠0` 时持仓符号为方向权威，
`chong_pos=0` 时多数侧定方向；**反向腿不跳过、不报错、不钳向，而是 `units=0`**：

```rust
// leg.rs:390-405
for leg in legs.iter_mut() {
    let same_side = matches!((direction, leg.side),
        (1, VoiceSide::Long) | (-1, VoiceSide::Short));
    match leg.side {
        VoiceSide::Flat => {}
        _ if same_side => { leg.units *= factor; stats.n_same_kept += 1; }
        _ => { leg.units = 0.0; stats.n_opposing_zeroed += 1; }
    }
}
```

`next_active` 里的反向腿**身份**保留（`step.rs:620-621` 明确这是短差减/补驱动，非持仓），
但其 `units` 已为 0。随后：

```rust
// step.rs:674-685
let sep_legs: Vec<SepLeg> = legs.iter().filter_map(|leg| {
    work.get(leg.e_idx).map(|e| SepLeg {
        id: e.id, side: leg.side,
        q_units: leg.units,          // ← 反向 restore 腿此处恒为 0.0
        role_v: leg.role.v, parent_id: e.parent_id,
    })
}).collect();
```

### 站 5：p_tilde → p* → order → fill → Chong.position_lots

- 生产组合层把 `p_t`（当前净持仓）作为 `chong_pos` 传入 coverage：
  `compose.rs:481-495`（`coverage_step_from_buckets_sep_with_risk_seeds(..., p_t, ...)`）。
- 回测主循环 `fill.rs:5594-5610` 调 `pi_theta_step_traced_with_risk_seeds(..., p_t, ...)`。
- 成交后 `fill.rs:5136-5142`：

```rust
let p_t = units;                       // 净 lot，单一有符号标量
chong_book.set_position_lots(&chong_key, p_t)…
```

- `Chong.position_lots` 是不变量承载字段，不参与计算：`chong.rs:51-56`、`chong.rs:160-167`。

零量反向声部在旁路账本也被同一过滤剔除：

```rust
// overlay_state.rs:228-232
let q = (leg.q_units / lot as f64).round() as i64 * lot;
if q <= 0 { continue; }   // 反向 restore 腿 q_units=0 ⟹ 不进 P^sep

// level_ledger.rs:61-65 同口径
if q <= 0 { continue; }
```

---

## 二、恢复腿 dir 来源：祖先持久方向 vs 候选 eps，相反时取哪个

按代码分支排序：

| 命中分支 | 方向取值 | 依据 |
|---|---|---|
| base 树 `id_idx` 命中 | **base 树当前元素 eps**（树前缀身份，非候选） | `held.rs:335-343`；注释 `held.rs:333`「base（id_idx）恒安全…不受此门禁」 |
| overlay 命中且 `idx >= overlay_cand_end` | **本 bar restore push 元素的 eps**（= registry `pe.dir`） | `held.rs:335-343` |
| overlay 命中且 `idx < overlay_cand_end`（候选拷贝） | **不复用**，落入下一分支从 registry 取 | `held.rs:328-334`、`held.rs:339` |
| registry 取元素 | **`eps: pe.dir`（持久方向）** | `held.rs:372-381` |
| 触发腿自身（`held_stale_reregister_idx` 新 push） | **`eps: leg.dir`（持仓腿方向）** | `held.rs:451-464` |
| open 候选同 id 撞 restore 腿 | 候选拷贝被跳过（`id_in_raw` 规则①），不覆盖恢复腿 | `step.rs:377-392` |

所以「候选 eps 方向可与真祖先持久方向相反」时，**持久方向赢，候选方向不进入恢复腿**。
这不是纸面注释，有直测锁定：

- `held_tests_1.rs:1125-1222 restore_does_not_reuse_candidate_segment_copy_for_ancestor`：
  候选拷贝 Short、registry 持久 Long；断言 `raw` 不含候选 idx、新 push 元素
  `eps == VoiceSide::Long`（`held_tests_1.rs:1196-1208`）。
- `held_tests_1.rs:940-1022 opened_restore_leg_not_externalized_for_same_carrier_candidate_pair`：
  restore 复用树 idx 时断言 `carrier_legs[0].dir == VoiceSide::Long`，
  「restore 腿方向取树/registry，非候选方向」（`held_tests_1.rs:1011-1021`）。
- `held_tests_1.rs:1045-1108 held_leg_id_hits_candidate_copy_keeps_held_identity`：
  候选 Short 与持仓 Long 同 id，持仓身份优先，方向不被候选翻转。

持久方向本身由 I2 首见锁保护：`persistent.rs:112-113`（`dir` 不变）、
`persistent.rs:376-393`（方向冲突拒绝覆写）；真 tree 段翻向事件另有
`dir_conflict_seen` 守卫（`persistent.rs:386-390`，restore 拉 registry 分支中断在
`held.rs:347-357`）。

---

## 三、一条具体腿的全程走查（restore → 合成 → Chong.position_lots）

复用 main 测试构型 `restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push`
（`held_tests_2.rs:1158-1279`）的数值，并把方向权威与门的行为写全：

**输入**
- `prev_active = [D, P]`，其中 D 为 LiveDetached 腿：`level=0, dir=Short, op_parent=Q`；
  P 为边界根腿：`level=2, dir=Long, is_boundary_root=true`。
- registry 有 `Q(level=1, Short, structural_parent_id=Some(P))`；Q 为 restore 祖先。
- `base_units=1000`，default `depth_weights=[0.60,0.30,0.10]`；`chong_pos=0`（测试值）。

**第 1 步 restore/物化**
- D 触发 held 路 restore：Q 从 registry 新 push，`eps=Q.dir=Short`，`raw.push(Q_idx)`；
  随后触发腿自身 D 经 `held_stale_reregister_idx` push，`eps=D.dir=Short`；
  P 由更晚 held 迭代的边界根分支直接 push，`eps=Long`。
- 统一 fixup 把 `Q.parent=P`、`D.parent=Q` 接上；AncOK 判 P/Q/D 全存活，全部进 `next_idx`。

**第 2 步合成**
- `strategy_target_legs` 产：
  - P：Long，depth 0 ⟹ `units=600`
  - Q：Short，depth 1 ⟹ `units=300`
  - D：Short，depth 2 ⟹ `units=100`

**第 3 步单向门**
- `long_sum=600`，`short_sum=400`；`chong_pos=0` ⟹ 多数侧 `direction=+1`（Long）。
- `factor=(600−400)/600=1/3`。
- P 同向：`600×1/3=200`；Q/D 反向：`units=0`。
- `p_tilde = net_target_units = +200`。
- 该数值正是 main 测试断言：`sep_q.q_units == 0.0`（`held_tests_2.rs:1262-1265`）、
  `sep_p.q_units == 200`（`held_tests_2.rs:1271-1274`）、`p_tilde == 200`
  （`held_tests_2.rs:1276-1279`）。

**第 4 步到 Chong.position_lots**
- `p_tilde=+200 → pi_theta_position → p*（lot 网格）→ schedule_order`，订单只有
  `Δ=p*−p_t` 一个净额标量（`sizing.rs:476-495`、`schedule_order` 在 `sizing.rs` 内）。
- fill 成交后 `units` 仍为单一净 lot，`ChongBook.set_position_lots` 写入该符号；
  不会出现「P 多 + Q/D 空」并存的 Chong 持仓。
- 若 `chong_pos=+3`，方向权威直接=Long，结果同上；若 `chong_pos=-3`，门对空侧
  使用 `factor=max(0,(400−600)/400)=0`，**全部归零**（钳 0 不穿零），仍不会让
  restore 腿建出反方向持仓。

这条走查说明：恢复腿**进入**了门，而不是绕过了门。

---

## 四、结论与边界

1. **旁路不成立**。生产侧 `coverage_step_from_buckets_sep_with_risk_seeds` 只有
   `compose.rs:316/397/482` 三个调用点，全部落在同一函数内、同一
   `strategy_target_legs → enforce_chong_unidirectional → net_target_units` 单源路径；
   restore 腿没有进入 `next_active` 之外的第二消费通道。
2. **门对反向恢复腿的处置是「零化 units」**（`leg.rs:401-404`），不是钳向/跳过/报错。
   零化后的 `sep_legs` 再被 `q<=0` 过滤（`overlay_state.rs:228-232`、
   `level_ledger.rs:61-65`），故反向声部不建仓、不进 LEE 镜像 ⟹ #837 D3 的
   「跨级反向持仓重叠」在现役 main 的声部账本口径下被结构性清零（`leg.rs:344-346`
   声明，执行层测试同见 `runner_tests.rs:1505`、`runner_tests.rs:1734`）。
3. **量级与可复现窗口不适用**：因为旁路不成立，不存在「restore 腿绕门放进反向持仓」
   的可复现数据窗口。若要回归验证结论，请用下方测试检索式；若要观测真实频次，需在
   main 工作区跑 restore 探针（`l3_fullwindow.rs` 已有 restored depth 读数）而非本工位。

**走查中看到、但不改变结论的边角（诚实声明）**：
`restore` 的 base `id_idx` 复用分支（`held.rs:335-343`）在命中 base 树元素时直接取当前树
eps，**不查询** `dir_conflict_seen`（该守卫只在 registry 拉取分支 `held.rs:347-357`）。
因此「tree 段真翻向 + 该 pid 是 op_parent 且仍在 base 树」的构型下，恢复腿可能取新世代
树方向而非 registry 首见持久方向。该构型可达性/频次本会话未验证；即使成立，被取到的
方向仍作为普通 `LegTarget` 走 `step.rs:622` 门，**不构成绕过单向门的通道**。

---

## 五、复现检索式

```bash
cd /Users/silencehan/Projects/NewChanlun

# 全部相关调用点
git grep -n "restore_ancestor_chain_from_registry\|enforce_chong_unidirectional" main -- rust/src/

# 关键实现
git show main:rust/src/theta_v0/strategy/coverage/held.rs | sed -n '282,391p;444,470p'
git show main:rust/src/theta_v0/strategy/coverage/step.rs | sed -n '251,293p;414,435p;481,536p;598,686p'
git show main:rust/src/theta_v0/strategy/coverage/leg.rs | sed -n '331,408p'

# 方向来源与门交互的既有直测
git grep -n "restore_does_not_reuse_candidate_segment_copy_for_ancestor\|opened_restore_leg_not_externalized_for_same_carrier_candidate_pair\|held_leg_id_hits_candidate_copy_keeps_held_identity" main -- rust/src/theta_v0/strategy/coverage/held_tests_1.rs
git grep -n "#879\|单向门" main -- rust/src/theta_v0/strategy/coverage/held_tests_2.rs
```

在 main 工作区可跑的最小回归（本会话未执行，因本工作区分支不含 S1 代码）：

```bash
cd rust
cargo test --lib restore_does_not_reuse_candidate_segment_copy_for_ancestor -- --nocapture
cargo test --lib opened_restore_leg_not_externalized_for_same_carrier_candidate_pair -- --nocapture
cargo test --lib held_leg_id_hits_candidate_copy_keeps_held_identity -- --nocapture
cargo test --lib held_leg_placeholder_parent_materializes_in_later_iteration -- --nocapture
cargo test --lib restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push -- --nocapture
cargo test --lib chong_uni_long_position_opposing_leg_becomes_reduction -- --nocapture
cargo test --lib chong_uni_never_crosses_zero -- --nocapture
```
