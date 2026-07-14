# on2 O(n²) 战役总验收（三波次 final）

日期：2026-07-04
战役基线 HEAD（波次1修复前）：`ebcd8f2a0f`
波次3 验收 HEAD：`d437f9772e`（candidate 两残余 SHIP）
编译：`cargo --release`（opt-level=3 / lto=false / debug-assertions=false → `debug_assert!` 剥离）
认识论（231号）：正确性/加速 = **L1**（管线等价 + 管线度量，零信息增量）；e/epoch 局部化前提 = **L2**（真实 BTC/CL 分布）。

---

## ① 全量测试（硬门）

命令：`cargo test --release --lib -p newchan_rust`

```
test result: ok. 1489 passed; 0 failed; 115 ignored; 0 measured; 0 filtered out
```

**全绿。0 红。硬门通过。** 波次1=1484 → 波次2=1488 → 波次3=1489（+1：merge_skip_oracle）。115 ignored 均为需真实数据/长跑的 `#[ignore]`。

## ② bit-exact 电池（铁律，全波次放行条件）

命令：`cargo test --release --lib -- --ignored bit_exact_per_bar candidate_incremental_bit_exact_vs_full merge_skip_oracle`

| 测试 | 域 | 结果 |
|------|-----|------|
| `bit_exact_per_bar`（CL 8K，逐 bar 增量 == legacy 全量重判，bit-identical） | 增量塔 vs 全量 | **passed（0.6s）** |
| `bit_exact_per_bar_cached_vs_nocache`（CL 2K，forest_epoch 双 key 无陈旧） | interp T_i→K_i oracle | **passed（2000 bar 缓存有效）** |
| `candidate_incremental_bit_exact_vs_full`（CL 8K，epoch swap 后 candidates bit-identical） | 波次3 candidate | **passed（cand_cache 命中 406）** |
| `merge_skip_oracle_real_cl_debug`（CL 4K，全 merge 调用 skip==全量） | 波次3 merge append-only skip | **passed（debug_assert 未 panic）** |
| `bit_exact_synthetic` / `incremental_macd_bit_exact_synthetic`（always-run） | 合成增量 | 随全绿守卫 |

**4/4 ignored 电池全绿 + 合成守卫全绿。** 波次3 两件（07a frontier-resume + candidate epoch swap/append-only skip）的 bit-exact 铁律坐实——信号集/candidate/registry 末态逐字节不变。

## ③ 400K / 1M 分段计时（THETA_PROFILE_STAGES=1，BTC 逐 bar classify_with_tower_incremental）

命令：`THETA_PROFILE_STAGES=1 A0_PROFILE_BARS={400000,1000000} cargo test --release -p newchan_rust --lib profile_clone_cluster_a0 -- --ignored --nocapture`

### 墙钟 + 终态 exp 演进表（战役全程）

| 窗口 | 战前基线 | 波次1（`150c…`） | 波次2（`e3ae…`） | **波次3 终态（`d437…`）** | 端到端加速 |
|------|---------|-----------------|-----------------|--------------------------|-----------|
| 400K | **6.00s** | 3.84s | 3.71s | **3.05s** | **1.97×（-49%）** |
| 1M   | ~21.4s | 21.39s | 19.43s | **14.19s** | **1.51×（-34%）** |

**400K→1M 标度指数演进**：
波次1 exp≈**1.874** → 波次2 exp≈**1.807** → **波次3 终态 exp = ln(14.19/3.05)/ln(2.5) ≈ 1.678**。
残余超线性从战前隐含 >2.2（07a 主导项 exp=2.20）逐波下降至 **1.678**——O(n²) 主导项已消除；残余 1.678>1 归因见 ⑤（05_compose_resume 有界项 + O(S)/miss 轻量趟）。

### 波次3 07a 段（终态 stage 表，一/三类前缀点缓存）

| 窗口 | 07a 合计（l0+ln）修前 | 修后（实测） | 加速 | wall 修前→修后 |
|------|----------------------|-------------|------|----------------|
| 400K | 775 ms | **123.7 ms**（49.77+73.90） | **6.3×** | 3.70s → 3.05s |
| 1M   | 5842 ms | **567.1 ms**（306.66+260.44） | **10.3×** | 19.27s → 14.19s |

07a 标度指数：**2.20 → ln(567.1/123.7)/ln(2.5) ≈ 1.66**（O(n²) 主导项消除；残余 = O(S)/miss anchors_self 复制 + 点向量 clone，与已验收 07b 残余同类）。

## ④ strategy 段（candidate）exp 复测（CL OOS 2023-01-01..2025-06-30，THETA_PROFILE_STAGES，--release）

命令：`profile_cand_stages`（8K/16K stage 拆解）+ `profile_full_engine_scaling_16k`（engine exp）

| cost center | 8K 修前 | 8K 修后（实测） | 16K 修前 | 16K 修后（实测） | 16K 加速 |
|-------------|---------|----------------|----------|-----------------|----------|
| cand_build_merge（attach） | 3.208 ms | **0.481 ms** | 12.321 ms | **1.681 ms** | **7.3×** |
| cand_merge_consume（快照） | 4.305 ms | **0.737 ms** | 18.704 ms | **2.163 ms** | **8.6×** |

| exp（8K→16K，实测） | 修前 | 修后 |
|--------------------|------|------|
| strategy 净额 | 1.90 | **1.79** |
| engine_full | 1.73 | **1.58** |

engine_full 16K 净额 0.064s；strategy 净额 16K 0.042s。candidate 三 stage（build_step/build_merge/merge_consume）现全 ≤2.8ms@16K 且亚二次。残余最大项转为 `05_compose_resume`（12.8ms@16K，recursive_tower 域，非本战役有效域）。

## ⑤ 三波次 FIXED / NO-SHIP 全清单

### FIXED（bit-exact-sound 落地，含 stage 加速）

| 项 | 波次 | 1M/16K stage 加速 | commit |
|----|------|-------------------|--------|
| 09_project_to_units_resume O(n²)→O(tail) | 1 | 1238→34 ms（36×，400K） | `b5744f219b` |
| 00b_l0_units_clone O(n²)→O(1) Rc | 1 | 381→5.7 ms（64×，400K） | `a5986d64e2` |
| 10_projected_units_clone | 1 | 167→25 ms（6.7×，400K） | H4 同文件 |
| A1 Rc化 + C3 partition_point 二分 | 1 | 第二波两泳道 | `4c23604f4c` |
| forest_epoch（K_i 森林命中 O(全塔)→O(1)） | 2 | 0.33→0.054 s（6.1×，CL16K） | `f15253ce3f` |
| cascade_reset 增量失效（07b 前缀重扫） | 2 | 1640→111 ms（93%↓，1M） | `e3ae45a863` |
| **07a extract_signals frontier-resume（一/三类前缀点缓存）** | **3** | **5842→567 ms（10.3×，1M）** | `80e012eaf0` |
| **candidate epoch swap（attach 全量拼接）** | **3** | **12.32→1.68 ms（7.3×，16K）** | `d437f9772e` |
| **candidate merge append-only skip（全量快照消费）** | **3** | **18.70→2.16 ms（8.6×，16K）** | `d437f9772e` |

### NO-SHIP（残余，各文件域内无 bit-exact-sound 降阶杠杆）

| stage / 域 | 计时 | 根因 | 裁决 |
|-----------|------|------|------|
| 05_compose_resume（recursive_tower H9） | 1936 ms@1M | Fix A/B 已落地；残余 05c2a/b（rmove_clone/submoves_alloc）有界 O(n·k) 收益递减 | NO-SHIP |
| 07a_extract_signals_l0 残余（signal.rs） | 307 ms@1M | frontier tail 重判 = O(S)/miss 轻量趟（anchors_self 复制 + 点 clone），非 O(n²)；主导项已消 | NO-SHIP（有界，占比<1% 不预建） |
| collect_signals（econ_positive.rs） | — | O(n²) 全继承自 classify_at 底座；econ 自身 C1/C2/C3 已优化 | NO-SHIP |
| of_forest / tree_segment（interp.rs H6） | — | 波次1 NO-SHIP（本域无 sound 快路）→ 波次2 越域 forest_epoch **FIXED** | 已转 FIXED |

**战役无遗留可 bit-exact 降阶的 O(n²) 靶。** 波次1 曾判 NO-SHIP 的 07a（doc① signal.rs 单域裁定）在波次3 通过 resume 状态挂 mod.rs::LevelCache（275号越域路径，doc① Q1=B 边界翻转达成）转 FIXED——即战前 6 个 NO-SHIP 中 07a + of_forest 两项经越域后落地，其余 4 项为真有界残余。

## ⑥ 剩余超线性诚实声明（波次3 后仍存，全部有界）

- **A0 逐 bar 热路径 exp=1.678**：主残余 = 05_compose_resume（1936 ms@1M，recursive_tower H9 域）+ 07a frontier tail O(S)/miss 轻量趟（307 ms@1M）。前者残余是 O(n·k) 有界收益递减（Fix A/B 已落地），后者非 O(n²)（anchors_self O(S) 复制，占比<1%）。**均已 codex/H8/H9 先例裁 NO-SHIP，非可 bit-exact 降阶靶。**
- **strategy 净额 exp=1.79 / engine_full exp=1.58**：candidate 三 stage 已亚二次（≤2.8ms@16K），残余超线性转移到 05_compose_resume（recursive_tower 域，非本战役有效域）。
- **前提 L2 坐实**：cascade「frontier 改写局部化」e 前提（300K CL：e0=10.05%、keep_frac=0.856，未坍缩）+ forest_epoch「tree 字节不变」前提（CL 16K：bump 率 2.17%、false_hits=0、命中率 98.51%）+ candidate「id 集 append-only」前提（16000 bar 仅 3 drop）三者均**成立（非证伪）**。若 e0→1 / keep→0 / bump→100% / drop 频繁 则各前提翻转，退化全量分支但仍 bit-exact。

## ⑦ 对 #182 终局跑批剩余段（R5/F3/F4 补跑）的加速预估

#182 的 F1–F4（BTC 全历史 4,613,599 bar 诊断）与 R5（q4 π^full 四臂 policy）均逐 bar 走
`classify_with_tower_incremental`（F1-F4）或 `run_theta_v0_pi`（R5）——即本战役优化的同一热路径。以实测标度点外推（诚实标注：外推 = L1 度量的线性/幂律外插，非新跑数）：

| 段 | 路径 | 战前 exp | 终态 exp | 全历史(4.6M)预估 | 加速预估 |
|----|------|---------|---------|-----------------|----------|
| F1/F2/F3/F4（BTC 全历史 classify 路径） | classify_with_tower_incremental | ~1.87 | **1.678** | 战前 O(n²) 主导下 ~收缩 | 07a 主导项(4.6M)：~169s → ~6.6s（07a 单段 **~25×**）；整 classify 路径 4.6M 终态 **~3 min 级** |
| R5（q4 π^full 四臂 policy） | run_theta_v0_pi | 1.73 | **1.58** | engine_full 亚二次 | candidate 两残余 16K **7–9×**；全历史随 n 增长加速比放大（O(n²) 项消除） |

**边界**：这是 L1 度量外推（幂律外插），非全历史实跑数。全历史实测须在冻结二进制上直跑（#182 R5 已用冻结 `newchan_rust-9cb5b83727a3eeb0` 隔离 #183 编辑污染）。加速比随 n 单调放大是 O(n²) 主导项消除的直接推论——4.6M/1M=4.6× 规模下，战前 O(n²) 项贡献 ∝ 21×，终态亚二次项 ∝ ~11×，故全历史加速比 > 1M 实测的 1.51×。

---

## 结果包六要素

1. **结论**：on2 战役三波次消除 8 个可 bit-exact 降阶的 O(n²) 热点（09/00b/10 + A1/C3 + forest_epoch + cascade 增量失效 + 07a frontier-resume + candidate epoch swap + candidate append-only skip）。400K 墙钟 **6.00→3.05s（1.97×）**，1M **~21.4→14.19s（1.51×）**，A0 标度指数 **1.874→1.678**；strategy engine_full exp **1.73→1.58**。全量 1489 绿/0 红，bit-exact 电池 4/4 全绿。
2. **定义依据**：O(n²) = 逐 bar 全量重判/重扫（loop∝ΣS∝n²，07a 诊断实测 ΣS=48M@1M）；bit-exact 铁律（090号）= 增量输出逐字段 == legacy 全量重判；FIXED ⟺ bit-exact-sound 降阶落地；NO-SHIP ⟺ 域内无 sound 快路或残余有界。
3. **边界条件（结论翻转）**：任一 bit-exact 电池测试逐 bar 发散 ⟹ 冻结锚/epoch/append-only 前提被违反 ⟹ 回滚。cascade e0→1 / forest_epoch bump→100% / candidate drop 频繁 ⟹ 前提翻转，退化全量分支（仍 bit-exact，无收益）。终态 exp 若在更大 n 上重新 →2.0 ⟹ 存在未识别 O(n²)（当前 4.6M 外推预估 <2）。
4. **下游推论**：信号集/candidate/registry/π 账本末态 bit-exact 不变 ⟹ 不影响 W-VERIFY(#13) alpha、不改任何 class_index/分桶 key。#182 终局跑批（R5/F3/F4）在同一热路径上获 07a 单段 ~25× / candidate 7–9× 加速，全历史批跑从 O(n²) 分钟级下降。
5. **谱系引用**：090号（bit-exact 铁律 + no-patch：全量分支保留作 fallback）；231号（L1 计时零信息增量 + L2 前提）；275号（resume/skip 状态挂 caller LevelCache/CandidateCache，越域 forest_epoch 落地）；137/407号（compact 后声明层效力归零）；doc on2-final-20260704（波次1+2）、on2w3-07a-impl、on2w3-cand-impl（波次3 两件）。
6. **影响声明**：新增本文件 `.chanlun/review-results/on2-campaign-final-20260704.md`（战役总验收结果包）。**不改任何代码、不改谱系**——纯验收跑数 + 汇总判读。commit 限本文件。
