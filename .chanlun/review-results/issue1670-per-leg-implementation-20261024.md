# #1670 per_leg 来源逐腿归属：实施与验证记录

日期：2026-10-24。候选分支 `codex/1670-per-leg-attribution`，代码候选 `75e397d860f244186812e5cbb91f2f7463441255`；本记录与工位登记另作纯文档提交。基线为 `origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`。

## TestOnly 归属形状

`per_leg_report.leg_attribution` 是以冻结 `chong_id` 为键的 JSON object；每个值只允许 `units`、`acquisition_cash`、`commission` 三个非负规范整数字符串字段：

```json
{
  "chong-N1": {"units": "2", "acquisition_cash": "200", "commission": "2"},
  "chong-N2": {"units": "1", "acquisition_cash": "100", "commission": "1"},
  "chong-N3": {"units": "0", "acquisition_cash": "0", "commission": "0"}
}
```

键集合必须恰好等于 manifest 的参与重集合，避免缺重、未知重和同一路由身份重复。数值字段直接复用 B 端既有分配字段，不在 E/B 之间另造同义词。报告只有一个 `unit_price`，因此每条归属须满足 `acquisition_cash = units × unit_price`；全部路由的数量、金额、费用分别与原件总数闭合。零数量路由只在金额和费用也为零时合法。缺整个 `leg_attribution` 仍走原有 `Unmatched`，不回落 netted 或比例分配。

## 实现面

- `rust/src/economic_session/tb03c_per_leg.rs`：校验 exact 形状和闭合关系，按来源归属产生确定路由。
- `rust/src/economic_session/tb03c_book.rs`：成功路由进入既有 B 书账；只有显式 `unmatched_reason` 的 per_leg 路由记 Unmatched。
- `rust/src/economic_session/tb03c_c4b_tests.rs`：原 Unsupported 锁改为成功断言，并覆盖恢复、重送、异内容冲突和负控。
- 新增 `PER-LEG-RAW.json`、`PER-LEG-HAND-ORACLE.json`、生成器和 `test_per_leg_oracle.py`；既有 `FROZEN-MANIFEST.json`、`RAW.json`、`HAND-ORACLE.json` 未改。

## 验收证据

- 来源数量 `[2,1,0]` 不等于冻结请求量 `[1,3,3]` 的比例结果，比例分摊变异体会被路由与账簿断言同时拒绝。
- Python 独立 oracle：`python3 s_session/tests/fixtures/tb03c_allocation/test_per_leg_oracle.py -v` → 4 passed；覆盖生成器复现、手算一致、非比例归属及缺重/未知重/数量/金额/费用/零量费用/来源指针负控。
- 既有 Python C4b oracle：`python3 s_session/tests/fixtures/tb03c_allocation/test_c4b_oracle.py -v` → 6 passed。
- 全部 allocation Python oracle：`python3 -B -m unittest discover -s s_session/tests/fixtures/tb03c_allocation -p 'test*.py' -v` → 17 passed。
- Windows 远端基线：`3ff38accdd` 的缺归属测试 → 1 passed。
- Windows 远端候选：`fed87e5e5e` 的 `c4b_per_leg` → 3 passed；成功应用三个 B、恢复后图像逐字一致、同内容重送返回原收据、异内容 IdentityConflict，六组负控保持 E/B 图像不变。日志 `/tmp/nc-remote/20261024-160545-fed87e5e5e.log`。
- Windows 远端代码候选 `75e397d860`：`economic_session::tb03c_allocation_tests` → 18 passed，日志 `/tmp/nc-remote/20261024-172025-75e397d860.log`；`cargo fmt --check` → PASS，日志 `/tmp/nc-remote/20261024-172516-75e397d860.log`。

## 尚未完成的交付门

PR CI、非作者独审与 PR 创建尚待完成。本票未合入 main、未关闭；本文不预写这些结论。
