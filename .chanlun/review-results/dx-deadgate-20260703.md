# Task #137 dx-deadgate-update 结果包（简化三要素）

**工位**：ws-dxgate | **日期**：2026-07-03 | **引用**：codex-t1 裁定A（#121）、#123（0a35f0167c）、G4 #134 移交（99bab5ad68）

## 1. 结论

### 1.1 dx 死门诚实重封（econ_positive.rs）

原死门 `assert_eq!(xzd_lge2_sub_bsp_type3_total, 0)` 的前提「level≥1 只产 B2/S2
（extract_second_for_level）⟹ lvl>=2 的 sub_bsp 恒无 Type3」已被 codex-t1 裁定A + #123
（级别≥1 一/三类候选生成实装，全历史三类净增 2364）合法作废。重封形式 = **锁默认窗确定性
基线数值**（非删除、非放宽为 >=0）：

```rust
if max_bars == MAX_BARS_DEFAULT {
    assert_eq!((n_lge2_routed, xzd_lge2_sub_bsp_type3_total), (75, 3150), ...);
}
```

### 1.2 dx 新基线数值表（BTC 冻结数据全量 4,613,599 bar）

| 计数 | 默认 300K 窗（2025-11-04→2026-05-31，**锁定**） | 全历史（2017-08-17→2026-05-31，参考不锁） |
|---|---|---|
| lvl>=2 routed（Xzd 路由） | **75** | 1075 |
| lvl>=2 sub_bsp Type3 点总数 | **3150** | 721818 |
| 信号总数（signals_dx） | 774 | 11939 |

默认 300K 窗 Xzd 路由分级明细：level1=165 / level2=56 / level3=14 / level4=4 / level5=1；
C2 成立=68/22/4/2/1；旧 C3（same_side_same_center）全级恒 0。

全历史门前三类分布（#123 生效直接可见）：level1=1792 / level2=531 / level3=155 / level4=46 / level5=13。

验证：默认窗两跑 bit-exact 复现（75/3150）；全历史跑走「非默认窗跳过基线断言」分支，
诚实报告参考值；`cargo test --release --lib` 1436 passed / 0 failed。

### 1.3 #115 fill-rate 断言实测数值（β^div 热路由 L2 首证回报）

| 窗口 | 信号总数 | 一类(buy1\|sell1) | force_state=Some | 填充率 |
|---|---|---|---|---|
| 默认 300K | 774 | **0** | 0 | 0%（断言空泛通过） |
| 全历史 4.61M | 11939 | **0** | 0 | 0%（断言空泛通过） |

**如实回报：fill-rate 断言在两窗均为空泛通过（vacuous pass）**——`if type1_sigs > 0` 门控
未触发，一类信号在门前（bsp_pre 第一类全级=0）即为 0，与 #116（一类零触达=市场事实）一致。
#123 提到的「牛市窗 L4 产 2 顶背驰卖点」是切窗普查产物；全历史因果重放中级别链不同
（级别截断窗口依赖），不出现该 2 点——两者并存不矛盾。**β^div 热路由的 L2 非空证据仍缺位**，
断言真正咬合需存在一类信号的窗口/品种（结构保障：一旦一类出现，force_state 全 None 即 fire）。

### 1.4 τ^reverse 拷贝处置（G4 #134 移交）——选降级标注（选项 B）

econ_positive.rs 的 `collect_signals`+`pair_signals`（入场=全部新确认 bsp 信号，出场=下一反向
新确认信号）即 τ^reverse 口径拷贝。处置：**显式降级标注 `legacy_reverse_exit_diagnostic`**
（模块头 + `decompose_capturable_spread` + `collect_signals` + `pair_signals` 四处），并移除全部
过期的「同 `build_walk_forward_mu`」同源声明。不选重接 typed ledger 的理由：
1. econ-663/664 谱系历史结论（可捕获价差分解、逐信号 μ̂）的可比性依赖旧口径；
2. dx harness 的诊断对象是**信号集本身**（门前后计数/配对丢失/bit-exact 对拍以 collect_signals
   为锚），不是 fill 腿——重接会使诊断失去对象；
3. 生产 μ 管线已在 G4（99bab5ad68）走 typed ledger，本处仅诊断域，两口径 μ̂ 不可互比已在
   模块头声明（样本量差约两个数量级）。

### 1.5 XZD C3 高级别 Type3 消费重新评估（任务②）

已确认登记在 #135 描述的重跑清单内（「XZD C3 高级别 Type3 消费重新评估」），无需重复写入。
本次 dx 数据供 #135 消费：旧 C3（same_side_same_center）在 #123 后**仍全级恒 0**（默认窗
routed=240 中 0 命中）——sub_bsp 有 Type3 点（3150 个）但无一个的 center 归属命中 last_zs，
center 对象身份分裂（678 号）问题在高级别同样存在。

## 2. 边界条件

- **基线是窗口函数**：锁定值 (75, 3150) 仅对默认 300K 窗 + BTC 冻结数据（全量 4,613,599 bar）
  定义；数据更新或窗口变更 ⟹ 断言跳过（非默认窗）或 fire（默认窗数据漂移），须重测重锁。
- 上游 BSP 生产链（extract/hl13 级别-N 判定）或 Xzd 路由逻辑任何变更 ⟹ 基线 fire ⟹ 重测
  重锁并审计来源（死门语义保留：防静默漂移）。
- 基线对 #138 G3 z 扩维稳健：死门计数不消费 z 字段；parity 对拍两侧同码自洽。
- fill-rate 结论翻转条件：任何窗口/品种出现一类信号（type1_sigs>0）即脱离空泛态。

## 3. 影响声明

- **改动文件**：仅 `rust/src/theta_v0/backtest/econ_positive.rs`——死门断言重封（唯一行为变更，
  且在 #[ignore] 诊断测试内）+ 注释层 τ^reverse 降级标注与过期前提修订（H3 判别文案、
  XzdEvidence.sub_bsp_type3_count 字段文档、报告文案）。不改任何生产逻辑/门判定/信号收集行为。
- **报告工件**：`.chanlun/review-results/acc-classification-level-hole-20260701.md` 由 dx 重跑
  刷新为 #123 后真值（此前工作树中该文件是 #123 前旧 300K 跑批的未提交残留，已被权威重跑覆盖）。
- **并发协调**：#138 G3 在飞改动（selector.rs ZExt 5 参、econ_positive.rs 三个 hunk：import/
  ZExt 装配/NestTrigger pub+Hash/Ord）**未打包进本 commit**（git apply --cached 逐 hunk 过滤，
  commit 前 --cached 全查）。G3 落地时需同步更新 dx 手写循环 :3665 的
  `z_of_candidate_with_force` 调用点（当前主工作树因此暂不可编译，属 G3 在飞态，非本 commit 引入）。
- 验证方式：因主工作树被在飞改动污染，最终提交树在隔离 worktree（HEAD+仅本工位 staged 内容）
  完成 lib 全绿 + dx 默认窗全绿复验。
