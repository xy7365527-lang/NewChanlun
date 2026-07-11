# P0 D_parent 复跑报告（2026-07-10）

## 1. 执行边界与提交链

- 分支：`p0-replay-dparent`；起点：`1dcdb95a09`。
- 已按序 cherry-pick `7a571d1a38`，只对当前基线的私有字段封装、getter 与构造器差异作最小冲突消解；本地提交为 `490b450199`。
- 实现前冻结提交：`2a4fcaa7dc`；实现提交：`9b46b45ea1`。
- 全程未 push。未修改 prereg、witness、goals、裁定文档或其他冻结资产；冻结文档在实现与复跑后未回改。
- 当前 harness 禁止写入工作区原 `.git`，因此上述提交记录在本机 shadow Git `/tmp/p0-replay-work2-shadow2/.git`，worktree 仍为本目录；代码与产物均在当前工作区。此环境限制不改变复跑输入或文件内容。

## 2. 重放前冻结定义

冻结文档：`chanlun/review-results/dparent-freeze-20260710.md`。依据裁决、`nest-lag-deep-research-20260709.md`（本机权威副本 SHA-256 `43b56ab127d40639c8c9c740d90a9de6608c20a898852e0e8063c05234677b36`）以及旧缠论第 24、27 课，口径如下：

1. `D_parent(p) := [p.enter_src, p.interval.1] = p.interval`。
2. 左端 `p.enter_src` 是最后一个已确认父级中枢之后，当前 C 离开 episode 中首个同向 Segment 的 `start_index`；生产入口为 `departure_move_c_start(...)`。
3. `D_child(c) := I(A_child) := c.a_interval`，不是 `I(C_child)`、确认窗或确认点。
4. 相邻边唯一结构门是闭包含 `child.a_interval.0 >= parent.enter_src && child.a_interval.1 <= parent.interval.1`，同时要求父子均 `cand_delta=true` 且方向一致。
5. `lag_conf := child.confirm_src - parent.interval.1` 仅登记有符号分布；`lambda_conf` 与 `epsilon_conf` 不参与过滤、排序或否决。

实现未发现与冻结文档冲突的字段能力或语义冲突。

## 3. 实现摘要

- `NestRung` 在保留旧构造路径的同时增加相邻边的 `child_interval` 证据；严格装配保存父级 `D_parent` 与子级 `I(A)`，逐边验证闭包含。
- 严格链基例改为基级 `I(A)`；每一级都按 `I(A_child) subset D_parent` 延长，而非把相邻事件的 C 区间或确认窗当作子对象。
- 装配排序仅使用结构字段 `(interval, a_interval, original_index)`；`confirm_src` 不参与排序，也没有确认时序门。
- 复跑漏斗显式过滤 `cand_delta=true` 与同方向，登记所有被考察候选和已接受边的确认延迟分布；`epsilon_conf` 只输出诊断标签。
- 新增回归覆盖：确认时序逆序仍可装配、C 区间越界但 A 区间在父 D 内可通过、C 区间在内但 A 区间越界必须拒绝、三级链逐边使用各自 `I(A_child)`。

## 4. 全量复跑结果

命令：

```text
cargo build --manifest-path rust/Cargo.toml --release --bin strict_nest_check
./rust/target/release/strict_nest_check
```

输入为 BTC 1m `4613599` bar（2017-08-17 04:00:00 至 2026-05-31 23:59:00）、`40001` 笔交易、`ThetaConfig::default()`（`l_max=6`、`min_parts_per_level=3`）；耗时 `1483.8s`。基线 sanity 全部一致，P1 共 `26618` 次逐 bit 比较、`0` 不一致；terminal 查无为 `0`。

### 4.1 cand_delta=true 产出与证书

| 级别 | Cand_delta 候选 | 从 L0 可达 | 完整证书 |
|---:|---:|---:|---:|
| L0 | 452 | 452 | 基例 452 |
| L1 | 7 | 3 | 3 |
| L2 | 13 | 0 | 0 |
| L3 | 9 | 0 | 0 |
| L4 | 2 | 0 | 0 |
| L5 | 0 | 0 | 0 |

L0 to L1 共得到 **3** 条证书，达到 `>=3` 的回归预期；该项按裁决只作非硬闸门。L1 to L2 仍为 **0/0**（相邻边 0、完整证书 0）。

### 4.2 lambda_conf(child) - right(D_parent) 延迟分布

分位数使用离散排序后的固定索引口径；全部数值单位为 bar。

| 相邻级 | 同向可达候选对 | 通过闭包含的边 |
|---|---|---|
| L0 to L1 | `n=1589`；负/零/正=`698/0/891`；min/p25/p50/p75/p90/p95/max=`-4221801/-1060996/233011/1622481/2837779/3368213/4257324` | `n=3`；负/零/正=`0/0/3`；min/p25/p50/p75/p90/p95/max=`60/60/87/87/87/87/183` |
| L1 to L2 | `n=22`；负/零/正=`12/0/10`；min/p25/p50/p75/p90/p95/max=`-3091118/-1415792/-14211/397509/1814313/1854358/2035579` | `n=0` |
| L2 to L3 及以上 | `n=0` | `n=0` |

延迟分布没有进入装配控制流；`epsilon_conf` 未设置数值、未作闸门。

## 5. L1 to L2 的 0/0 归因

首个归零点明确位于 L1 to L2：L1 已有 3 个可达 partial chain，L2 有 13 个 `cand_delta=true` 候选；同向可达候选对 22 个，但 `I(A_child) subset D_parent` 的边为 0。最接近的三个样本都只失败于 Sub 左界，子级 `I(A)` 分别比父级 D 左端早 `11504`、`19853`、`34445` bar；对应确认延迟 `-8518`、`-14211`、`-28864` bar 仅作诊断。

按裁决预注册的甲/乙假设，本次结果**倾向甲**：在当前 BTC 1m、默认 Theta、趋势背驰-only 与冻结 D_parent 映射下，高级别完整趋势背驰的严格逐级嵌套本来就稀少。L0 to L1 已恢复到 3、而 L1 to L2 的损失全部发生在冻结的结构 Sub 左界，因此“仍在使用错误 parent-window 对象”的乙假设被削弱。该归因是本数据与参数域内的证据，不是对其他品种、Theta 或盘整背驰的证明。

## 6. 回归测试

| 测试 | 结果 |
|---|---|
| `cargo test --manifest-path rust/Cargo.toml nest` | 37 passed |
| `cargo test --manifest-path rust/Cargo.toml --bin strict_nest_check` | 7 passed |
| strict sidecar runner tests | 2 passed |
| cand predicate tests | 17 passed |
| `cargo test --manifest-path rust/Cargo.toml --release --bin strict_nest_check` | 7 passed |
| `cargo test --manifest-path rust/Cargo.toml --release -- --test-threads=1` | 全量通过；主库 1545 passed、127 ignored，所有 bin/integration/doc tests 无失败 |

首次默认并行执行 release 全量测试时，唯一失败为既有测试 `opsem_dump_env_gated_bit_exact`；单测立即复跑通过。根因是该测试修改进程全局 `OPSEM_DUMP_DIR`，并行的其他 runner 测试也会读取同一环境变量并创建同一临时输出，从而可能截断 `trades.jsonl`。本次 D_parent 改动未触及该生产路径；为避免把基线测试的进程全局环境竞态误判为功能回归，随后以 `--test-threads=1` 完成 release 全量确定性验证并全部通过。本任务未扩大范围修改该既有测试架构。

## 7. 产物

- 逐 bit、sanity、证书总判：`STRICT-NEST-DPARENT-CHECK.md`
- 原始漏斗、延迟分布与近失样本：`chanlun/review-results/dparent-funnel-20260710.md`
- 本报告：`chanlun/review-results/p0-replay-dparent-20260710.md`

最终判定：**PASS**。实现符合冻结语义；L0 to L1 回归预期达到，L1 to L2 的 0/0 已按甲/乙假设完成边界内归因；相关测试及 release 串行全量测试通过。
