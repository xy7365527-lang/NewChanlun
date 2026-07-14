# P43 正式隔离重放 v2：D_parent = c_interval_full（2026-07-12）

- 最终判定：**PASS**
- 权威：`dparent-leftend-p0-review-20260711.md` §7；能力基线：`p45-cp-capability-20260712.md`。
- 输入：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`，4613599 bar（2017-08-17 04:00:00 .. 2026-05-31 23:59:00），40001 trades；`ThetaConfig::default()`（l_max=6 / min_parts_per_level=3）；全量因果重放 1545.8s。
- 唯一语义变量：`D_parent: c_episode_interval -> d_parent_interval_full(c_interval_full)`；`D_child := child.a_interval` 保持 **provisional / pending separate ruling**。方向、`cand_delta`、右端证明规则、`confirm_src/lag_conf/epsilon_conf` 均未改，后三者仅诊断。
- 写入边界：本次只落盘本报告；未回写 2026-07-10 漏斗、冻结或裁决文档，未修改 `departure_move_c_start`。

## 1. 重放与旧基线门
- sanity / P1 / E1 / terminal / 装配镜像：true / true / true / true / true。
- 旧基线复核：L1 partial chain=3（期望 3），L2 cand_delta=true=13（期望 13），L1→L2 同向可达候选对=22（期望 22）→ **无漂移，允许横比**。

## 2. 每个父事件的完整 c / episode 三元组与结构证据
`关系` 比较 `c_start_full` 与 `c_episode_start`。`c_end_full=None` 的事件被正式装配拒绝并计数；绝不回填 episode。

| L | side / confirm | B_p（ID / source） | c_start_full | c_episode_start | c_end_full | 关系 | c_p 首尾递归 ID | 第三类（leave / retest / source） | A_left / 未扩张检查 |
|---:|---|---|---:|---:|---|:---:|---|---|---|
| 1 | `Short / 345518` | `L2#148 / (340499, 344222)` | 344233 | 344830 | None（拒绝） | < | `L1#704 .. None` | `None` | 340499 / true |
| 1 | `Long / 759272` | `L2#360 / (757550, 758668)` | 758713 | 758713 | None（拒绝） | == | `L1#1625 .. None` | `None` | 757550 / true |
| 1 | `Long / 1044073` | `L2#495 / (1042134, 1043782)` | 1043884 | 1043884 | None（拒绝） | == | `L1#2194 .. None` | `None` | 1042134 / true |
| 1 | `Short / 2168349` | `L2#988 / (2160411, 2165116)` | 2165236 | 2167920 | None（拒绝） | < | `L1#4469 .. None` | `None` | 2160411 / true |
| 1 | `Long / 2365194` | `L2#1066 / (2361351, 2364547)` | 2364887 | 2364887 | None（拒绝） | == | `L1#4843 .. None` | `None` | 2360807 / true |
| 1 | `Short / 3306324` | `L2#1508 / (3303342, 3305431)` | 3305536 | 3305536 | None（拒绝） | == | `L1#6703 .. None` | `None` | 3303342 / true |
| 1 | `Short / 4264505` | `L2#1931 / (4261326, 4264053)` | 4264209 | 4264209 | None（拒绝） | == | `L1#8591 .. None` | `None` | 4260602 / true |
| 2 | `Short / 132770` | `L3#8 / (109807, 122447)` | 122472 | 130865 | None（拒绝） | < | `L2#50 .. None` | `None` | 105684 / true |
| 2 | `Short / 174527` | `L3#12 / (160931, 172156)` | 172192 | 172192 | None（拒绝） | == | `L2#69 .. None` | `None` | 160931 / true |
| 2 | `Short / 354036` | `L3#31 / (340499, 350827)` | 352003 | 352003 | None（拒绝） | == | `L2#151 .. None` | `None` | 340499 / true |
| 2 | `Long / 510836` | `L3#47 / (501432, 508383)` | 509564 | 509564 | None（拒绝） | == | `L2#229 .. None` | `None` | 498611 / true |
| 2 | `Long / 658767` | `L3#67 / (650224, 656902)` | 657601 | 657601 | None（拒绝） | == | `L2#309 .. None` | `None` | 650224 / true |
| 2 | `Short / 1761310` | `L3#172 / (1746389, 1758355)` | 1758355 | 1760226 | None（拒绝） | < | `L2#834 .. None` | `None` | 1746389 / true |
| 2 | `Short / 1843204` | `L3#178 / (1823829, 1835970)` | 1835988 | 1840266 | None（拒绝） | < | `L2#860 .. None` | `None` | 1823829 / true |
| 2 | `Long / 1967685` | `L3#191 / (1957188, 1965971)` | 1966116 | 1966116 | None（拒绝） | == | `L2#915 .. None` | `None` | 1957188 / true |
| 2 | `Short / 2182560` | `L3#206 / (2167920, 2178474)` | 2180264 | 2180264 | None（拒绝） | == | `L2#993 .. None` | `None` | 2160411 / true |
| 2 | `Short / 2197213` | `L3#207 / (2180264, 2187028)` | 2187038 | 2194856 | None（拒绝） | < | `L2#996 .. None` | `None` | 2180264 / true |
| 2 | `Long / 2636582` | `L3#248 / (2624161, 2634521)` | 2635395 | 2635395 | None（拒绝） | == | `L2#1183 .. None` | `None` | 2624161 / true |
| 2 | `Short / 3427468` | `L3#330 / (3404968, 3425208)` | 3425747 | 3425747 | None（拒绝） | == | `L2#1563 .. None` | `None` | 3398264 / true |
| 2 | `Short / 3436636` | `L3#331 / (3427576, 3434763)` | 3435436 | 3435436 | None（拒绝） | == | `L2#1568 .. None` | `None` | 3425747 / true |
| 3 | `Short / 206731` | `L4#1 / (160931, 187631)` | 190956 | 202739 | None（拒绝） | < | `L3#15 .. None` | `None` | 81584 / true |
| 3 | `Long / 693716` | `L4#12 / (657601, 679412)` | 681658 | 681658 | None（拒绝） | == | `L3#71 .. None` | `None` | 650224 / true |
| 3 | `Short / 960644` | `L4#18 / (908805, 951156)` | 954139 | 954139 | None（拒绝） | == | `L3#99 .. None` | `None` | 908805 / true |
| 3 | `Short / 1036108` | `L4#19 / (964857, 1009736)` | 1015889 | 1028321 | None（拒绝） | < | `L3#104 .. None` | `None` | 954139 / true |
| 3 | `Short / 1835970` | `L4#31 / (1770084, 1815305)` | 1823829 | 1823829 | None（拒绝） | == | `L3#178 .. None` | `None` | 1647951 / true |
| 3 | `Short / 1870550` | `L4#32 / (1823829, 1860928)` | 1866122 | 1866122 | None（拒绝） | == | `L3#181 .. None` | `None` | 1823829 / true |
| 3 | `Long / 2329634` | `L4#37 / (2253253, 2299566)` | 2301262 | 2321414 | None（拒绝） | < | `L3#214 .. None` | `None` | 2253253 / true |
| 3 | `Long / 2534466` | `L4#41 / (2477437, 2520991)` | 2527548 | 2527548 | None（拒绝） | == | `L3#238 .. None` | `None` | 2477437 / true |
| 3 | `Long / 4001207` | `L4#70 / (3949783, 3994672)` | 3996422 | 3996422 | None（拒绝） | == | `L3#385 .. None` | `None` | 3949783 / true |
| 4 | `Long / 2826563` | `L5#6 / (2527548, 2737472)` | 2741411 | 2741411 | None（拒绝） | == | `L4#46 .. None` | `None` | 2477437 / true |
| 4 | `Short / 4071883` | `L5#10 / (3798471, 3925661)` | 3949783 | 4054487 | None（拒绝） | < | `L4#70 .. None` | `None` | 3798471 / true |

分布 `== / < / > / c_start 缺失` = **21 / 10 / 0 / 0**。若 `>` 非零，其表中 `B_p -> departure_move_id` 说明被排除前缀归属于 B 而非完整 c；本次不以 episode 值替代。所有可表达左端均来自扫描侧车的 non-extension 离开单元；`mapping_ok=true`，证明未读取父 A 起点或整趋势起点。
父事件总数 = **31**；完整 `c_interval_full` = **0**；未闭合并被拒绝 = **31**。

## 3. §7.B 新旧漏斗
| L | Cand 候选旧→新 | 相邻边旧→新（差） | 可达 Cand 旧→新（差） | 完整父事件 / 未闭合拒绝 | 完整链证书旧→新（差） |
|---:|---:|---:|---:|---:|---:|
| 0 | 452→452 | 0→0 (+0) | 452→452 (+0) | 0 / 0（pair拒绝=0） | 452→452 (+0) |
| 1 | 7→7 | 3→0 (-3) | 3→0 (-3) | 0 / 7（pair拒绝=1589） | 3→0 (-3) |
| 2 | 13→13 | 0→0 (+0) | 0→0 (+0) | 0 / 13（pair拒绝=0） | 0→0 (+0) |
| 3 | 9→9 | 0→0 (+0) | 0→0 (+0) | 0 / 9（pair拒绝=0） | 0→0 (+0) |
| 4 | 2→2 | 0→0 (+0) | 0→0 (+0) | 0 / 2（pair拒绝=0） | 0→0 (+0) |
| 5 | 0→0 | 0→0 (+0) | 0→0 (+0) | 0 / 0（pair拒绝=0） | 0→0 (+0) |

确认延迟分布仍只诊断：
- L0→L1 considered `n=0`；accepted `n=0`。
- L1→L2 considered `n=0`；accepted `n=0`。
- L2→L3 considered `n=0`；accepted `n=0`。
- L3→L4 considered `n=0`；accepted `n=0`。
- L4→L5 considered `n=0`；accepted `n=0`。

### 所有新增 L1→L2 边
无新增边。

## 4. §7.C 三个预注册近失样本（按身份重定位）
| # | 旧父窗 | child.a_interval（provisional） | 新 c_start_full / 闭合状态 | child.left - c_start_full | Sub 左 / 右 | 最终结果与 c_p 分解 |
|---:|---|---|---|---:|---|---|
| 1 | `(352003, 354036)` | `(340499, 341236)` | 352003（右端未闭合，完整父证书拒绝） | -11504 | 未进入（完整证书拒绝） / 未进入（完整证书拒绝） | **不通过**；B=Some(ElementId { level: 3, ordinal: 31 }); c=L2#151..None; third=None |
| 2 | `(2180264, 2182560)` | `(2160411, 2161133)` | 2180264（右端未闭合，完整父证书拒绝） | -19853 | 未进入（完整证书拒绝） / 未进入（完整证书拒绝） | **不通过**；B=Some(ElementId { level: 3, ordinal: 206 }); c=L2#993..None; third=None |
| 3 | `(2194856, 2197213)` | `(2160411, 2161133)` | 2187038（右端未闭合，完整父证书拒绝） | -26627 | 未进入（完整证书拒绝） / 未进入（完整证书拒绝） | **不通过**；B=Some(ElementId { level: 3, ordinal: 207 }); c=L2#996..None; third=None |

`P0-43-L1L2-CFULL`：**SUPPORTED-ON-THIS-REPLAY**。三例在本次同数据同参数重放均未通过；不外推到其他数据、参数或品种。

## 5. 裁决 §7 清单逐项勾选

### A. 定义 / 字段
- [x] 唯一变量为 D_parent.left 切到 c_start_full；D_child/方向/cand_delta/确认诊断门不变。
- [x] 每个父事件均输出 c_start_full / c_episode_start / c_end_full（None 明示拒绝）。
- [x] 每个完整事件附 B_p、c_p 首尾及第三类；完整证书内部一致。
- [x] 已统计 == / < / > / 缺失；> 由结构归属解释且未静默接受。
- [x] D_parent 未扩到父 A 或整趋势起点。
- [x] confirm_src / lag_conf / epsilon_conf 仅诊断。

### B. 漏斗
- [x] 旧基线 3 / 13 / 22 复核；漂移时停止横比。
- [x] 已输出各级新旧候选、边、完整父事件、完整链证书及差分。
- [x] 所有新增 L1→L2 边均逐条打印要求字段（空集亦明示）。
- [x] 每条新增边均有独立递归 sub_moves 归属证明。

### C. 预注册样本
- [x] 三例按原 parent confirm/episode 与 child confirm/a_interval 身份唯一重定位。
- [x] 已报告新左端或未闭合、有符号差、Sub 左右界。
- [x] 任一通过则标 FALSIFIED 并附 c_p 分解。
- [x] 三例全不通过则仅标 SUPPORTED-ON-THIS-REPLAY。

### D. 最终判定
- [x] `PASS`
- [ ] `FAIL-DEFINITION-MAPPING`
- [ ] `FAIL-EVIDENCE`
- [ ] `BLOCKED-CAPABILITY`

## 6. 验证记录

| 命令 | 结果 |
|---|---|
| `cargo run --release --bin strict_nest_check` | PASS；4,613,599 bar；1545.8s；P1 26,618 次比较、0 mismatch |
| `cargo check --all-targets` | PASS |
| `cargo test --bin strict_nest_check` | 9 passed |
| `cargo test nest --lib` | 109 passed / 0 failed / 3 ignored |
| `cargo test cp_ --lib` | 4 passed / 0 failed |
| `git diff --check` | PASS |
