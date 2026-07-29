# #602 探针可复算物（票 #629 P2 条件结账，诊断用，非生产）

> 角色：executor（#629 条件结账 P2 项，前台单线程、禁子代理）
> 依据：`shadow-602-review-20260728.md` P2（探针读数不可第三方复算）；交付报告
> `issue602-panlive-l3-provider-20260728.md` §3（探针方法与读数）
> 口径：本文件不改教义、不关票、不改生产代码；唯一目的是把 §3 的读数变为**可第三方复算**。
> **探针代码本身不进 commit**（与原纪律一致），仅本文件登记 patch 与读数。

## 0. 结论

探针在**当前 HEAD**（`16cd83db4c`）的隔离副本上重建并重跑两轮。**5/6 类读数逐值复现**：
重扫次数（970/970）、两轮 nwin 分布（逐值相同）、末窗吸收虚拟单元（638/970、363/970）、
`lower_frontier_not_after_units`（728→0）、`l3:window`（37→72）、「批内非末窗 0/8」join。

**1 类读数不一致，照实上报**：轮次 2 的 `Σ(m−1)` 报告声明 **91**，本次复算按报告同一定义
（「只取末窗丢掉的窗口实例」= 对每次重扫的 `nwin−1` 求和）得 **236**，且该公式在**轮次 1**
上逐值精确复现报告声明的 **480**（同一公式、同一探针、同一数据，轮次 1 命中轮次 2 不命中）。
另试两种替代解释（batch 数 `nwin≥2` 计数 = 71；跨 bar 去重后的不同坐标窗口数 = 9），均不等于
91。判定：轮次 2 的 91 这一数字**目前找不到能同时满足「轮次 1 用同公式验证为真」且「轮次 2
算出 91」的单一口径**——原探针脚本已删除且未附 patch，本评审只能确认**不一致**，无法确认何种
口径产出了 91（这正是 shadow-602-review P2 指出的不可复算性的具体实例）。

## 1. 探针实装（本文件唯一权威 patch，未进 commit）

隔离副本：`/tmp/wt629-probe`（`git archive HEAD` 导出 `16cd83db4c` + 全量 `touch` 避重放缓存）。

### 1.1 `rust/src/theta_v0/classifier/nest_lifecycle.rs`：`scan_active_window` 加 `probe_tag` 参数

```diff
 fn scan_active_window(
     units: &[UnitRange],
     virtual_unit: UnitRange,
     resume_from: usize,
     build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
+    probe_tag: Option<&'static str>,
 ) -> Result<ActiveWindowScan, ActiveWindowOutcome> {
-    if units
+    let not_after_units = units
         .last()
-        .is_some_and(|last| virtual_unit.start_index < last.end_index)
-    {
+        .is_some_and(|last| virtual_unit.start_index < last.end_index);
+    // 探针：先于三道守卫无条件重扫一次，供离线复算 nwin 分布 / Σ(m−1) / 批内非末窗坐标
+    // join / guard1 拒绝率——不改变下方守卫对返回值的控制流，只多做一次诊断性观测。
+    // `resume_from > units.len()` 时 `detect_centers_windowed_resume` 的
+    // `while i + 2 < units.len()` 循环条件恒假，安全返回空产出，不 panic。
+    if let Some(tag) = probe_tag {
+        if std::env::var("P602_PROBE").ok().as_deref() == Some("1") {
+            let mut probe_extended = units.to_vec();
+            probe_extended.push(virtual_unit);
+            let (probe_windowed, _m, _c) = detect_centers_windowed_resume(
+                &probe_extended, build, resume_from.min(units.len()),
+            );
+            let nwin = probe_windowed.len();
+            eprintln!(
+                "P602_PROBE_SCAN tag={} not_after_units={} nwin={} resume_from={} units_len={}",
+                tag, not_after_units, nwin, resume_from, units.len()
+            );
+            for (i, (_c, w)) in probe_windowed.iter().enumerate() {
+                eprintln!(
+                    "P602_PROBE tag={} nwin={} idx={} is_last={} start_index={} end_index={} absorbs={}",
+                    tag, nwin, i, i + 1 == nwin,
+                    probe_extended[w.0].start_index, probe_extended[w.1].end_index,
+                    w.1 + 1 == probe_extended.len()
+                );
+            }
+        }
+    }
+    if not_after_units {
         return Err(ActiveWindowOutcome::LowerFrontierNotAfterUnits);
     }
     ...
```

两个调用点：`active_l1_window_frontier` 传 `None`（不探）；`active_l2_window_frontier`（L2 层
重扫，产出 L3 的 C）传 `Some("l2")`。

**与原探针（已删除）的方法论差异（照实登记）**：原探针方法未知（无 patch）。本探针**无条件**
在三道守卫之前跑一次完整重扫，即使 guard1（`not_after_units`）会拒绝该次调用，也照样记录
`nwin`——这是本探针能在**轮次 1**复现「`nwin` 分布覆盖全部 970 次、无 0 桶」这一报告现象的
原因（轮次 1 无截断，guard1 拒绝率高达 728/970，但报告的轮次 1 nwin 分布仍是 970 次全覆盖、
无 0 桶，说明原探针也是「先扫后判」而非「守卫拒绝就不扫」）。

### 1.2 `rust/src/bin/p123_fast_replay.rs`：轮次切换

轮次 2（=当前 HEAD 生产代码，含 F-新2 截断，未改动）：

```rust
let scan_units = if level == 3 {
    &units[..l1_view.confirmed_len.min(units.len())]
} else {
    units
};
```

轮次 1（`/tmp/wt629-probe-r1`，唯一改动）：

```diff
-                let scan_units = if level == 3 {
-                    &units[..l1_view.confirmed_len.min(units.len())]
-                } else {
-                    units
-                };
+                let scan_units = units;
```

## 2. 复现读数（BTC 100k，`P602_PROBE=1`）

| 项 | 报告声明（轮1） | 本次实测（轮1） | 报告声明（轮2） | 本次实测（轮2） |
|---|---:|---:|---:|---:|
| 重扫次数 | 970 | **970** ✅ | 970 | **970** ✅ |
| `nwin` 分布 | `{1:695,2:197,3:24,4:7,5:22,6:24,7:1}` | **逐值相同** ✅ | `{0:145,1:754,2:4,3:14,4:13,5:35,6:5}` | **逐值相同** ✅ |
| 末窗吸收虚拟单元 | 638/970 | **638/970** ✅ | 363/970 | **363/970** ✅ |
| `lower_frontier_not_after_units` | 728 | **728** ✅ | 0 | **0** ✅ |
| `l3:window` | 37 | **37** ✅ | 72 | **72** ✅ |
| Σ(m−1) | 480 | **480** ✅（`Σ(nwin−1)`，与报告公式一致） | **91** | **236**（同一公式）❌ |
| 批内非末窗命中 8 只 λ_C | — | — | 0/8 | **0/8** ✅（全跑扫描，8 个 λ_C 值均未出现在任何 `is_last=false` 的窗口坐标中） |

### 2.1 Σ(m−1) 不一致的诊断（三种候选口径，均试过，均不等于 91）

| 候选口径 | 轮1结果 | 轮2结果 |
|---|---:|---:|
| A. 逐次重扫朴素求和 `Σ(nwin−1)`（报告公式字面读法） | **480**（=报告声明） | 236（≠91） |
| B. `nwin≥2` 的重扫批次数（「批次」而非「实例」计数） | 275 | 71（shadow-602-review 正文提到的「m≥2 的重扫占 71/970」，本评审复核该 71 成立，但 71≠91 且这不是「实例」定义） |
| C. 按 `resume_from` 分组后的窗口坐标去重（跨 bar 重复重扫同一早期窗口只算一次「实例」） | 未测（轮1本身无重复抑制意义，因几乎每次重扫内容都不同——见下） | 9（≠91） |

口径 A 在轮次 1 精确命中报告声明（480），说明**探针方法与报告一致**；同一口径在轮次 2 算出
236 而非 91，说明**轮次 2 的 91 不是「同一公式在轮次 2 数据上的直接结果」**。口径 B/C 均不等于
91，排除「91 是某种去重计数」的猜测。**本评审未能重建出产出 91 的口径**，故只能确认不一致，
不能确认孰是孰非——原探针脚本已删除且报告未附其口径细节，这正是 shadow-602-review P2 指出的
「核心裁断的证据不可第三方复算」在数值层面的具体例证。

**不影响主结论**：无论 91 还是 236，Σ(m−1) 衡量的是「批量丢弃机制的规模」，报告 §3.4 的核心
裁断——「批量丢弃机制成立，但对本窗 8 只 L3 完成身份的解释力为 0（0/8）」——其证据是**批内非
末窗 0/8 join**（本评审逐值复现 ✅），不依赖 Σ(m−1) 的具体数值。Σ(m−1) 只是「机制规模」的量级
描述，91 与 236 都远小于总重扫次数 970 且不改变「0/8」这一结论。

## 3. 结果包（简化版：本文件为诊断附件，不改任何生产代码/教义定义）

1. **结论**：探针可复算物已落盘（本文件 §1 patch + §2 读数）。5/6 类读数逐值复现报告声明；
   Σ(m−1) 在轮次 2 不一致（236 实测 vs 91 声明），照实登记，不静默采信也不静默改写报告数字。
2. **边界条件**：若后续有人能提供原探针脚本或其口径说明，且该口径能同时在轮次 1/2 上自洽复现
   480 与 91，则本文件 §2.1 的「不一致」应改判为「本探针口径与原探针不同」而非「报告数字有误」。
3. **影响声明**：本文件不改动 `/tmp/kimi-nest-mainline` 内任何生产文件；探针 patch 仅存在于
   `/tmp/wt629-probe`（`scan_active_window` 签名改动）与 `/tmp/wt629-probe-r1`（额外去掉 L3
   输入截断），均为隔离副本，未进 commit，交卷后按纪律不保留在生产分支。

## 4. 复现命令

```bash
# 隔离副本
rm -rf /tmp/wt629-probe && mkdir -p /tmp/wt629-probe
git -C /tmp/kimi-nest-mainline archive HEAD | tar -x -C /tmp/wt629-probe
find /tmp/wt629-probe -name '*.rs' -exec touch {} +
mkdir -p /tmp/wt629-probe/analysis/data_cache
ln -sf <repo>/analysis/data_cache/btc_1m_full.json /tmp/wt629-probe/analysis/data_cache/btc_1m_full.json
# 对 nest_lifecycle.rs 打本文件 §1.1 的 diff（scan_active_window probe_tag）

# 轮次 2（截断，=HEAD 原样）
cd /tmp/wt629-probe/rust
CARGO_TARGET_DIR=/tmp/t629probe2 cargo build --release --bin p123_fast_replay
P602_PROBE=1 P116_MAX_BARS=100000 /tmp/t629probe2/release/p123_fast_replay \
  ../analysis/data_cache/btc_1m_full.json > /tmp/wt629-round2.stdout 2> /tmp/wt629-round2.stderr

# 轮次 1（去截断）
cp -r /tmp/wt629-probe /tmp/wt629-probe-r1
# 对 p123_fast_replay.rs 打本文件 §1.2 的 diff（scan_units = units）
cd /tmp/wt629-probe-r1/rust
CARGO_TARGET_DIR=/tmp/t629probe1 cargo build --release --bin p123_fast_replay
P602_PROBE=1 P116_MAX_BARS=100000 /tmp/t629probe1/release/p123_fast_replay \
  ../analysis/data_cache/btc_1m_full.json > /tmp/wt629-round1.stdout 2> /tmp/wt629-round1.stderr

# 读数复算：grep P602_PROBE_SCAN / P602_PROBE / P527_L1_LIVE_OUTCOMES，python3 聚合
# （脚本内联于本任务对话，未落独立文件——纯一次性统计，未来复算请照 §2 表格重写）
```
