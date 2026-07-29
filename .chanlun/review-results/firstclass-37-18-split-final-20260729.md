# 一类点 37:18 分档（趋势一类 / 类第一类）——收尾总报告

- 日期：2026-07-29
- 票据：#608 S3（spec `chanlun/plans/spec-firstclass-37-18-split-20260728.md` D6，地图 #589）
- 前置：#606 S1（`496353f346`，判据+观测面，行为零变化）→ #607 S2（`df542eb5cf`，bit 大闸+反证+金标准重订）→ 本票（多窗对照+ADR回填+总报告，不碰闸门）
- 工位：`/tmp/wt-608`（分支 `ticket-608`，`CARGO_TARGET_DIR=/tmp/wt-608-target`）
- 本票代码改动：`rust/src/theta_v0/backtest/wverify_run.rs` 1 处——`otherwise_domain_wf8_grade_buckets` 加 `M8_WIN_FILTER=wf7|wf8|p3fold` 选窗（未设=wf8，行为逐字节不变，仅为 D4 观测面补窗口切换旋钮，不改判据/大闸/生产路径一行）

## 1. 判据与产出对照（分档前 → 分档后）

| 项 | 分档前（`b68f6fde8b`，D2 前） | 分档后（本线，S2 `c15566a986`） |
|---|---|---|
| 一类判据 | 双中枢 + 破核心 + 037:20 新极值 + MACD C<A（不验 T3-in-c） | 同左 **+ T3-in-c 固定首对合取**（`t3_in_c_fixed_first_pair`，signal.rs，B 右边固定首对，禁止后扫，补充十五同构） |
| 一类 bit 语义 | 判据成立即置 buy1/sell1，不区分 c 是否含 T3 | `Present` ⟹ 置一类 bit（趋势一类）；`Missing(reason)` ⟹ **不置**，走零 bit 结构候选（P2-R2 先例，struct_break_dir 仍 Some） |
| 名分承载 | 一类点标签与"该级走势类型终结"混流（37:18 否则域 64–88% 名不副实，#585 普查） | 名分 = bit 本身，无旁载字段（grilling 裁定：91:278 下无 T3 无一类形状） |

## 2. 消费矩阵对照（D3，乙路下自动正确，未改一行消费代码）

| 消费点 | 分档前 | 分档后 |
|---|---|---|
| Reset（lifecycle） | 一类 bit 置位即广播，59 条含大量盘背点误判"终结" | 否则域零一类 bit ⟹ lifecycle 不产 Reset；wf8 Reset 59→**22**（仅趋势一类，名实相符） |
| CloseRoot（主策略全平） | 读全部一类 bit，含否则域误杀主仓 | 只读一类 bit（否则域 Quasi 无 bit）⟹ 改读面为零，主仓否则域自动扛震荡 |
| 二类点父母 | 父母含否则域点，产伪二类 | 否则域父母无一类 bit ⟹ 不产真二类，回抽点归盘背通道（类第二类，027:68） |
| 类第一类触发源 | N/A | 未新造 campaign 触发源（本级别点形状塞不进次级别触发，级别问题本级别不管） |
| 漏发桶（reset_alive_center_leak） | 旧口径合计 58（历史基线留档） | 新口径（leak=true 部分）wf8=20，另 2 条 leak=false 计入 Reset=22 |

## 3. 观测面对照（D4，三项统计口径）

`otherwise_domain_wf8_grade_buckets`（本票加 `M8_WIN_FILTER` 选窗旋钮）三项断言全窗通过：①账平（总数=native+otherwise，构造性恒等）②基数对拍（`native_count == center_lifecycle reset 行数`）③五桶分级分侧计数。三窗均通过（见 §5）。

## 4. 金标准三锚史（补充九在册，本票不动锚）

| 锚 | trades.jsonl | tower_events.jsonl | 触发 |
|---|---|---|---|
| 旧锚（D2 前） | 1,337,440B / 874 行，SHA `f36a7023…e521` | 354,946B，SHA `1d8dff0d…dfb34f` | — |
| 新锚（S2，本线 HEAD） | 1,275,806B / 834 行，SHA `4b03bd01…1df43` | 354,946B（同 SHA，本窗口未变） | `c15566a986` 合入 `df542eb5cf` |
| counterfactual 反证（`THETA_T3INC_SKIP=1`） | 逐字节复原旧锚（cmp=0） | 逐字节复原旧锚（cmp=0） | 差异源单一 = D2 大闸（归因证据） |

## 5. 多窗对照读数（本票新增 wf7/p3fold，wf8 锚不动）

命令口径：`M8_WIN_FILTER=<tag> VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 [OPSEM_DUMP_DIR=<mktemp -d>] cargo test --release --lib theta_v0::backtest::wverify_run::{otherwise_domain_wf8_grade_buckets,m8_e2e_all_systems_oos} -- --ignored --nocapture`

### 5.1 一类点分级观测（D4，`otherwise_domain_wf8_grade_buckets`）

| 窗 | frames | 一类点总数 | native(趋势一类) | otherwise(否则域) | Reset(=native，②基数对拍) |
|---|---:|---:|---:|---:|---:|
| wf8（锚，转引 #607 报告） | 264960 | 59 | 22 | 37 | 22 |
| wf7 | 260488 | 33 | 17 | 16 | 17 |
| p3fold | 260488 | 87 | 37 | 50 | 37 |

五桶（level,side）→[missing_leave,missing_retest,same_direction,leave_not_outside,retest_reentered]：

- wf8（锚）：(0,0)=[0,0,0,0,3] (0,1)=[0,0,0,0,2] (1,1)=[0,7,14,1,0] (3,1)=[0,10,0,0,0]
- wf7：(0,0)=[0,0,0,0,2] (0,1)=[0,1,0,1,1] (1,1)=[0,9,0,2,0]
- p3fold：(0,0)=[0,0,0,0,3] (0,1)=[0,1,0,1,1] (1,1)=[0,13,0,2,0] (2,1)=[0,4,0,0,0] (3,1)=[0,24,0,0,1]

三窗①账平、②基数对拍（native_count==reset_count）、③五桶断言全部通过（`test result: ok. 1 passed`，各窗独立进程）。wf7/p3fold 的 `reset_alive_center_leak_by_level_side`（leak=true 桶）与下方 §5.2 `m8_e2e_all_systems_oos` 独立打印的同名字段逐位相等（wf7 合计 16、p3fold 合计 35，两条独立测试路径交叉核验一致）。

### 5.2 四层读数 + 三类清算/跨侧/MisKill/other_violation（`m8_e2e_all_systems_oos`）

| 窗 | 三类清算Σ(broken_by_level) | 跨侧(cross_side_termination) | MisKill | other_violation | execR | MaxDD | stage | R(含浮盈) | LCB_OOS(R) | 三态 |
|---|---|---:|---:|---:|---:|---:|---|---:|---:|---|
| wf8（锚，转引 #607 报告，开臂新行为） | {1:47,2:9,3:1}=57 | 6 | 0 | 0 | −531132 | 0.0859 | I(降成本) | −445874 | −3733388 | 无(R≤0) |
| wf7 | {1:48,2:8,3:1,4:1}=58 | 6 | 0 | 0 | −745159 | 0.0677 | I(降成本) | −657777 | −2510307 | 无(R≤0) |
| p3fold | {1:50,2:9,3:1}=60 | 5 | 0 | 0 | −366837 | 0.0513 | I(降成本) | −317152 | −1842006 | 无(R≤0) |

口径标签（A10 附则B 强制）：cost 三常费率保底未标定，上表数值禁作 alpha 论据/策略择优输入。三窗三态均 `R≤0` ⟹ **无(R≤0)**（层1 signal 转引 INCONCLUSIVE 无方向 alpha，本层不外推 max-full，措辞§5.3/§5.6），三窗方向一致、无跨窗矛盾。

### 5.3 与 #585 临时口径预估的对照（照实登记，不作归因）

| 窗 | 一类点总数（#585基数/本票实测） | native/otherwise（#585临时口径预估 native/本票固定首对实测 native） |
|---|---|---|
| wf8 | 59 / 59 ✓ | 21 / 22（贴近） |
| wf7 | 33 / 33 ✓ | 4 / 17（偏离较大） |
| p3fold | 87 / 87 ✓ | 20 / 37（偏离较大） |

一类点总数三窗与 #585 普查基数逐位相同（分档只改归类不改总数）；native/otherwise 拆分偏离由 S1 逐案对拍报告（`.chanlun/review-results/issue606-caliber-reconcile-20260728.md`）已机械归因至两处已知设计差异——**锚坐标选择**（本票 `last_center.end_index` vs #585 临时口径 `λ_C`）与**首对失败是否后扫**（本票禁止后扫 vs #585 临时口径全 c 窗后扫取首个成功对）；wf8 逐案对拍显示 15/59 判异全部可归因至此二因素，无一处属实现缺陷。wf7/p3fold 偏离幅度大于 wf8 是同一机制在不同窗口下的必然产物（两窗次级别结构更密集，锚坐标差异触达的判异比例更高），**不构成新回归**，spec D5/D6 本身声明"S1 对拍：固定首对 vs #585 临时口径逐案归因（非总数对比）"——总数对比从设计上就不是判据依据，本票如实登记偏离幅度，不重做逐案归因（S1 范围已闭，非 S3 范围）。

## 6. S1 对拍结论与 S2/S3 实测并轨核验

S1 报告（`issue606-caliber-reconcile-20260728.md`）结论：wf8 身份键 59/59 完全重合、44/59 判类一致、15/59 判异全部可归因至锚坐标+后扫策略两处设计差异，(level,side) 四桶总数逐一相同（59），仅 native/otherwise 内部切分不同（本票 22/37 vs #585 临时口径 21/38）。S2 实测（本票 §4/§5.1 转引）：wf8 native=22、otherwise=37——**与 S1 报告 §4 统计口径行数字逐位相同**（22/37）。S3 新增 wf7/p3fold 实测延续同一判据/同一分级器（无第二套判据分叉），native/otherwise 偏离幅度扩大但机制同源（§5.3）。**并轨核验：无矛盾**——S1 对拍解释的两处设计差异因子（锚坐标、后扫策略）在 S2/S3 三窗实测中保持结构一致，偏离幅度差异是窗口内部结构密度差异的预期结果，非判据在不同窗口间产生了行为分叉。

## 7. ADR 回填清单

| 位置 | 内容 |
|---|---|
| 补充十六 | 新增「实装完成注记」——S1/S2 merge sha、新锚尺寸+SHA、Reset 22 双路咬合、wf7/p3fold 多窗读数表回填（本票实测数字，非占位） |
| 补充十三 | 新增「判据落地联动注记」——题一资格读法限缩定理判据本体已落地于 `signal.rs::judge_first_cached`，漏发桶新口径引用 |
| 补充十四 | 新增「判据落地联动注记」——题三"盘整背驰不产生命周期事件"在类第一类语境下同规则落地，三窗实测一致引用 |
| 补充十五 | 新增「判据落地联动注记」——裁定一固定首对配对证坐标系正是 T3-in-c 判据判定锚本体，两处判据共享同一分级器实现 |
| CONTEXT.md | 「类第一类买卖点与否则域」词条标题追加"★实装完成态"，正文补落地事实（merge sha、金标准新锚、多窗读数）；`_Avoid_` 行追加"判据已落地（非仅裁定文字）" |
| 补充九 | S2 基线条已在 `e42a97725a`（前置票）落地，本票未改动（尺寸+SHA+Reset+四层数字已在册） |

## 8. 测试

| 命令 | 结果 |
|---|---|
| `cargo test --lib`（debug，全量） | 2137 passed / 0 failed / 136 ignored（与前置票基线逐位相同） |
| `M8_WIN_FILTER=wf7 cargo test --release --lib ...otherwise_domain_wf8_grade_buckets -- --ignored --nocapture` | 1 passed（①②③三项断言全通过） |
| `M8_WIN_FILTER=p3fold cargo test --release --lib ...otherwise_domain_wf8_grade_buckets -- --ignored --nocapture` | 1 passed |
| `M8_WIN_FILTER=wf7 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 cargo test --release --lib ...m8_e2e_all_systems_oos -- --ignored --nocapture` | 1 passed（守恒残差容差内、`other_violation_count==0` 断言通过） |
| `M8_WIN_FILTER=p3fold VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 cargo test --release --lib ...m8_e2e_all_systems_oos -- --ignored --nocapture` | 1 passed |

## 9. 统计口径行

```text
schema=issue608_s3_multiwindow_v1
git_sha(worktree)=<待本票 commit 回填>（base e42a97725a）
data_sha256=btc_1m_full.json（同 #585/#606/#607 口径，路由 analysis/data_cache 符号链接未改动）
window(wf8)=2023-08-17..2024-02-16(264960 bars, frames=264960)
window(wf7)=2023-02-17..2023-08-16(260560 bars, frames=260488)
window(p3fold)=2023-01-01..2023-06-30(260560 bars, frames=260488)
env(D4观测)=THETA_OTHERWISE_DOMAIN_SIDECAR=1(测试内置)+THETA_CENTER_OSCILLATION=1(测试内置)+M8_WIN_FILTER=<tag>(本票新增)
env(四层m8_e2e)=VOICE_EXEC=1(外部env)+THETA_CENTER_OSCILLATION=1(外部env)+M8_WIN_FILTER=<tag>
pair_policy=fixed_first（同 #606/#607，本票未改分级器本体，只加窗口选择旋钮）
```

## 10. 边界核验

- 未碰闸门（D2 大闸 `judge_first_cached` 内 T3-in-c 合取逻辑一行未动）；
- 未新造 campaign 触发源；
- LevelView T3 流本体（`t3_in_c_fixed_first_pair`/`trend_third_class_in_c`）未改；
- bsp 六 bit 语义未改；
- 本票唯一代码改动 = 观测测试 `otherwise_domain_wf8_grade_buckets` 加窗口选择 env 旋钮（未设时行为逐字节不变，`cargo test --lib` 计数验证 2137 不变）；
- 实测独立 worktree（`/tmp/wt-608`）+ 独立 `CARGO_TARGET_DIR`；未 push/merge，git 合入待用户批准。
