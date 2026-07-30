# #749（C3）fill.rs 三职责拆分——评估结论：不拆 + 理由

- 票面：#749（parent #743，blocked-by #748——已闭合）；spec #756 C3 形状："campaign wiring 及其测试分家 strategy/oscillation 侧；探针组收敛单 witness 结构"，票面明文准许「不拆+理由」关票（deletion test 反用）。
- 勘察底座：`arch-survey-e2e-fable-20260729.md` §3.2 C3（fill.rs 彼时 5109 行；本评估实测已长至 **6577 行**）。

## 1. 实测数据（call-site 密度）

`rust/src/theta_v0/backtest/fill.rs` 三"职责"实为**同一循环体内的紧耦合子步骤**，非独立模块：

| 职责 | 定义位置 | 调用点数 | 调用方 |
|---|---|---|---|
| `drive_campaign_wiring` | :1341（body 88 行）+ 内嵌测试 :1430-2530（≈1100 行） | **1** 处生产调用（:5128） | 全部在 `pi_theta_fill_loop_overlay`（:4369-6373，同一 ~2000 行循环体）内 |
| `account_mirror_{post,open,close}` | :538-711（≈170 行） | **7** 处生产调用（:653/682/5448/5511/5552/5591/5794/6072） | 同一循环体内，逐腿传入 `account_view`/`leg`/`px`/`i` 等循环局部可变态 |
| 8 组探针（`*_probe_bump/reset/count/snapshot`） | 散落 :366-524、:4127-4368 | 循环体内部 bump；`fill.rs` 自身测试 + `runner_tests.rs` 跨文件读 | 生产调用方同为该循环体 |

`pi_theta_fill_loop_overlay` 是**唯一生产调用方**——三"职责"不是三个独立子系统互相协作，而是一个 2000 行循环对三组 helper 的顺序编排。搬文件不改变该循环的编排复杂度：读者理解这条循环仍需依次理解 account_mirror 语义 + campaign wiring 语义 + 探针语义，无论这些函数体物理落在哪个文件。

## 2. deletion test 反用论证

- **drive_campaign_wiring**：单一生产调用点 + `strategy/oscillation_campaign.rs` 已有 3 处注释反向引用它（:491/:1003/:2638，自述"生产侧 drive_campaign_wiring 对每个(级别,侧)每 bar 必调一次"）。即便搬迁，读者仍须跨文件核对循环侧的调用上下文（bar/level/side 编排）与 oscillation 侧的语义——**搬迁不消灭跨文件跳转，只是换了跳转方向**，不构成deletion test 意义上的"复杂度真集中"。
- **account_mirror_\***：三个私有 helper（`fn`，非 `pub`）只在本文件内被同一循环 7 处调用，无外部消费者、无独立测试文件引用（`runner_tests.rs` 只在注释中提及，非实际调用）。是循环的直接副作用实现，非可独立复用的模块——搬去 `strategy/account.rs` 除了物理挪位没有额外内聚收益。
- **8 组探针**：这是唯一"结构真有改善空间"的一处（8 个并列 bump/reset/count 家族 → 1 个 witness struct 确有归并价值），但也是唯一**跨文件**（`runner_tests.rs` 消费 count/snapshot）的一处——改造要触达生产 bump 调用点（循环内部）+ 两个文件的读取断言，是本票范围内风险最高、生产收益（可读性局部改善，无生产语义变化）最低的一块。

## 3. 风险 vs 收益

- **风险**：fill.rs 是 wf8 双锚（trades.jsonl/tower_events.jsonl 逐字节 cmp=0）覆盖的核心生产路径，任何函数搬迁都要求全量重跑双锚 + lib 全绿验收；三块中两块（drive_campaign_wiring、account_mirror）单/少调用点意味着**搬迁本身机械风险低**，但收益也同样低（真实认知负担来自循环编排本身，不来自函数物理位置）；探针一块收益略高但改造面刺破测试断言、风险最高。
- **收益**：可读性局部改善（文件行数下降），无接口收窄、无消费者解耦、无判据/绑定单源化之类"结构性"收益（对照 C1/C4/C6 三票——它们都真实消灭了复制点或错层）。C3 与那三票不同类：C3 是纯搬家候选，不是纠错候选。
- **结论**：三职责的"耦合密度"实测为"同循环体单编排"，拆分收益（局部可读性）不抵成本（碰金标准锚 + 探针改造的跨文件断言风险）。按票面明文准许，**判定不拆**。

## 4. 重启拆分的触发条件

- 若 `drive_campaign_wiring` 的生产调用点从 1 处增至 ≥2 处（例如 N7 消费点接入后从多个路径调用），届时其编排语义已脱离单一循环，搬迁才真正带来"复用点单源化"收益；
- 若探针组数量继续增长（当前 8 组）或其跨文件读者从 1 个（`runner_tests.rs`）增至 ≥3 个，届时并入 witness struct 的改造/风险比会反转，值得单独立票；
- 若 `pi_theta_fill_loop_overlay` 本体（现 ~2000 行）本身被拆分为更小的子循环（非本票范围），届时 account_mirror/campaign wiring 的物理位置应随新边界重新评估。

## 5. 后续动作

- 本票（#749）按「不拆+理由」关闭；C3 在 #743 依赖链上已是末位（殿后），七票主线（C5→C6→C2→C1→C4→C3）随此报告全数完结，无阻塞遗留给 #529/#743 后续。
- 不涉及代码改动，未 commit。
