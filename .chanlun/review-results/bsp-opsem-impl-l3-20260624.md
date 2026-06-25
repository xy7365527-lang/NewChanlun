# 节点向量 route_bsp 实装 L3：踏空溶解 = regime 函数（核心保护禁次级别反核心向 sink）

**工位**：swarm/bsp-opsem-impl ｜ topo_address: swarm/bsp-opsem-impl ｜ 基因 073a/274号 ｜ 任务 #63
**日期**：2026-06-24 ｜ 分支：orbit9-B-dispatch-20260624（基座 031747898b = task#40 B route_bsp 9轨道分派 + O3 add 原语）
**认识论等级**：L0（节点向量路由架构）+ L1（OFF bit-exact）+ **L3（8 标的真实数据，否定性结果：强牛溶解/震荡爆仓 = regime 分裂非趋同）**

---

## 〇、一句话结论

**节点向量路由的核心保护实装成功（read r* 级别角色 ⊗ 力度轴 d_top）：在强牛标的踏空被溶解（BTC +26%→+1537% 超 BH+1380%、ES +828% 超 BH+594%，net_short 62.3%→0.5% = 核心不再被次级别回调腿砍仓做空主升浪）。但 8 标的 NOT 趋同——含下跌段标的（CL/BRN/OKLO/QQQ −87~−105%）核心裸仓无对冲爆仓。这正是 #61 §七预言：形态层踏空消除 ≠ L3 净收益普适，有效域 = regime 函数（561 b2/563 未决）。** OFF bit-exact PASS（CL +20.3% sink=2707 short_pnl=−13050 逐字）。

---

## 一、节点向量路由 diff（坍缩点 → 解坍缩）

### 1.1 坍缩点（修复前，#61 §五/§七「拍扁纤维」）

`route_bsp(j, is_buy, node, c)` 分派只读 **2 个标量**：`nearest_active_parent(j)`（最近祖先）+ `highest_active()`（核心级）。节点 j 在**所有参与级别**的角色 R+/R−（区间套给出的级别角色向量）被压扁——次级别 type1/type3（R− 回调腿）走 `Some(p)` 的 `is_reduce` 分支 → sink，**sink 减核心机动仓**（OFF anchor=0 ⇒ mob_base=u_p）。反复 sink（BTC 2744 次）累积抽干核心 = 主升浪回调处砍核心 = **踏空根因①④**（#61 §六 N1@k,R−/N3@k,R−）。

### 1.2 解坍缩（节点向量 = 拓扑级别角色 ⊗ 力度轴）

新增 `enable_orbit9_nodevec`（env `T_ORBIT9_NODEVEC`，缺省 OFF）。route_bsp +`view` 参数（传力度轴）。`is_rstar_pullback_leg(j, is_buy, view)` 双源判别（#61 §3.0）：

```
R− 回调腿（保护核心）⟺ topo_pullback ∧ ¬rstar_trend_done
  topo_pullback   = j<r* ∧ 反核心向（买点 vs 核心Short / 卖点 vs 核心Long）  [拓扑轴]
  rstar_trend_done = view.d_top[r*]（r* 区间套链贯通真顶/真底=走势完成）       [力度轴 F]
```

R− 回调腿 ⇒ `sink_impl(protect_core=true)` ⇒ `mob_base=0` ⇒ m=0 ⇒ **核心 units 不减**（机动不动核心，§六 O6）。R+ 延续腿 / r* 走势已完成（真转折）⇒ 正常 sink（bit-exact 透传）。

**改动文件**：`rec_engine.rs`（+`enable_orbit9_nodevec` 字段 EngineConfig/TRoot/from_env/off；`is_rstar_pullback_leg` 双源判别；`sink_impl(protect_core)` + `sink_guarded` 包装；route_bsp +view 签名；2 on_bar + 2 测试调用点）。

## 二、OFF bit-exact（L1，PASS）

`cargo test --release recursive_t` = **126 passed / 0 failed**（含 `rec_flat_bit_exact_含走势完成清仓路径`、`off_bit_exact_orbit9未激活`）。真实数据：NODEVEC OFF（缺省）CL Structural = **+20.3% / sink=2707 / recover=1369 / short_pnl=−13050** = 与基座逐字一致 ⇒ 新字段+view 参数不扰动 OFF 路径。

## 三、8 标的修复前后对照（L3，Structural，net-up 全程）

| 标的 | OFF（修复前） | NODEVEC ON（节点向量核心保护） | BH | sink OFF→ON | net_short OFF→ON | 踏空判定 |
|---|---:|---:|---:|---:|---:|---|
| **BTC** | +26.0% | **+1536.7%** | +1380.4% | 2744→0 | 62.3%→0.5% | **溶解（超 BH）** |
| **ES** | −2.3%* | **+828.5%** | +594.3% | →0 | →5.9% | **溶解（超 BH）** |
| GC | −22.7% | −27.9% | +257.3% | 2908→0 | 63.9%→86.0% | 未溶解（核心裸仓） |
| DX | −0.7%* | −16.9% | +4.1% | →0 | →78.7% | 劣化 |
| CL | +20.3% | −104.9% | +28.2% | 2707→0 | 37.9%→26.2% | **爆仓（裸多打穿）** |
| BRN | −26.0%* | −100.0% | +87.4% | →0 | →18.9% | 爆仓 |
| QQQ | −8.6%* | −86.8% | +174.6% | →0 | →75.3% | 爆仓 |
| OKLO | +55.3%* | −100.4% | +307.1% | →0 | →14.1% | 爆仓 |

\* OFF 值取自 next-a 报告（同基座口径）；BTC/GC/CL OFF 本工位实测复现（BTC+26.0/GC−22.7/CL+20.3 逐字一致）。

**计数**：踏空溶解超 BH **2/8**（BTC/ES，强牛）；爆仓/深亏 **6/8**（含下跌段标的核心裸仓）。**NOT 趋同**——是 regime 两极分裂。

## 四、机制分析：为什么 sink=0 全标的 + 为什么不趋同

1. **sink=0 全标的（含力度轴叠加后仍不变）**：`view.d_top[r*]` 在 net-up 全程**几乎恒 false**——r* 是最高涌现级别，其走势恒生长中、永不"完成"（结构必然，见 `project_l4_consoldn_no_leave`「最高涌现级别恒不产 type1」+ `project_highest_level_sigma_frozen`「最高级别 σ 冻结」）。⇒ **力度轴 F 在 r* 退化为常数 false** ⇒ 双源乘积 `topo ∧ ¬done` 退化回单源 `topo` ⇒ 所有次级别反核心向 sink 全禁。**这经验印证 #61 §3.0：拓扑不能导出力度，但当力度轴在 r* 退化时双源坍回单源**。
2. **强牛溶解**：BTC/ES 核心永远在主升浪，禁次级别反核心向 sink ⇒ 核心不被回调腿砍 ⇒ 骑住全程 ⇒ 超 BH。这直接命中 `project_btc_bull_bear_attribution`「核心永久锁定踏空」的反面——核心不再被抽干。
3. **含下跌段爆仓**：CL/BRN/OKLO 有真实下跌段，下跌段中核心多头**需要** sink 短差腿对冲下行，但节点向量路由把下跌段的次级别反弹（拓扑上也"反核心向"）误判为回调腿禁了 sink ⇒ 核心裸多被下跌段打穿（CL −104.9%、short_pnl 缺失对冲）。
4. **根因 = r\* 力度轴退化 ⇒ 无法区分「主升浪回调（该保护）」vs「下跌段反弹（该对冲）」**。二者拓扑相同（次级别反核心向），唯一区别是 r* 是否真在上涨——而 d_top[r*] 恒 false 无法提供这个信息。**这是 #61 §七「形态相同、纤维角色（损益功能）相反」的精确经验实例**：BTC 回调腿与 CL 下跌段反弹形态全同，损益功能相反，节点分类给形态完备但有效域是 regime 函数。

## 五、结果包六要素

### 1. 结论
节点向量路由（read r* 级别角色 ⊗ 力度轴 d_top，对 R− 回调腿 sink 强制核心保护 mob_base=0）实装成功 + OFF bit-exact。**踏空在强牛标的被溶解**（BTC+1537%/ES+828% 超 BH，net_short→0.5%）；**8 标的 NOT 趋同**——含下跌段标的核心裸仓爆仓（6/8 深亏）。

### 2. 定义依据
- **#61 §3.0 双源乘积**：节点身份 = 拓扑级别角色 ⊗ 力度公理 F。本实装 `is_rstar_pullback_leg = topo_pullback ∧ ¬d_top[r*]`。
- **#61 §六 N1@k,R−/N3@k,R− → O6 sink 机动不动核心**（编排者亲点根因④）：R− 回调腿不砍核心。
- **#61 §3.4 断言U**：j 对 r* 两角色 R+/R−（顺/逆核心主向），穷尽无第三态。
- 输入满足：`highest_active()=r*`（核心级）；`instances[r*].direction`（核心主向）；`view.d_top[r*]`（r* 走势完成力度判据）。

### 3. 边界条件（结论翻转）
- **若 d_top[r*] 在某 regime 致密**（r* 走势频繁完成）⇒ 力度轴不退化 ⇒ 双源生效 ⇒ 能区分回调腿 vs 真转折 ⇒ 可能恢复部分 sink 救下跌段标的。**当前 net-up 全程 d_top[r*] 恒 false ⇒ 不翻转**（结构必然）。
- 若 `protect_core` 改为「机动 1/3 仍 sink 仅保护 2/3 核心」（而非 mob_base=0 全禁）⇒ 下跌段标的可能保留对冲。**但 OFF 的 quota 本就是 1/3，反复 sink 累积仍抽干核心 ⇒ 部分保护不解踏空**（这是初版语义选择：踏空溶解需全禁次级别反核心向 sink）。
- 若换 bear 窗（真下跌全程）⇒ 强牛踏空机制消失 ⇒ BTC/ES 可能失去优势（委托 bear 窗工位）。

### 4. 下游推论
- **踏空（纤维拍扁病理）在强牛被溶解 = 节点向量路由方向正确**（核心保护是对的存在论形式）。但**净收益不普适 = 有效域 regime 函数**（继承 561 b2/563，不声明膨胀）。
- **r\* 力度轴退化是节点向量路由的真实瓶颈**：d_top[r*] 恒 false ⇒ 双源坍回单源 ⇒ 无法区分主升浪回调 vs 下跌段反弹。下一轴 = 找 r* 走势"是否真在上涨"的**非 d_top 力度代理**（d_top[r*] 在最高级别恒失效）。
- 节点向量路由须 per-regime 白名单门控（与项目所有 regime 函数同构），不能全局开。

### 5. 谱系引用
- **#61（双源乘积完全分类）**：本工位是其节点侧实装。**经验印证 §3.0**：拓扑⊗力度，但**力度轴在 r* 退化**时双源坍回单源（新增有效域边界发现）。
- **561 号（Ω 坍缩 = 539 根因）**：本工位是 561 节点侧解坍缩的实装；net 收益有效域 L3 未决（b2/563）继承。
- **next-a（接通 B/C add）**：本工位是其更深一层——next-a 仍按单级别 type 分派（只接 add），本工位读 r* 级别角色保护核心。
- **project_btc_bull_bear_attribution**（核心永久锁定踏空）：BTC 溶解 = 该踏空机制的反面（核心不再被抽干）。
- **project_l4_consoldn_no_leave / project_highest_level_sigma_frozen**：r* 力度轴退化（d_top[r*] 恒 false）的根因 = 最高涌现级别恒生长不产 type1。
- **#61 §七（形态量 vs 功能量，next-c）**：BTC 主升浪回调与 CL 下跌段反弹**形态全同损益相反** = 本工位的精确经验实例。

### 6. 影响声明
- **改动**：`rec_engine.rs`（`enable_orbit9_nodevec` 字段 ×3 接入点；`is_rstar_pullback_leg` 双源判别新方法；`sink_impl(protect_core)`+`sink_guarded`；route_bsp +view 签名；4 调用点更新）。
- **影响模块**：route_bsp 路径（sink 调用）；OFF bit-exact 契约（NODEVEC OFF ⇒ protect_core 恒 false ⇒ sink 原行为逐字）。
- **未改动**：八轨道原语 OFF 行为、add/recover 逻辑、信号层、收益口径。
- **重诊断**：把「节点向量路由能否溶解踏空 + 趋同」重诊断为「踏空在强牛溶解（形态层成立）但不趋同（净收益 = regime 函数）；瓶颈 = r* 力度轴退化使双源坍回单源」。

## 六、认识论等级标注

| 命题 | 等级 | 信息增量 |
|---|---|---|
| 节点向量路由架构（read r* 角色 ⊗ 力度轴 → 核心保护） | **L0**（#61 §3.0/§3.4/§六 实装） | 高：解坍缩节点侧 |
| OFF bit-exact（CL +20.3% 逐字 + 126 测试） | **L1**（真实数据全管线） | 零（验证 OFF 不变） |
| **踏空强牛溶解 / 8 标的不趋同（regime 分裂）** | **L3**（8 标的真实数据，否定性） | **高：踏空溶解=形态成立 ∧ 趋同假设否证（有效域 regime 函数）** |
| r* 力度轴退化 ⇒ 双源坍回单源 ⇒ sink=0 全标的 | **L2→L3**（真实数据机制） | **高：揭示节点向量路由真实瓶颈（d_top[r*] 恒 false）** |

**核心诚实声明**：① **只声明踏空在强牛被溶解（BTC/ES 超 BH，net_short→0），不声明净收益普适**（6/8 爆仓 = regime 函数，561 b2/563 未决）。② **8 标的 NOT 趋同**——纤维拍扁病理在强牛被消除但在含下跌段标的产生新病理（核心裸仓）。③ **瓶颈诚实暴露**：r* 力度轴退化（d_top[r*] 恒 false，结构必然）使双源乘积坍回单源——这是 #61 §3.0「拓扑⊗力度」在最高级别的有效域边界（力度轴失效）。**否定性结果的价值**：否证了「读全节点向量 ⟹ 8 标的趋同」，缩小有效域边界（踏空溶解 ⊂ 强牛 regime ∧ 须 r* 非退化力度代理）。

## 七、约束 4 异质审计降级

codex 死（venv editable 损坏，593号）⇒ 标 **pending-real-heterogeneous**。L0 架构（#61 §3.0 双源）不依赖异质复验；OFF bit-exact = L1 自验（CL 逐字 + 126 测试）；L3 = 真实数据否证（可复跑 `T_ORBIT9_NODEVEC=1 cargo test --release recursive_t::rec_stream::tests::rec_btc -- --ignored --nocapture`）。
