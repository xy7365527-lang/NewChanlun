# #578 合成夹具坐标约定对齐生产共端点——交付报告

> pre = HEAD `04c360a4101bc615f42c60ab5cc2a07df5fc8f76` 的 `git archive` 净出；
> post = 本工位（唯一差异 = `rust/src/theta_v0/classifier/nest_lifecycle.rs`）。
> 产物前缀 `/tmp/wt578-*`。

## 1. 结论

1. **盘点**：`nest_lifecycle.rs` 测试族中，唯一使用「段间 +1 不共端点」坐标约定的合成夹具是
   `pan_real_fixture`（`nest_lifecycle.rs:2581`）。其 5 段序列 `(50,59)/(60,69)/(70,79)/
   (80,89)/(90,99)` 段间存在 1 bar 空隙（`end_index+1==next.start_index`），与生产
   `end_stroke=k-1`/`seg_start=k` 共端点约定（`end_index==next.start_index`）不一致。
   `p123_fast_replay.rs` 无独立合成夹具（其固定输入是真实 BTC 数据，不适用本票范围）。
   `T2/T6/T6b/T7/T8/T15/T16/T17/T18/T19/P1` 等测试使用独立自建 `hist`/`dif`/`pan_window`，
   与 `pan_real_fixture` 无耦合（不共享 `Segment` 结构），不在本票范围内。
2. **对齐**：`pan_real_fixture` 的 5 段改为共端点承接——`(50,59)/(59,69)/(69,79)/(79,89)/
   (89,99)`（保留 seg_a=(50,59) 不变，c_start 由 70 → 69，末段起点 90 → 89）；
   随之更新 T11/T14/F1–F10/P2–P5/F9 共 ~20 个测试内的 `(70,X)` 派生字面量、P2/P3 的
   `ActiveSegmentFrontier.start_index`（90→89）、`identity_close_src` 截断长度（70→69，
   对应 `map_src_to_close_idx` 判失败的边界值）。全部改动列于本次 commit diff。
3. **守卫可测性——尝试收紧后因真实数据证伪而撤回**：夹具对齐后 `provide_l1_active_pan_live_windows`
   的 `<` 守卫（`nest_lifecycle.rs:1503`）**首次**具备条件真做「改 `==`」的探针（此前该分支
   在旧夹具上恒拒，是死分支）。改 `<`→`!=` 后单测全绿，但用 p123 20k bar 真实 BTC 数据对拍
   发现：`frontier_not_after_confirmed` 从 0 跳到 **210**，`window` 命中同时从 101 降到 58——
   即 `active.start_index > last.end_index`（有洞、非倒灌）在真实数据里**频繁发生**，与
   #559 条件 C2 的「`>` 分支生产不可达」断言矛盾。该断言此前**从未被真做 `!=`/`>` 判别的探针
   实测过**（旧夹具令 `==` 守卫恒拒，无法在不改夹具前提下把 `>` 分支接上真实数据）——是未经
   验证的声明膨胀（090 号语法禁令）。**故不收紧守卫**：保留原 `<` 判据（生产行为不变），
   在守卫注释中照实记录此发现与证伪过程，并在 P2 测试新增一条「已知缺口」用例
   （`start_index = confirmed.last().end_index + 1`）钉住现状（有洞 frontier 当下被接受产窗，
   非本票裁定的正确性）。是否应收紧为拒绝需教义/架构裁决（有洞成因是否合法、该拒绝还是
   该扩展定界窗口），**不在本票 Scope**，登记为遗留供后续 ticket 跟进。

## 2. 定义依据

- 生产共端点约定：`parser/segment.rs:415-425`（`end_stroke=k-1`/`seg_start=k`，笔首尾相接）；
  `classifier/mod.rs` `moves_tower_l0` 由 `l0.segments[..]` 纯函数生成，`tower[0]` 含未确认末段。
- `map_src_to_close_idx`（`recursive_tower.rs:986`）：`close_src` 截断长度 == `start` 时
  `lo==len` 触发 `None`（`CoordinateMapFailed`）——T14 的 `identity_close_src(70)`→`(69)`
  更新即依此边界值推导，非任意改动。
- `locate_pan_div_structure`/`departure_move_c_start`/`episode_start_in`
  （`signal.rs:655` / `divergence.rs:841,814`）：`λ_C` 取窗口内首个方向匹配段的
  `start_index`，故 c_start 随 `pan_real_fixture` 段2的 `start_index` 变动（70→69）而变动，
  非独立字面量——对齐时漏改此值会导致真实产窗结果与测试断言不一致（本票交付过程中
  确实先漏改一处，被 `cargo test` 现场抓出，已订正）。

## 3. 边界条件

- 若未来任何调用方假设 `pan_real_fixture` 的段间存在 1-bar 间隙（本次盘点未发现此类假设），
  本次对齐会使其失效——但该假设本身即与生产约定矛盾，理应失效。
- 有洞 frontier（`>` 臂）当下产窗行为（accept）是**现状**而非**裁定**；若后续教义裁决要求
  收紧为拒绝，需重新评估 210/258（20k bar 窗口内）个受影响身份的下游影响，不能直接照抄
  本票的 `!=` 尝试补丁（本票已证该补丁非零行为变化）。

## 4. 下游推论

- 夹具与生产坐标约定一致后，未来任何在 `pan_real_fixture` 上补测「衔接守卫」「λ_C 边界」
  等生产不变量的测试不再是死分支——`P2` 新增的两条用例（倒灌负控 + 有洞现状钉住）即是首批
  受益者。
- `frontier_not_after_confirmed` 现状（0 计数）与「有洞」现状（210/463 outcomes，本票发现）
  的落差，提示 `L1LiveOutcome` 诊断口径可能需要拆分「倒灌」与「有洞」两类原因码（现均归
  同一 `FrontierNotAfterConfirmed` 枚举值，但语义/成因不同）——留供后续 ticket 判断是否需要
  仿照 #559 的 `VanishCause` 两类拆分先例处理。

## 5. 谱系引用

- `.chanlun/genealogy` 未见与本票直接相关的既有条目；`formalization-validity-domain` 规则
  （有效域 ≠ 定义域）适用于本票发现——#559 条件 C2 的「`>` 分支生产不可达」是 L0（纯代数
  推理：parser 不变量 ⟹ 恒等式）声明，但从未有 L2+（真实数据）验证支撑，本票首次给出 L2
  验证并证伪其外推到「守卫可收紧」这一推论。

## 6. 影响声明

- 改动 1 个文件：`rust/src/theta_v0/classifier/nest_lifecycle.rs`（仅 `#[cfg(test)]` 测试
  夹具与断言 + 1 处生产代码注释更新；`provide_l1_active_pan_live_windows` 的判据逻辑
  **未变**，`<` 保持不变）。
- 测试门：`cargo test --lib theta_v0::classifier::nest_lifecycle` 36/36 全绿；
  全库 `cargo test --lib` 2030 passed / 1 failed（唯一红 = 已知票 #491）/ 136 ignored，
  与改动前基线逐位一致。
- 生产零行为变化护栏（全部 cmp=0）：

| 护栏 | 窗口 | cmp | SHA-256（pre=post） |
|---|---|---|---|
| p123 stdout | 20k | **0** | `bd9ac1d655f9d615a5d9b3495b3a92465fb1fae13ce5dadfa28033c47d375b6` |
| P116 dump | 20k | **0** | `fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a4` |
| P421 lifecycle dump | 20k | **0** | （字节 diff 为空；未另算 sha） |
| m8 trades | p3fold | **0** | `2da686833581d5358a1806aa41ad7ba16371bc3db190fe2a8ccfe62b9cb4662` |
| m8 tower_events | p3fold | **0** | `83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c` |

  （首三项/末两项哈希分别与 #527 交付报告登记值一致，交叉确认无漂移。）
- outcome tally（`P527_L1_LIVE_OUTCOMES`，20k）pre/post 逐位相同：
  `center_not_consolidation=67 no_active_frontier=57 structure_not_locatable=155 window=101`。

## 7. 复现命令

```bash
cd rust
CARGO_TARGET_DIR=/tmp/kimi-nest-target-578 cargo test --lib theta_v0::classifier::nest_lifecycle
CARGO_TARGET_DIR=/tmp/kimi-nest-target-578 cargo test --lib

# pre/post 隔离对拍（pre = HEAD git archive 净出，post = 仅打本票 nest_lifecycle.rs 单文件 diff）
DATA=/Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json
for tag in pre post; do
  P116_MAX_BARS=20000 P116_CKPT=0 \
    P116_DUMP=/tmp/wt578-$tag-20k.p116 \
    P421_LIFECYCLE_DUMP=/tmp/wt578-$tag-20k.dump \
    /tmp/wt578-target-$tag/release/p123_fast_replay "$DATA" \
    > /tmp/wt578-$tag-20k.stdout 2> /tmp/wt578-$tag-20k.stderr
  M8_WIN_FILTER=p3fold OPSEM_DUMP_DIR=/tmp/wt578-$tag-m8-p3fold \
    CARGO_TARGET_DIR=/tmp/wt578-target-$tag \
    cargo test --release --lib m8_e2e_all_systems_oos -- --ignored --nocapture
done
```

产物：`/tmp/wt578-{pre,post}-20k.{stdout,p116,dump,stderr}`、`/tmp/wt578-{pre,post}-m8-p3fold/`、
`/tmp/wt578-nest_lifecycle.diff`。

## 8. 遗留（不在本票 Scope，照实登记）

1. **有洞 frontier 的教义/架构裁决**：`active.start_index > last.end_index`（有洞、非倒灌）
   在真实 BTC 数据 20k bar 窗口内出现 210 次（约占该函数总 outcome 的 45%）。当下行为 =
   接受产窗（`<` 守卫不拦截）。是否应改为拒绝、或应扩展 λ_C 定界窗口以正确跨越该洞，需要
   教义（这类「有洞」在缠论线段/中枢结构上对应什么现象）与架构（λ_C 计算是否需要感知洞）
   两层裁决，需要新 ticket。
2. **`L1LiveOutcome::FrontierNotAfterConfirmed` 语义拆分**：现单一枚举值同时覆盖「倒灌」
   （`<`，20k 窗口内 0 次）与「有洞」（`>`，210 次）两种不同成因；若遗留 1 判定为需要
   收紧，届时应参照 #559 `VanishCause` 先例拆两类原因码，而非继续合并。
3. 本票未改动 `p123_fast_replay.rs`（无需——该文件只消费 `provide_l1_active_pan_live_windows`
   的判据，判据本身未变）。

---

## commit

`test(theta): #578 合成夹具坐标约定对齐生产共端点`
