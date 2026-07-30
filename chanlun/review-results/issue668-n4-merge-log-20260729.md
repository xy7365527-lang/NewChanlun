# ticket-668c（N4 事件↔BSP 桥接对象）合流 main —— merge log

日期：2026-07-29　执行：编排者 session（Kimi）手工合流（分支为干净重贴版 ticket-668c）

## 关键 hash

| 项 | hash |
|---|---|
| merge commit | `a4c24e86d1`（第一父 = main `b16ece1dbc`，第二父 = ticket-668c 尖端 `ebece2e68b`） |
| 分支基 | `1b7ba1a1a5`（cherry-pick 重贴自 ticket-668b，侧线污染已弃，#668 票内登记） |

## 并发插曲（main 极热，基点四落）

main 当日并发 session 持续落地，ff 四次才被允许：**①** `234ad22f70` → ff 时被 #761 C7-E2 抢先；**②** `b1032618e6` → 验收发现 env_registry 测试红，核为**先存红**（#758 restore 恢复 m8/report 旧文件带回裸 env 字面量，碰 #746 注册表门；净树复现，非合流引入），且 #758/#766 线已在修；**③** `586ace1510`（#766 收口批，该红已转绿）→ 合并树验收全绿（2624/0/139），commit `f694d7d438` 已写就，ff 时又被 #774（#634 漏网 battery 修复）抢先，**该轮 merge log 文件随弃**（本文件为事后补登，名实照实）；**④** `b16ece1dbc` → 同解法重合并，验收 2624/0/139，ff 成功 = `a4c24e86d1`。

## 冲突与解法（2 文件，均并集，四轮基点同型）

| 文件 | 冲突 | 解法 |
|---|---|---|
| `CONTEXT.md` | main 侧 #685 两词条（地板三型/严档悬置档）与分支 N4 两词条同位追加 | 并集全保留 |
| `classifier/mod.rs` | main 侧 #106 `pub mod interval_necessity;` 与分支 `pub mod bsp_bridge;` 同位插入 | 并集全保留 |

## 合并后适配（1 处，#634 API 漂移）

#634 把 `cand_event.rs` 拆五件并将 `CandidateObservation.state` 从 `CandidateState`（四态）改为 `ObservedState`（三态无 Invalidated）。分支测试 `bsp_bridge/tests.rs:47` 构造点适配（`ObservedState::Provisional` + import）。**生产代码零适配**（bsp_bridge.rs 用四态事件侧，未受影响）。main 侧 #774 同期修同族漏网（issue550_event_battery），互不重面。

## 验收门读数（合并树实测，`b16ece1dbc` ⊕ ticket-668c）

- `cargo test --lib`：**2624 passed / 0 failed / 139 ignored**
- 分支侧既有门沿用 #668 票内读数（对拍三窗 cmp=0、幂等 300k delta2=0、查询 29/29；合并属纯叠加 + 两并集 + 一适配）
- 主仓工作区他 session 在飞脏面与分支 18 文件**零重叠**（comm 实测）

## 遗留（照实）

- 一闪红 1/60 局（#668 票内登记，未归因）；二类桥接待编排者裁；#688/#754/#671 在 frontier；分支两诊断 bin 主仓原生可跑（数据 `analysis/data_cache/btc_1m_full.json` 本机在册）。
