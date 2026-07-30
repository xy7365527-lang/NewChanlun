# #668（N4）文字收口轮——#670 影子三审 PWC 收口（2 MED / 3 LOW）

- dispatch：#668 文字收口轮，编排者 2026-07-29 批，工位 `/tmp/wt-668c`（分支 `ticket-668c`），
  全程前台单线程，未派发任何子代理/后台任务。
- 起点：`0e3d0ca82c`（三审报告落地提交）。评审对象：
  `chanlun/review-results/shadow-668-review3-20260729.md`（判 PWC，R2 全部 10 条真修实、新增
  2 MED / 3 LOW，无 HIGH）。
- 处置原则：本轮不改判据、不改生产行为——纯文书 + 独立参照集实现对齐（让参照集追上已裁定的生产
  判据，不是新裁定）。生产代码零改动（`bsp_bridge.rs` 唯一改动是文档注释，非逻辑）。

## 逐条处置

### R3-MED-1：`CONTEXT.md` 两词条冻结在修复轮 1，四处名实不符

按 append-only 纪律（删除线保留原句 + 订正段），逐句核对现行实现改写：

1. 「事件↔BSP 桥接边」词条：`heads()` 只见链头/历史物理点见 `edges()` 那句——**这正是
   R2-HIGH-2 已修掉的破行为本身**，改为：折叠为一条 revision 携有序 `source_index` 集合，
   `heads()` 携完整覆盖点集、`edges_for_bsp_point` 按集合成员判断，300k 窗查询完备性 29/29。
2. 三类「判据未改」——订正为已迁移到 episode 区间覆盖（第四轮 supersede 裁定 2 显式撤销该例外）。
3. 「BTC 三窗实测零违反」——订正为证据须收窄至仅一类、仅 100k/300k 两窗（20k 窗 `episodes=0`，
   `EMPTY_DOMAIN` 不能作证据），合计 32 个 bit 实例 `owned_many=0 max_owners=1`。
4. 同段补 `debug_assert` 生效域说明（仅 debug profile，release 为空操作）——见 R3-LOW-3。

文件：`CONTEXT.md`（两处词条本体，未动他条）。

### R3-MED-2：对拍 bin 参照集三类判据未随生产迁移，头注「本轮未改」与事实相反

`p_issue668_bsp_bridge_battery.rs::reference_join` 三类分支此前仍用第四轮 supersede 之前的
`trend_by_exact_end`（`leave_interval.1` 精确等值），与生产 `resolve_bridge` 三类分支（已迁移
`find_episode` episode 区间覆盖）不同构。处置：

- 三类判据迁移到 `find_covering(level, side, parent, entry.leave_interval.1)`——**复用参照集
  自己的独立扫描实现**（与一/二类同一个 `find_covering` closure），不调用/不复写生产
  `find_episode`，代码路径独立性不变。
- 废弃的 `trend_by_exact_end` 索引一并删除（迁移后无调用点，编译期确认无死代码警告新增）。
- 头注「本轮未改」按 append-only 纪律加删除线 + 订正段。

**迁后三窗复跑**（release，`analysis/data_cache/btc_1m_full.json`）：

```
bars=20000  BATTERY_BY_CLASS Buy3/Sell3 reference=0 produced=0 cmp=0 note=此类空域，cmp无信息量
bars=100000 BATTERY_BY_CLASS Buy3/Sell3 reference=0 produced=0 cmp=0 note=此类空域，cmp无信息量
bars=300000 BATTERY_BY_CLASS Buy3/Sell3 reference=0 produced=0 cmp=0 note=此类空域，cmp无信息量
```

**照实报数**：迁移后三类两侧仍同为空集（与三审报告独立探针 `production_cover_hit=0
reference_exact_end_hit=0` 一致）——判据同构不等于命中数变化，三类近零覆盖是独立于本次判据
对齐的既有事实（根因未查，归 #688）。三窗总 `BATTERY` 行（`ref/prod/heads/cmp`）与三审报告
逐位一致（20k: 0/0/0/0；100k: 3/3/3/0；300k: 29/22/29/0，`heads` 计数折叠语义不变）。

文件：`rust/src/bin/p_issue668_bsp_bridge_battery.rs`。

### R3-LOW-1：battery bin `:189` 二类行内注释仍携已撤回断言

`// 二类：...（HIGH-2 修复覆盖二类）`——与真值表 bin 头注、ADR 第五轮 §3 已撤回的断言同型，
原处补撤回标注（二类锚坐标生产上恒判 `None`、零命中，此判据未实测）。

### R3-LOW-2：ADR 第三轮裁定 2 关于二类的半句原处无「未实测」标注

`docs/adr/0008-…md`「二类同样按其一类锚坐标走 episode 区间覆盖（沿用同一失效模式的修复）」——
原处补「未实测」标注 + 指向第五轮 §3 的查明结论，不再是文末孤立登记。

### R3-LOW-3：「机器保证」只在 debug profile 生效，四处补生效域说明

`find_episode` 的 `debug_assert!` 仅 debug profile 生效，release 编译为空操作；release 下若
归属不再唯一会静默取遍历序首个 episode，唯一 release 可见探测是真值表 bin 独立实现的
`episode_owned_many` 计数（不在库内路径上）。四处补齐：

- `rust/src/theta_v0/classifier/bsp_bridge.rs` 模块头（`:36-42` 段）。
- `docs/adr/0008-…md` 第三轮裁定 1。
- `docs/adr/0008-…md` 第五轮 Consequences。
- `CONTEXT.md`「BSP 结构身份键 v2」词条。

## ADR-0008 第六轮 supersede

补一段收口段，登记本轮 5 条处置、声明「本轮不改判据、不改生产行为」，二类锚归属结构性不可解
的补充数据（145/280 锚坐标在簿内但无一持一类 bit、135/280 锚坐标根本不在簿内）留待编排者裁定，
不给方案。不关票。

## 验收

```
cargo test --lib             → 2569 passed; 0 failed; 138 ignored（debug，基线 2569 ✓）
cargo test --lib --release   → 2565 passed; 0 failed; 138 ignored（release，基线 2565 ✓）
cargo check --all-targets    → Finished（仅既有 warning，exit=0）
cargo test --release --test theta_v0_lean_parity --test theta_v0_buy_parity
                              → 11 passed; 0 failed
p_issue668_bsp_bridge_battery 三窗（20k/100k/300k）→ 逐位一致三审报告读数，cmp=0，
                                                       三类迁移后仍空域（照实）
git diff --stat -- rust/src/theta_v0/backtest/ rust/src/bin/p92* rust/tests/
      rust/src/theta_v0/classifier/cand_event.rs rust/src/theta_v0/classifier/bsp.rs
      rust/src/theta_v0/signal.rs rust/src/theta_v0/types.rs   → 空（p92/runner/golden/老对象零 diff）
```

生产行为零改动：`bsp_bridge.rs` 唯一改动是模块头文档注释（补 debug_assert 生效域说明），
无逻辑/签名/调用点变更。`p_issue668_bsp_bridge_battery.rs` 改动限独立参照集/验收物，非生产
消费路径（全仓 `bsp_bridge` 生产引用面仍仅 `classifier/mod.rs:107` 一处，未新增）。

## 改动文件

- `CONTEXT.md`（两词条 append-only 订正）
- `docs/adr/0008-bsp-bridge-object-and-structural-key-v2.md`（第三/五轮段落补生效域标注 + 第六轮
  supersede 收口段）
- `rust/src/bin/p_issue668_bsp_bridge_battery.rs`（三类判据迁移 episode 覆盖 + 二类行内注释撤回
  标注 + 头注订正）
- `rust/src/theta_v0/classifier/bsp_bridge.rs`（模块头补 debug_assert 生效域说明，纯注释）

不关票。
