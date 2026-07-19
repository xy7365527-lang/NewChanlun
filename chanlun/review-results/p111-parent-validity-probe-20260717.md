# p111 父级背驰有效性判别探针：全部被拒链边父级的 (a)/(b) 实证归属

日期：2026-07-17 ｜ 数据：btc_1m_full.json（4,613,599 bar，as_of=4,613,598）｜ 状态：**定稿**
探针：`rust/src/bin/p111_parent_divergence_validity.rs`（新写，只读；bin 自动发现，未动 Cargo.toml）
探针输出：`/tmp/p111_full.txt`（全量）；`/tmp/p111_full_rerun.txt`（复跑，逐字节一致）；`/tmp/p111_smoke.txt`（25 万 bar 冒烟）
判别式来源：`chanlun/review-results/doc-divergence-validity-test-20260717.md` §2/§5（下称 doc，引用 `doc:行号`）
复核输入：`/tmp/p92_ckpt_dump.txt`（只读，下称 dump，行号引其）

## 0. 结论先行

**全部 30 个被拒链边父级（去重事件）跑完修正版判别式：(b) 拒发正确 17 个、(a\*) 父级背驰不成立
13 个；但 (a\*) 内部 (a1)『背驰段仍在延伸、应以修正父区间重发』= 0 ——没有任何一条死链可按
重锚复活（30 条包含门死链 0 复活、24 条 A 证书判负边 0 复活）。doc §4 的 p102 四条『子在父右』
边全部实证为 (b)。**（数据事实）

- 判别式实装（doc:124 修正逐字落地）：Trend 父级阈值=最后中枢（B）**DD**（029:30）、Pan 父级=
  中枢（A）**ZD**（020:60）；Short 镜像 Trend→GG、Pan→ZG。核心判据=**时序**「先触阈值、后破
  极值」⟹(b)；「先破极值、未触阈值」⟹(a\*)；日历窗口 (parent.end, child.end] 只是近似
  （doc:109-110,149），本探针以 (parent.end, 数据末端] 全扫描为准、窗口事实并列输出。
- 父级总体 = 30 个去重父级事件：p109 结构链包含门 30 条击杀边的同侧父池（`P111_INCLUSION_ROWS
  rows=30`，与 p109 逐链一致）＋ A 口径证书 24 条 is_sub@B 判负 rung 边（`P111_CERT_EDGES_A
  b_fail_edges=24`，含 p102 五边锚 6/6 行命中）＋ 对照组 B 唯一多级证书 rung 父级 1 个。
- (a\*) 13 个父级的细分（doc §5.2，doc:150）：**(a1)=0**；(a2) 力度转强 1 个（边行 2 条）；
  (a3) 父级层面本无背驰力度结构（`divergence_confirmed=false`）8 个（边行 16 条）；其余 (a\*)
  父级仅出现在『子在父左』边（重锚不适用，边行 9 条）。⟹ doc §4(i) 预设的『(a1) 下存在教义上
  应发而未发的链』在本数据集**不存在实例**（doc:144）。
- 对 p102 台账：doc:153 §5.5 裁定『(b) 成立方可签市场事实』——四条边全 (b)，p102 §0(a) 的
  「Long 侧多级 0 张＝市场结构真相」**经教义强制检查后立住**，无需改写为装配工件；(a3) 案例
  下拒发理由从『无嵌套关系』改写为『父级非真背驰段』，但拒发结论不变（doc:115 对称保全）。
- **新发现（待裁定）**：对照组——B 口径唯一 exec=1 多级证书（dump:806）的 L2 rung 父级
  `2:147126:143038-147126` 被判 (a3)（先破高未回 ZG 且 MACD 未确认）。同一判别式回问**已签发**
  证书的 rung 父级，其被冻区间非真背驰段。生产 #97 裁定 rung 不要求 divergence_confirmed；
  该证书的教义地位需另行裁定，本报告只测量不建议（§6）。

## 1. 探针规格（doc §5.1 的工程落地）

对每条被拒边的父级事件：

1. **最后中枢**：Trend→B（生产同一 `DivergencePair` 的 `block_end_center` 中枢，
   `level_view.rs:674-676` 同路径）；Pan→A（c 段 start 前最近已确认中枢，`signal.rs:208`
   `nearest_confirmed_center_idx` 的探针内等价复刻）。阈值：Long＝Trend DD(B)/Pan ZD(A)；
   Short＝Trend GG(B)/Pan ZG(A)（doc:62,99,110,114 的镜像）。
2. **时序扫描**：自 parent.end（=interval_b.1=turn_source）下一 bar 起扫至数据末端。
   Long：触阈值＝`high ≥ threshold`；破极值＝`low < c_low`（c_low=interval_b 内 bar low 最小值）。
   Short 镜像（`low ≤ threshold`；`high > c_high`）。
3. **三态输出**（doc:149）：`TOUCH_BEFORE_BREAK`→(b)；`BREAK_BEFORE_TOUCH`→(a\*)；
   `NEITHER`/`SAME_BAR` 单列观察（本次均为 0）。
4. **(a\*) 细分**（doc:150 §5.2）：父事件 `divergence_confirmed=false` ⟹ (a3)（027:136 型，
   父级层面本无背驰力度结构）；否则『子在父右』边在 child.end 处重测父级 c（延伸至 child.end）
   对 seg_a（教义 b 段）的 MACD 面积（061:26 口径，`segments_diverge` 同函数）：仍背驰 ⟹ (a1)
   （可重锚：`is_sub(child_b, (seg_c.0, child.end))`）；转强 ⟹ (a2)。
5. **近似声明**（doc:149）：029:30 的反弹教义指次级别走势类型，bar 级触价是操作近似
   （029:152 缠师接受触价粒度）；bar high/low 与中枢 ZD/DD 同量化 Tick 域，整数比较。

## 2. 硬锚（全部 PASS）

- `P111_ANCHOR_EVENTS candidates=3683 divergence_confirmed=1235 trend_confirmed=429` —— 与
  p92 postfix 回归 / P109_ANCHOR_EVENTS 逐字一致（`p92-postfix-regression-20260717.md:16`）。
- `P111_ANCHOR_CERT reproduced=66 dump=66 missing_in_repro=0 extra_in_repro=0` —— 终态快照
  双口径证书集与 dump CERT 66 行（dump:753-818）双向 diff=0。
- `P111_ANCHOR_ENUM chains=6482 dp_complete=6482`；`P111_ANCHOR_GATE` identity=3790 /
  terminal=1631 / divergence=1020 / **inclusion=30** / direction=11 / alive=0（未打印即 0）
  —— 与 p109 §0 门级对账逐字一致（`p109-chain157-gate-attrition-20260717.md:16-26`）。
- p102 五边锚：`p102_anchor=true` 命中 6 行 = p102『5 条去重边、按证书逐条共 6 行』逐字吻合
  （`p102-b-long-zero-attribution-20260717.md:48-50`）；几何互洽：chain=2 左/右 gap +2719/−1883、
  chain=1944-1946 +13870/−13690、chain=5450 −4458/+4831 与 p102 §3、p109 §3.4 逐字一致。
- **中枢复原 miss=0**：`P111_CENTER_RECOVERY recovered=3683 miss=0` —— doc:151 §5.3 预言的
  工件缺口坐实（`NestCandidateEvent` 无中枢字段，`level_view.rs:475-490`），探针侧几何复原
  全量成功，无 `P111_CENTER_AMBIGUOUS` 行（Trend 事件的 pair→中枢映射无一歧义）。
- 冒烟锚：25 万 bar 复现 11 张证书全部 ∈ dump（extra=0，即首检查点 A=7/B=4 子集，同 p109 冒烟
  口径 `/tmp/p109_smoke.txt`）。复跑确定性：全量连跑两次 stdout 逐字节一致（`diff` 空）。

## 3. 父级判别结果（30 个去重父级事件）

| 判定 | 父级数 | 明细 |
|---|---:|---|
| (b) TOUCH_BEFORE_BREAK | 17 | 见下表 |
| (a\*) BREAK_BEFORE_TOUCH | 13 | (a3) 8 个；(a2) 1 个；仅『子在父左』边出现 4 个 |
| NEITHER / SAME_BAR / NO_DATA | 0 | — |

### 3.1 doc §4 四条『子在父右』边的逐边归属（裁定材料）

doc:138-141 强制判别式逐边执行结果（`P111_EDGE p102_anchor=true` ＋ `P111_PARENT` 行）：

| 基例 | 边 | 父级 kind | 阈值（Tick） | c 段极值 | t_touch | t_break | 判定 |
|---|---|---|---|---:|---:|---:|---|
| 706241 | L2→L1 父 704358 | Pan | ZD(A)=3955.0 | 3764.0 | 705160（+802） | 708318 | **(b)**，非平凡回触 |
| 1588321 | L2→L1 父 1574631 | Trend | DD(B)=11650.0 | 11776.0 | 1574632（+1） | 1574636（+5） | **(b)**，平凡几何* |
| 1834972 | L2→L1 父 1832757 | Pan | ZD(A)=47127.8 | 45570.8 | 1832907（+150） | 1844584 | **(b)**，非平凡回触 |
| 706241 | L3→L2 父 689893 | Trend | DD(B)=3224.6 | 3324.3 | 689894（+1） | 692030 | **(b)**，平凡几何* |

（价格按 Tick÷10⁸ 缩放到美元量级展示；+N＝parent.end 后第 N bar。）

\* **平凡几何标注（口径产物）**：1588321 与 706241-L3 两例中，父级 c 段极值**未跌破 DD**
（11776.0 > DD 11650.0；3324.3 > DD 3224.6；但均跌破 ZD，离开段成立）。『反弹触 DD』在 c 段
结束时已平凡成立——029:30 保证的最弱兑现在此无额外要求，其后新低按 027:314/029:252 属新结构
（『背了又背』＝更大级别走势未终结）。判别式通过的判定为 (b)，但这两例的 (b) 由几何平凡性
驱动而非显著回拉，裁定引用时建议附带本注。另两例（706241-L2、1834972）c 段深破 ZD 后反弹
802/150 bar 触回 ZD，(b) 非平凡。

⟹ doc:143『原文未决』的四边归属实证落地：**全 (b)，拒发正确、市场事实**（doc:144 第 (ii) 分支）。
p102:75-78 的『回试/反应段』归因在补上教义强制检查后成立（doc:130-132 的三步审查至此闭环）。

### 3.2 p102 第五边（3745754，子在父左）

父 3750585（Pan，div_confirmed=false）：t_break=3750690 先破低、t_touch=3753393 后触 ZD ⟹
BREAK_BEFORE_TOUCH，细分 **(a3)**。该边形态为『子在父左』（前 C 区，p102:80），doc 判别式针对
『子在父右』，父级有效性系顺带判出：父级 Pan 背驰 MACD 未确认且其后先破极值——父级层面
本无该级背驰结构（027:136 型），子级嵌套对象不存在，拒发在教义上**更加**正确（doc:115）。

### 3.3 (a\*) 13 个父级全表

| 父级 | kind/侧 | div_confirmed | t_break→t_touch | 涉及边 | 细分 |
|---|---|---|---|---|---|
| L2:1583467 | Pan/Long | false | 1583470→1589676 | chain 1947-1949（右）×3 | a3 |
| L2:1595601 | Pan/Long | true | 1595611→1652789 | chain 1947-1949（左）×3 | shape_not_right |
| L2:3750585/3751058/3752114 | Pan/Long | false | 各先破低→3753393 | chain 5450（左）×3＋CERTA×1 | a3 |
| L2:74621 | Pan/Short | true | 74629→658921 | base 79510（右）×2 证书边 | **a2** |
| L2:1745573 | Pan/Short | false | 1745648→2534733 | chain 2313-2318（左）×6 | a3 |
| L2:3060088 | Pan/Short | true | 3060144→3060201 | chain 4788/4789＋CERTA（左）×3 | shape_not_right |
| L2:3062339 | Pan/Short | false | 3062360→3062412 | chain 4788/4789（左）×2 | a3 |
| L2:147126 | Pan/Short | false | 147661→243238 | 对照组（活体 B 证书 rung） | a3，§6 |
| L3:3625589 | Pan/Short | false | 3625599→3655623 | CERTA top=3（右）×1 | a3 |
| L3:1766034 | Trend/Short | true | 1766053→1766421 | CERTA top=3/4（左）×2 | shape_not_right |
| L4:1905266 | Trend/Short | true | 1910442→1921904 | CERTA top=4（左）×1 | shape_not_right |

- 唯一进入力度重测的『右形＋确认』(a\*) 父级是 74621：扩展 c 面积 349.85B ≥ seg_a 面积 17.41B
  （20 倍，061:26『力度大于前者』逐字形态）⟹ **(a2) 背驰段不成立**，父证无效方向——但该父级
  本就非任何 B 证书成员（A 证书 rung），拒发结论不变。
- (a1)＝背驰段仍在延伸且可重锚：**0 实例**。`P111_REVIVE chains_inclusion_killed=30
  chains_revivable_a1=0 revive_levels={} cert_edges_revivable_a1=0`。

## 4. 日历窗口近似的实证误差（doc 修正的必要性证据）

85 条边级判定行（CHAIN 61＋CERTA 24）上，窗口 (parent.end, child.end] 近似 vs 时序判据：

| 窗口内事实 | 边行数 | 时序判据结果 | 窗口近似的表现 |
|---|---:|---|---|
| 触∧¬破 | 27 | 全 (b) | 一致 |
| ¬触∧破 | 6 | 全 (a\*) | 一致 |
| 触∧破（窗口无法定序） | 9 | 全 (b) | **歧义**，原 formulation 不可判 |
| ¬触∧¬破（窗口无信息） | 43 | 22 (b)／21 (a\*) | **无判定能力** |

⟹ 52/85（61.2%）边行上日历窗口要么歧义要么无信息，doc:124『时序条件是判别核心、日历窗口
只是近似』的修正在实证上是**必要**的而非润色；凡窗口可直接判的 33 行，时序判据全部一致
（0 例冲突）——两 formulation 无矛盾案例，时序版严格占优。（数据事实）

## 5. 对既有台账的影响

- **p102（#102）**：四条 doc 边全 (b) ⟹ §0(a)『市场结构真相』经教义强制检查后维持；第五边
  （3745754）父级判 (a3)，但该边本就是『子在父左』的前 C 区判负，拒发结论与 p102:80 一致且
  理由补强（父级非真背驰段）。**0 张 Long 多级链的归因不改写**。
- **p109（#109）**：包含门 30 条击杀全部维持拒发——(b) 边＝无嵌套关系该拒；(a3) 边＝嵌套对象
  非真背驰段更该拒；(a2) 边＝父级背驰不成立该拒；左形边＝重锚不适用。doc:115 对称保全逐字
  落地：裁定②不受本测试威胁，0 条链可复活。
- **doc §4(i) 预设的 (a1) 分支**（『确认谓词过早冻结 c 终点』装配工件假设，doc:144-145 赔率
  提示）：本数据集**证伪**——13 个 (a\*) 父级无一在 child.end 处仍保持背驰力度；唯一 right 形
  确认父级（74621）力度 20 倍转强。(a1) 零实例 ⟹ Long 侧不存在『应发而未发的多级链』。
- **46.5% 待裁定池**（p109 §5：754 谓词分歧／1,238 pan 定位／1,020 pan MACD）不在本探针射程
  （那些边无父级事件可测），本报告不对其投票。

## 6. 对照组新发现：活体 B 证书的 rung 父级被判 (a3)（待裁定）

B 口径唯一 exec=1 多级证书（dump:806，`2:147126:143038-147126|1:147126:147050-147126`，Short）
的 L2 rung 父级事件 `L2:147126`：ZG(A)=8160.3，c_high=9654.3；t_break=147661（c 段结束后
535 bar 创新高）先于 t_touch=243238（回落触 ZG，96,112 bar 后）⟹ BREAK_BEFORE_TOUCH；且
`divergence_confirmed=false` ⟹ (a3)。

- 含义：同一判别式不只裁决被拒边，也回问**已签发**证书的 rung 父级——该被冻区间
  (143038,147126) 按修正判别式非真背驰段（先破极值未回中枢、MACD 未确认）。
- 边界（090 纪律，只测量不建议）：(i) 生产 #97 裁定 rung 级不要求 divergence_confirmed
  （`nest.rs:550-556` 注释），该证书按现行裁定合规；(ii) doc §5 探针规格的对象是『被拒链边』，
  对已签发证书 rung 是否适用同一判别式、适用后该证书的教义地位如何，**裁定事项**，不在本
  报告权限（doc:152 §5.4 同纪律：探针不得逆流入证书门）；(iii) p110 已记 judge_at 语义与
  终态冻结的口径产物性质（`p110-assembly-artifact-audit-20260717.md:80-97`），本发现与之
  正交：它是价格-中枢维度的事实，不是钟维度。

## 7. 边界、近似与纪律声明

- 口径：触/破判定用 raw bar high/low（含 untradable bar，纯价格事实）；c 段极值取 interval_b
  内 bar 极值（bar 级口径，比 range_envelope 的段端点口径更深，破极值检测偏保守——更早判破）。
  1m bar 内触破同时发生的 `SAME_BAR` 情形实测 0 例，无 bar 内时序不可知问题。
- 力度重测（a1/a2）仅对『右形＋divergence_confirmed』(a\*) 父级执行（doc:150 定义域）；
  面积＝`segment_macd_area`（Σ\|hist\|，merged 序列，`map_src_to_close_idx` 映射）——与生产
  `segments_diverge` 同函数同序列，无第二口径。
- 『子在父左』边的 (a\*) 父级不参与复活统计（重锚区间在其右侧，与左方子级无交集）；方向门
  11 条击杀的父级方向不一致，不在判别式对象内（p109 已定数据事实）。
- 中枢复原是探针侧只读补位（生产源码零改动）；doc §5.3 工件缺口（事件记录无 DD 字段）坐实
  但可补——3683/3683 复原成功、零歧义。
- 生产源码零改动；未改 `rust/Cargo.toml`（bin 自动发现）；未做任何 git mutation；主仓
  `/Users/silencehan/Projects/NewChanlun` 零写入（数据经 worktree 内 symlink 只读消费；dump
  只读）。新增文件仅 `rust/src/bin/p111_parent_divergence_validity.rs` 与本报告。
- 声明与能力一致：本探针做终态快照父级有效性判别＋(a\*) 力度细分；未做 prefix 首证钟重放
  （判别式不需要钟）；不声明任何择时 alpha；不为增产放宽任何判据。

## 8. 复现

    cd rust && cargo run --release --bin p111_parent_divergence_validity -- \
      ../analysis/data_cache/btc_1m_full.json /tmp/p92_ckpt_dump.txt   # 全量约 40s
    P111_MAX_BARS=250000 <同上>                                        # 冒烟
    # 复跑确定性：连跑两次 stdout 逐字节一致（/tmp/p111_full.txt vs _rerun.txt）
