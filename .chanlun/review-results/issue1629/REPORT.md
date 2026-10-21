# #1629：完整事实入口端点合并 Θ(n²) → 共享子树指针相等剪枝

- 分支：`codex/1629-endpoint-prune`，基于 #1628 head `2d95fd36f8`（`codex/1597-coords-shared`）。**依赖 #1628**（#1628 又依赖 #1596）。
- 远端：windows-5090 WSL（8 核，47 GB），`nc-remote` 私有 target（`NC_TARGET=issue1629*`），release 构建。本机未跑 cargo。
- 数据与窗口口径沿用 #1597：`btc_1m_full.json`（sha256 `16ea13d5…707b`，4,613,599 根）；尾 `W` 根去前 5 根取前 n 根（n = W−5）。注意：不同 n 是**不同起点、同一终点**的历史后缀，不是同一次运行的前缀。
- 读数脚本：`bench.sh`（每次运行前后记 loadavg / MemAvailable / CPU 前 5 进程）；日志在 `logs/`。

## 改了什么

#1628 的端点来源并集 `union_into` 先把其余集合与基底树做一次有序归并——遍历整棵基底，单端点 Θ(i)，总时间 Θ(n²)。本 PR：

1. `evidence.rs` 新增持久集合并集 `CoordsRef::union`（`union_nodes` + `split_node` + `join_opt`）：两棵树指向同一节点 ⟹ 整棵跳过；值域不交 ⟹ 一次 AVL 拼接；否则按 a 的左子树最大值切分 b、左右递归；左右结果都与 a 的原子树同指针 ⟹ 原样返回 a（可归纳证明「返回 a 的指针」⟺ b ⊆ a，不新建留存节点）。
2. `union_into` 改为逐元素持久插入（已含则返回原路径），不再遍历基底：O(|others|·log² n)。
3. `stroke.rs`：端点来源 = 基底证据树 ∪ 其余（去重后的）证据树（`union`）∪ 组成员（`union_into`）；确认证书来源 = 两端点来源树的 `union`。
4. 成本计数口 `cost_probe`（仅 `#[cfg(test)]`，非测试构建为空函数）：`TREE_OPS` = 并集/切分/插入的递归调用次数；`TREE_ITER_STEPS` = 逐节点遍历共享树的步数（`SourceCoords::iter`、`CoordsRef::materialize`、`CoordsRef::count`）。
5. 探针 `issue1588_parse_mem_probe`：`ISSUE1629_LAYER_ONLY=1`（≥1M 前缀只摘 layer，Θ(n²) 的 Debug 摘要不可行）；`ISSUE1588_SKIP_FACTS_DIGEST` 同时跳过 Θ(n²) 计数；rustfmt（#1628 遗留）。

公开合同不变：`SourceCoords` 的 `Debug`/`Serialize`/`PartialEq` 只看元素序列；树形状本就不对外承诺（`root_node()` 的唯一下游 `content_producer.rs` 遍历的是包含证据树，本 PR 未动，且按规范 M 序列取内容身份，独审核实）。

**复杂度与退化**：完整事实路径上的证据树都来自同一条持久追加链（`E_{g+1} = concat(concat(E_g, tail_g), leaf)`），两处调用都以最大值较大者（较新、超集）为 a，正是剪枝生效的方向；每处不共享接缝 polylog。两树内容重叠但**不共享指针**时退化为 Θ(min)·polylog（慢于有序归并）——完整事实路径到不了，文档已写明。递归深度 ≤ 树高 ≈ 32（4.6M），无栈风险。

## 输出与 #1628 逐位相同

| n | 对拍面 | #1628 | 本 PR |
|---|---|---|---|
| 49,995 | layer / stroke_facts / 整份 facts Debug 摘要 | `0b2f23ed…4dde7` / `9fa0c2d5…8775` / `4c926f23…ee05` | 相同 |
| 49,995 | S 生产入口 `group-evidence` 末根一次：layer/groups/stroke_facts/segment_facts | `0b2f23ed…` / `bcc44397…5e5b` / `152214cd…dc78` / `2d1a1645…36c1` | 相同 |
| 20,000 | `group-evidence` 每 500 根滚动摘要 | `dd8b0180…410c` | 相同 |
| 149,995 | layer / stroke_facts | `3a6e95ed…6e4e` / `4ca1de12…0c3f` | 相同 |
| 299,995 | layer（facts 入口 vs 紧凑入口） | `94028455…5127` | 相同 |
| 1,000,000 / 2,000,000 / 3,000,000 / 4,613,495 | facts 入口 layer = 紧凑 `parse_layer` layer | — | `29a0a3ee…` / `ba0ec812…` / `eff2d82a…` / `bb067c8b…` 两入口相同 |

#1628 侧摘要取自 #1597 远端日志 `20261021-065736-0f747b938c.log`（#1628 分支上的 `0f747b938c`；其后到 head `2d95fd36f8` 只改测试、探针与文档，解析代码相同）；group-evidence 摘要取自 #1597 报告（修前 = 修后）。≥149,995 的 Θ(n²) facts 整份摘要与 ≥1M 的 stroke_facts 摘要不可行（展开元素 >10¹⁰），逐位等价另由逐前缀单测 `shared_full_facts_match_flat_reference_for_every_prefix`（PartialEq + `{:?}` + `{:#?}` + JSON，对扁平参照）承担。

## 读数：完整事实入口 `parse_layer_with_inclusion_facts` 一次全量调用

两轮（`logs/scaling-rep-af6976de06.txt`），取较小值；同一轮紧跟紧凑入口 `parse_layer` 作宿主噪声参照。宿主 loadavg 全程 9.7–12.4（8 核；其他代理作业，含 #1597 遗留的 #1628 299,995 探针单核满载），未能取得独占时段。

| n | #1628 secs | 本 PR secs（两轮） | 紧凑 layer secs | facts/layer | alloc_total | peak | retained |
|---|---|---|---|---|---|---|---|
| 49,995 | 31.73 | **0.22**（0.31 / 0.22） | 0.25 | 0.9 | 297.5 MB（#1628 320.7） | 129.5 MB | 98.0 MB |
| 149,995 | 232.41 | **1.00**（1.00 / 2.15） | 0.95 | 1.1 | 1,074.9 MB（1,200.4） | 463.1 MB | 344.9 MB |
| 299,995 | 2,586.63 | **2.38**（2.38 / 3.05） | 1.81 | 1.3 | 2,175.3 MB（2,449.3） | 937.3 MB | 700.2 MB |
| 1,000,000 | 外推 ≈1.3×10⁴（n²） | **9.19**（9.19 / 11.52） | 4.74 | 1.9 | 6,681.9 MB | 2,800.4 MB | 2,166.1 MB |
| 2,000,000 | — | **20.16**（20.16 / 21.96） | 10.17 | 2.0 | 13,547.0 MB | 5,672.9 MB | 4,407.0 MB |
| 3,000,000 | — | **25.88**（28.50 / 25.88） | 13.87 | 1.9 | 20,130.5 MB | 8,776.6 MB | 6,820.8 MB |
| 4,613,495（全史） | — | **39.09**（50.55 / 39.09） | 22.05 | 1.8 | 35,847.5 MB | 15,432.2 MB | 11,693.2 MB |

alloc/peak/retained 两轮逐位相同（确定性）。另一轮单次读数（`logs/big-full-af6976de06.txt`、`logs/near-full-af6976de06.txt`）：1M 10.48 s、2M 19.53 s、3M 57.47 s（同时段 loadavg 11–12 且有并行构建）、全史 70.67 s——墙钟受宿主噪声影响可达 2×，分配读数不受影响。

**斜率（log-log 最小二乘，七点取较小墙钟）**：墙钟 1.14（49,995→1M 段 1.24，≥1M 段 0.93）；alloc_total 1.03。紧凑 `parse_layer` 自身斜率 ≈0.99，facts/layer 比在 ≥1M 稳定在 1.8–2.0 ⟹ 完整事实入口与紧凑入口同阶，剩余超线性是小前缀段的常数项与每接缝 polylog。#1628 的墙钟斜率约 2（49,995→149,995 为 1.81；299,995 那次读数宿主有负载，偏高）。

S 生产入口 `parse_layer_with_group_evidence`（49,995，末根调用一次）：修前 83.24 s → #1628 21.94 s → **本 PR 0.21 s**；alloc 258.4 → 235.2 MB；peak 118.4 MB。

## 全史成本

- **直接实测**（不是外推）：4,613,495 根（`W=4,613,500`）一次全量调用 39–71 s（宿主负载 9–12），peak 15.4 GB，retained 11.7 GB（≈2.5 KB/根），alloc_total 35.8 GB。
- #1628 同口径外推（n²，以 49,995 的 31.73 s 为基）：31.73 × (4,613,495/49,995)² ≈ 2.7×10⁵ s ≈ 75 h；以 299,995 的 2,587 s 为基 ≈ 6.1×10⁵ s ≈ 170 h。本 PR 快约 4–5 个数量级。
- 内存：全史 peak 15.4 GB，在 47 GB 远端可行；Destination ④ 若要在更小机器或并发多品种回放，retained 2.5 KB/根是下一处约束。

## ⚠️ 旁见（非本票，未改）：全文件起点窗口退化为单组

`W=4,613,599`（全文件去前 5 根）时 facts 与紧凑 `parse_layer` **都**得到 `merged=1 fractals=0 strokes=0`（摘要 `1483a9ef…`，两入口相同，非本 PR 引入）。窗口起点扫描（`logs/degenerate-window-probe-af6976de06.txt`，layer 入口）：起点下标 9 / 599 / 3,599 / 13,599 / 213,599 退化为单组；起点 104 与 613,594 正常（3,037,848 / 2,702,328 组）。数据首段（2017-08-17 04:00 起）是一串 high=low=4261.48 的平盘根。疑与 #1393（TB-02-U 未建方向与全包含前缀）口径相关，**Destination ④ 全史回放从文件头起算时会撞上**，建议 #1393 / #1323 线核实；本报告「全史」读数改用起点 104 的 `W=4,613,500`。

## 测试

- `cargo fmt --all -- --check` 通过；`cargo test --release --lib theta_v0::parser`：190 passed / 0 failed / 22 ignored（`af6976de06`，`logs/tests-af6976de06.txt`；#1628 为 187，新增 3 个）。
- 新增测试：
  - `coords_ref_union_matches_set_semantics_and_keeps_inputs`：60 轮随机树（含单元素、重叠、不相交、同指针、空集）与同谱系新旧版本双向，对照 BTreeSet；逐节点核高度字段与 AVL 平衡；核输入版本不被改写；旧 ⊆ 新时返回原指针。
  - `coords_ref_union_cost_is_decoupled_from_size`：N = 1k/8k/64k，新版本 ∪（近版本 / n/2 远版本 + 插入）再逐元素插入，遍历步数 = 0，递归调用次数 148/202/256（N ×64，步数 ×1.7）。
  - `shared_stroke_facts_cost_is_decoupled_from_prefix`：LCG 行情 n = 1k/4k/16k 构造新笔事实，遍历共享树步数 = 0，每端点递归调用次数 152/184/217（n ×16，×1.43；阈值 3×）。
- 既有锁全部保留：扁平参照逐前缀对拍、留存形状锁（共享节点 < 64n、扁平元素 = 0）。

## 变异负控（本地临时分支，未推送；`logs/mut-*.txt`）

| 变异 | 内容 | 结果 |
|---|---|---|
| M1 `9e2d998546` | 去掉 `ptr_eq` 剪枝与「左右未变返回原 a」（结果仍正确，只是慢） | 4 FAILED：两把成本锁、留存形状锁、并集语义锁（原指针断言） |
| M2 `be6ce65f0f` | `union_into` 改回 #1628 的「与基底有序归并」 | 2 FAILED：两把成本锁（遍历步数 ≠ 0） |
| M3 `8f5a21b0d7` | 切分键改为 `max(left) − 1`（边界元素落错侧 → 重复） | 5 FAILED：并集语义锁、扁平参照对拍、留存锁、两把成本锁 |
| M4 `957f0d5c95` | 只判左侧未变就返回原 a（丢右侧新元素） | 2 FAILED：并集语义锁、并集成本锁（计数断言） |
| M5 `1cea720521` | 组成员跳过第一个 | **存活**（190/0）：等价变异——窗口首组首成员必在基底证据树内，插入本就不新建节点 |
| M5b `d579a7a79e` | 组成员插入全部去掉 | 1 FAILED：扁平参照逐前缀对拍 |

## 非作者独审

`review-claude-opus-5-5.md`：本机 `claude` CLI OAuth 过期，经本机现成的 `claude-proxy` 路由（CLIProxyLocal `127.0.0.1:8318`，密钥由脚本从 0600 文件读入环境，未打印）以 `claude-opus-5-5` 执行，只读审查。结论 **APPROVE**，0 HIGH / 2 MEDIUM / 6 LOW，全部已处理：

- M-1（计数口径只计递归入口）：注释与 PR 文案改称「递归调用次数」，写明实际指针跳转约 10–30 倍、同为 polylog；附 ≥1M 与全史墙钟/分配收据。
- M-2（遍历探针只装在 `iter`）：`materialize`、`count` 也计遍历步数（`len`/`to_vec`/`contains` 经由二者或 `iter`）；改后两把成本锁仍为 0。
- L-1 过期注释、L-2「零分配」措辞、L-3 复杂度界与退化形态、L-4 远版本（n/2）成本锁、L-5 不变量构造口清单：已改。L-6（单种子）：补真实行情读数（上表）代替加种子。

独审在 `6b270b145d` 上进行；其后提交只改测试、注释、`#[cfg(test)]` 计数口与探针，非测试构建的解析代码与读数所用 `af6976de06` 相同。

## 残余

1. 全文件起点窗口退化为单组（见上，非本票）。
2. 每端点 polylog 的常数：facts/layer ≈ 2；若要再降，可在 `CoordNode::Concat` 缓存 min/max（`insert_node`/`split_node` 每层的 `node_max` 下行），但 `CoordNode` 是公开枚举、下游 `content_producer.rs` 按 `Concat(l, r, _)` 匹配，加字段会破坏下游，未在本票做。
3. 299,995 的 stroke_facts 整份摘要（Θ(n²) 展开）仍在远端计算，结果补在票评论。
