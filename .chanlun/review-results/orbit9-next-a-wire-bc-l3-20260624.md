# orbit9-next-a：接通 B/C，测完整 9 轨道操作语义分类 L3（核心轴 a）

**工位**：swarm/orbit9-next-a | **基因** 073a/274号 | **日期** 2026-06-24
**worktree**：`/private/tmp/orbit9-next-a-wt @ orbit9-next-a-20260624`（从整合点 `4c0afeae11` 隔离）
**承接**：#39 9轨道分类 / orbit9-A（H⁰ flip confirmed 门）/ orbit9-B（O3 add 原语 + route_bsp 9轨道分派）/ orbit9-C（区间套 H¹ 定位算子 `locate_nest`）
**编排者最初追问**：完整 9 轨道分类（B/C 死代码激活）能否解决失败模式？— **本工位首次真正测了这个问题**（此前 L3 只测 A/H⁰，B/C 占位 `fallback=true` ⇒ add 永不激活）。

---

## 〇、一句话结论

**接通 B/C 完成，add 不再是死代码（n_adds=259~3269/标的，8/8 真激活）。但完整 9 轨道分类 NOT 补偿 A-only 的 6/8 劣化——full-ON 仍劣于 OFF 6/8，仅优于 A-only 3/8（BRN/DX/OKLO）。接通后的增益是 regime 函数（震荡型 OKLO +104pp 巨幅改善 / 强牛 BTC -20.5pp、GC -20.1pp 进一步失血），不是单调改善。** 概念映射**非平凡**：`located ≠ sub_trend_done`，严格映射 = `located ∧ type1`（divergence.rs:264-268 锚定）。

---

## 一、接通点 diff（概念映射依据）

### 1.1 接通前现状（占位）

`rec_engine.rs:1430` `orbit9_sub_trend_done(_sub) -> bool { true }`（占位 fallback=true）⇒ route_bsp 同父向分支 `!orbit9_sub_trend_done(j)=false` ⇒ **O3 add 永不激活**，全走 O7 recover。这是 B/C 死代码的根源——DISPATCH 即使 ON，add 也不触发。

### 1.2 概念缺口（任务第2点：located ≠ done，非平凡，已 surface）

| | B 需要 `sub_trend_done(sub)` | C 提供 `locate_nest(sub,...).located` |
|---|---|---|
| 语义 | 次级别走势**完成**（recover 整条 vs add 部分的判别） | 操作级别 sub 处于一致极性**背驰段链**且可逐级收缩 |
| 数据源 | type1（走势终完美）| `level_diverge[sub]`（背驰段，rec_stream.rs:353-357）|
| true 含义 | 走势完成 → recover 整条 | 区间套定位成功（H¹ 位次）|

**形式化锚点（divergence.rs:264-268，L0）**：
> 背驰段 = 走势完美判据去掉几何门 G（c 创新高）分量 ⇒ **背驰段 ⊋ 走势完成（type1）**；两者唯一差 = 创新高确认门。

∴ `located=true`（= 背驰段存在）是 `sub_trend_done`（= 走势完成）的**必要非充分**条件。located 为真时走势**可能仍在背驰段中（未完成）**。**直接令 `done := located` 是语义颠倒的 workaround**（把"背驰段中"误判为"完成"，且 located 在背驰段几乎恒真 ⇒ add 仍恒不激活 = 占位 fallback 的同义反复）。**未硬塞"能跑"的映射**（遵守 no-workaround）。

### 1.3 严格映射（实装）

```rust
fn orbit9_sub_trend_done(&self, sub: usize, view: &LevelView) -> bool {
    if !self.enable_orbit9_nest { return true; }     // NEST OFF ⇒ fallback=true ⇒ recover-only bit-exact
    let mob = self.instances[sub].direction;          // 次级别回调腿极性（= 反父向短差极性）
    let loc = locate_nest(sub, mob, view, self.cur_bar);          // C 项：区间套定位（背驰段链贯通，必要）
    let type1_confirmed = view.t1buy[sub] || view.t1sell[sub];   // type1 项：创新高确认（充分补足）
    loc.located && type1_confirmed                    // 严格 done = located ∧ type1
}
```

- `done=true`（located ∧ type1）⇒ 走势完成 ⇒ **O7 recover**（整条升回）。
- `done=false`（located ∧ ¬type1：背驰段中未创新高 / 或 ¬located）⇒ 回调未完成 ⇒ **O3 add**（部分买回 m=quota，不动 h，骑背驰段等下一 type1 定位点）。

### 1.4 改动清单

| 文件 | 改动 | 行数 |
|---|---|---|
| `rec_engine.rs` | `orbit9_sub_trend_done(sub, view)` body 替换占位为 `locate_nest ∧ type1` 严格映射；`route_bsp` 签名 +`view: &LevelView`；`use locate_nest`；`TRoot` +`enable_orbit9_nest` 字段（B/C 只在 EngineConfig 有，无消费者）+ 构造；on_bar 2 调用点传 view；3 新 wired 测试 + 1 dispatch_nest 测试 + off_bit_exact 更新 | +134 |
| `rec_stream.rs` | `rec_btc` eprintln 加 `n_adds`/`n_h0_flip_blocked`（L3 验收 add 激活）| +4 |

---

## 二、OFF bit-exact 复核（L1，PASS）

**单测**：`cargo test recursive_t` = **140 passed / 0 failed**（137 基线 + 3 新 wired + 1 dispatch_nest，旧 on_orbit9 占位测试替换为真实接通语义）。`rec_flat_bit_exact_含走势完成清仓路径` PASS（rec≡flat OFF 路径）。

**真实数据全管线 bit-exact（强证明，对照整合基线 `4c0afeae11`）**：

| | 我的 OFF（next-a）| integration 基线（4c0afeae11，父 commit）|
|---|---|---|
| CL Structural strat | **+20.3%** | **+20.3%** |
| enter/sink/recover/drain/flip/ascend | 12/2707/1369/1086/5/11 | 12/2707/1369/1086/5/11 |
| short_pnl | -13050 | -13050 |

**逐字一致** ⇒ route_bsp 加 `view` 参数 + `enable_orbit9_nest` 字段**不扰动 OFF 路径**。全 8 标的 OFF `add=0 ∧ h0blk=0`（orbit9 三分支均未执行，必要条件）。三 env 缺省 = facea 逐位。

---

## 三、full-ON / A-only-ON / OFF 三方 L3 对照（Structural，net-up 8 标的）

**配置**：OFF = 三 env 全缺省；A-only = `T_ORBIT9_H0=1`（仅 H⁰ flip confirmed 门）；full-ON = `T_ORBIT9_H0=1 T_ORBIT9_DISPATCH=1 T_ORBIT9_NEST=1`（H⁰ + 9轨道分派 + 区间套消费全开）。数据 = `analysis/data_cache/*_1m_*.json`（10y/full）。

| 标的 | OFF | A-only | full-ON | **add** | h0blk | full vs A | full vs OFF |
|---|---:|---:|---:|---:|---:|---:|---:|
| CL | +20.3% | -28.1% | -37.2% | 761 | 64 | -9.1 | -57.5 |
| BRN | -26.0% | -16.4% | **+2.8%** | 1601 | 16 | **+19.2** | **+28.8** |
| DX | -0.7% | -4.0% | -2.5% | 1010 | 10 | +1.5 | -1.8 |
| GC | -22.7% | -11.5% | -31.6% | 3269 | 3 | -20.1 | -8.9 |
| ES | -2.3% | -2.9% | -7.6% | 1740 | 61 | -4.7 | -5.3 |
| QQQ | -8.6% | -21.1% | -29.5% | 598 | 17 | -8.4 | -20.9 |
| BTC | +26.0% | -28.3% | -48.8% | 3146 | 29 | -20.5 | -74.8 |
| OKLO | +55.3% | -29.6% | **+74.4%** | 259 | 0 | **+104.0** | **+19.1** |

**计数**：
- A-only 劣于 OFF：**6/8**（除 OKLO/DX? 实为 CL/BRN？— A-only 仅 BRN/GC 相对 OFF 改善方向，但 6/8 标的 A-only < OFF）。
- full-ON 劣于 OFF：**6/8**（仅 BRN/OKLO 优于 OFF）。
- full-ON 优于 A-only（**接通 B/C 是否补偿**）：**仅 3/8**（BRN/DX/OKLO）；恶化 5/8（CL/GC/ES/QQQ/BTC）。
- **n_adds>0 验收：8/8 全部 add 真激活**（259~3269）⇒ B/C 不再是死代码。

---

## 四、机制分析（为什么接通未补偿）

1. **add 全面替代 recover**：full-ON CL `recover=0`（706 sink 的升回全部变成 add）。严格映射 `done=located∧type1` 在真实数据中 type1 稀疏 ⇒ 绝大多数同父向 BSP 走 add（部分买回）而非 recover（整条升回）。这根本改变资本动力学。
2. **强牛 add 抽干核心**：BTC/GC（强单边牛）add=3146/3269，full-ON -48.8%/-31.6% ≪ OFF。部分买回在牛市中反复用机动配额 m=quota 重建短差腿，核心 units 被 sink 减后只部分恢复 ⇒ 踏空主升浪（与 project_btc_bull_bear_attribution「核心永久锁定踏空」同构）。
3. **震荡 add 有益**：OKLO（+104pp vs A-only）/BRN（+19.2pp）。震荡 regime 中部分买回/卖回吃回调差价，不踏空（无主升浪可踏）⇒ add 短差腿净赚。
4. **这是项目反复出现的 regime×模式签名**（见 memory: project_t_emergence_upgrade_ab「无单一支配=regime×模式」/ project_nrf_v4_strict_accounting「regime 第 N 例」）。完整 9 轨道分类**不是**失败模式的解，是**又一个 regime 函数**。

---

## 五、结果包六要素

### 1. 结论
接通 B（O3 add 分派）/C（区间套 H¹ 定位）完成，`orbit9_sub_trend_done` 由占位 `true` 替换为严格映射 `locate_nest(sub).located ∧ (t1buy[sub]∨t1sell[sub])`。add 8/8 真激活（n_adds=259~3269）。完整 9 轨道分类 L3：full-ON 劣于 OFF 6/8、优于 A-only 仅 3/8 ⇒ **未补偿 A-only 劣化**，增益是 regime 函数（震荡改善/强牛失血）。

### 2. 定义依据
- **divergence.rs:264-268（L0）**：背驰段 = 走势完美去几何门 G ⇒ 背驰段 ⊋ type1 ⇒ `located`（背驰段）是 `done`（走势完成=type1）的必要非充分条件 ⇒ 严格映射须 `located ∧ type1`。
- **C 报告 §3 边界（行 28-29）**：`located` ≠ 走势完成（区间套定位到买卖点 ≠ 完成）。
- **B 报告 §2.1**：O7 recover（走势完成→整条）vs O3 add（回调未完成→部分），判别 = `is_sub_trend_done` 接口。
- 输入数据满足条件：`level_diverge[sub]=Some(mob)`（背驰段链贯通，C 项）；`t1*[sub]`（type1 创新高确认，走势完成项）。

### 3. 边界条件（结论翻转）
- 若 type1 信号在某 regime 致密（背驰段几乎都伴随创新高）⇒ done 频繁为真 ⇒ add 退化为 recover ⇒ full-ON → OFF（add 不激活）。**反例**：CL/BTC type1 稀疏（add≫recover）⇒ 当前不翻转。
- 若 add 配额 m=quota(1/3 σ-不变) 改大 ⇒ 部分买回逼近整条 ⇒ add → recover 语义坍缩。当前 quota=1/3 ⇒ add ≠ recover，结论成立。
- 若 net-up 数据换 bear 窗（真下跌段）⇒ 强牛踏空机制消失 ⇒ add 可能在 BTC/GC 转正（regime 翻转）。**本工位仅测 net-up 全程**，bear 窗委托 next-b（task#58）。
- **若严格映射改回 `done:=located`（去掉 type1 项）** ⇒ located 背驰段恒真 ⇒ add 恒不激活 ⇒ 退回占位 fallback（语义颠倒）。type1 项不可去。

### 4. 下游推论
- 9 轨道完整分类**不是失败模式的架构解**——失败模式（A-only 6/8 劣化）的根因不在"分类不完整"（add 缺口），补 add 后仍 6/8 劣于 OFF。**编排者最初追问得到否定回答**：完整分类 ≠ 解。
- add 的有效域 ⊂ 震荡 regime（OKLO/BRN/DX）。若要用，须 per-asset regime 白名单门控（与项目所有 regime 函数同构），不能全局开。
- recover→add 全替代（type1 稀疏）暴露：O7/O3 的判别阈值（type1 vs 背驰段）对资本动力学是支配性的——这本身是一个可结晶的概念点（"完整分类的有效性受 type1 密度调制"）。

### 5. 谱系引用
- **#34 NestEntry 范畴错误**（概念分离领域）：本工位**避免重犯**——`located` 不直接当 `done`（区间套不是方向/完成滤波器），严格补 type1 项。located≠done 是 NestEntry「区间套当方向门」的同类张力在「区间套当完成门」上的再现。
- **#39 9 轨道分类 / 543 add=word**：本工位是 #39 的 L3 验收——分类完整性（O3 补缺）的经验有效域 ≪ 定义域（代数完整 ≠ 收益改善）。
- **587 候选A（τ 对称）/ 539（做空腿失血）**：full-ON 强牛失血与 539「下跌段做空 / 牛市踏空」同根。
- **regime 函数谱系**（memory 多例）：project_t_emergence_upgrade_ab / project_nrf_v4_strict_accounting / project_osc_whitelist_elimination——本工位是「完整 9 轨道分类 = regime 函数」的新一例。

### 6. 影响声明
- **改动**：`rec_engine.rs`（orbit9_sub_trend_done 严格映射 / route_bsp +view 参数 / TRoot +enable_orbit9_nest 字段 / 4 测试）；`rec_stream.rs`（rec_btc +n_adds/h0blk 观测）。
- **影响模块**：route_bsp 签名（rec_engine 内部 + 2 on_bar 调用点 + 2 测试调用点，全部更新）；orbit9_sub_trend_done 消费 view + locate_nest。
- **bit-exact 契约**：三 env 缺省 ⇒ enable_orbit9_nest=false ⇒ done fallback=true ⇒ recover-only ⇒ 对照 4c0afeae11 逐字（CL Structural +20.3% 全字段一致）。
- **未改动**：八轨道原语行为（OFF）、信号层、收益口径。add 原语（B）/locate_nest（C）逻辑不动，仅接通消费路径。

---

## 六、认识论等级标注（formalization-validity-domain 强制）

| 命题 | 等级 | 信息增量 |
|---|---|---|
| located ≠ done（背驰段 ⊋ type1）| **L0**（divergence.rs:264-268 形式化）| 高：概念缺口形式化 |
| 严格映射 done=located∧type1 | **L0**（定义推导）| 高 |
| OFF bit-exact（对照 4c0afeae11 全字段）| **L1**（真实数据全管线，CL 逐字）| 零（管线，验证 OFF 不变）|
| 3 wired 单测（located∧¬type1→add 等）| **L1**（合成 LevelView 注入）| 零（管线验证，不验 alpha）|
| **完整分类 L3：full-ON vs A-only vs OFF（8 标的）** | **L3**（多标的真实数据，否定性结果）| **高：否证「完整分类=解」假设，划定 add 有效域 ⊂ 震荡 regime** |

**否定性结果价值**：full-ON 优于 A-only 仅 3/8、优于 OFF 仅 2/8 = **否证了「9 轨道分类不完整是失败模式根因」假设**。这缩小了有效域边界（add ⊂ 震荡 regime），比确认性结果更有价值。

---

## 七、约束 4 异质审计降级声明

约束4（codex-challenger）：codex 死（593 venv editable 损坏）⇒ 本产出标 **pending-real-heterogeneous**。概念映射（located≠done，divergence.rs:264-268）= L0 不依赖异质复验；OFF bit-exact = L1 自验（对照 4c0afeae11）；L3 三方对照 = 真实数据否证（可复跑：`T_ORBIT9_H0=1 T_ORBIT9_DISPATCH=1 T_ORBIT9_NEST=1 cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`）。
</content>
</invoke>
