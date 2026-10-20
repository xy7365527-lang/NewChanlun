**VERDICT: PASS**

本轮只核作者对第 1 轮必改 1、3、4 的落实，以及新增 TestOnly 臂 `OPEN_SKIP_ARM`；三项均已落实，TestOnly 臂不进生产；无新增必改。

## 必改落实核验

**必改 1（OPEN 封段对照见证）— 已落实。** `rust/src/theta_v0/parser/tb02w_tests.rs:1215` `tb1493_open_second_candidate_blocks_later_candidates`：59 笔链上末段第二种候选 b=s42 第二序列 `semantic_state == Some(OpenAtPrefixEnd)`，候选之后标准元素 8 个 > `TAIL_WINDOW=7`（断言 `after > TAIL_WINDOW`），新规则 6 段+pending；旧规则臂同输入 7 段且前 6 段逐一相等（见证非空转）；逐前缀 0..=59 全量≡增量、已发射段单调。这正是第 1 轮要求的构造性测试。夹具常量 `OPEN_BLOCK_PRICES`（:1203）与远端搜索日志 `20261020-115110-e03730534c.log` 第 2 行 seed=1324 的 60 个价格逐值一致（我做了 diff，仅文件末换行差异）。搜索测试 `tb1493_search_open_block_witness` 为 `#[ignore]`，不进常规计数。

**必改 3（`17c7e24e08` 收据）— 已落实。** 本地 `b/evidence/20261020-114349-17c7e24e08.log`：sha 对上，FMT_RC=0，`cargo test --lib` 2864 过 0 败 149 ignored，release ignored 回放 3 过。HEAD `afd7eadf37` 收据 `20261020-115550-afd7eadf37.log`：FMT_RC=0，2865 过 0 败 150 ignored（+1 过 = 新见证测试，+1 ignored = 搜索测试，数目吻合），ignored 回放 6 过。

**必改 4（§15 行号）— 已落实。** X-33～X-35 全部行号逐条打印核对，均指到 HEAD 对应代码：feature_seq.rs `:432-455`（cfg(test) thread_local 块）、`:514-519`（`unresolved_blocked` 文档）、`:695`、`:707-725`、`:859-864`、`:995-1001`、`:1002-1011`；segment.rs `:1145-1148`、`:1319/:1336/:1349`、`:1811-1850`；tb02w_tests.rs `:1007`、`:1215`；tb1493_replay_tests.rs `:34/:68/:96/:173`、`:251/:282/:425`（三处均为「非末笔可改写的改写 = 0」断言）、`:480/:521/:559`；tb02c_tests.rs `:678/:710/:756/:802/:914/:939`。§7.1 第 1 条 `065:28` 引文「唯一的要求就是被分析的K线已经走出来」在 `docs/chanlun/text/blog/065-第65课.md:28` 原文命中。

**必改 2（三方对照与手算写票）— 已出现。** #1493 最新评论（owner_1493b 交付报告）含随机 802 组差异 [3,262,263]、段端≠P 0/0/204、真实 51,741/51,417/51,825、3 组手算（rand0/true #8、rand0/false #26、rand1/true #35），与第 1 轮我独立手算的 #1、#3 一致；「段端≠段内极值」33/32/226 读数与口径解读也已如建议写明。

## `OPEN_SKIP_ARM` 核验

- 声明在 `feature_seq.rs:431` `#[cfg(test)] thread_local! { … }` 块内（:442），与 `CASE_ONE_ORACLE_067`、`GAP_INCLUSION_ARM` 同块，非测试构建下不存在该符号。
- match 臂 `:995-1001` 自带 `#[cfg(test)]`，且臂体 `skipped_secondkind = true; continue;` 与基线 `36adc7c7c8` 的旧 OPEN 臂（基线 :962-967）逐字一致，忠实复现旧规则。
- 非测试构建删去该臂后，紧随的 `OpenAtPrefixEnd | None` 臂（:1002）仍覆盖该变体，match 穷尽性与分派顺序不变；测试构建下默认 `false`，守卫不命中即落入同一生产臂，行为等同生产。
- 仅在 tb02w_tests.rs 两处设 true 并立即复位（:1236-1238、:1291-1293），复位在断言之前，无跨测试泄漏；每个 libtest 测试独占线程，thread_local 不串。

## 观察（非阻断）

- §15 X-33 把 segment.rs `:1336/:1349` 描述为「与续扫路径同一过滤」，实际这两处是字面 `None`（注释 #804：无确认段、无可扫种子）。语义等价（续扫起点为 0 时过滤结果必为 None），措辞略宽，可不改。
- PR #1582 CI（run 39366853562，HEAD `afd7eadf37`）：test / fixture-drift / s-observer 已绿，**rust-check 在我核对时仍在运行**，合入前需确认转绿。

## 检查读数

- `git diff 17c7e24e08..HEAD`：3 文件，+135/−3（xianduan.md 2 处、feature_seq.rs +10、tb02w_tests.rs +122）；无生产逻辑改动。
- 远端收据：`17c7e24e08` 2864/0/149、FMT 0；`afd7eadf37` 2865/0/150、FMT 0、ignored 回放 6 过。
- 本机未跑 cargo，未改任何文件，未输出任何令牌。
