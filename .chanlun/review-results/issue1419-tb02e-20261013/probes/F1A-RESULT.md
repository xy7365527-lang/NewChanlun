# #1419 F1a 打点诊断结果（TB-02-D 冻结包副本，树外诊断，不进准入证据）

运行：2026-10-14 08:04:41–08:16:24，两臂各一次、不重试，exit 均为 0。产物 `/Volumes/AgentStorage/issue1419-f1a-20261014/`：`F1A-TIMELINE.json`（sha256 `8baf1f9f…`）、`F1A-PLAN.json`（`aab3c65c…`）、`RUN-STDOUT.txt`（`a38fc143…`）、`SLOT-SNAPSHOT.json`（`64dc0a39…`）、`SLOT-REAP.json`（`d59e3cd5…`，clean，清掉残留 .sock 2 个）。源为 `179412c33d`（与冻结包 101 个源逐个比过 SHA），写者为冻结写者 `58a13510…`，两臂消息与冻结包逐字节相同；补丁只加打点。

**与冻结包的偏差**：
- Chrome：本次 `chromium.launch` 拉起的是磁盘上的 155.0.8059.40（`Google Chrome` 可执行 sha256 `428dccc1…`）；冻结包当时是 154.0.8037.99。用户自己正在运行的 Chrome 进程仍是 154 的 framework（待重启更新），与本探针无关。
- 计时背景：开跑前 1 分钟负载 7.3（`receipts/closeout/probe-background-load-f1a.txt`），没有 cargo/rustc/lake/S/Q；背景同 S0b（用户 Chrome 154 约 1.5–3 核、agent-browser 无头 Chrome 约 1 核、WindowServer、CLIProxy）。
- 页面用的是 playwright 非持久上下文（隔离临时 profile）。

## 读数

| 项 | second_open_up | second_open_down |
|---|---:|---:|
| 臂用时 | 347.4 s | 356.3 s |
| 浏览器窗（READINESS） | **29.55 s** | **28.39 s** |
| 浏览器结果 | passed，page_errors=[] | passed，page_errors=[] |
| goto | 0.04 s | 0.03 s |
| 等最终提交（commit_final_wait） | 9.78 s | 9.82 s |
| AsKnown 到提交（asof_to_commit） | 4.99 s | 4.78 s |
| 五次点击合计 | 9.17 s | 9.03 s |
| 　其中首次点击 CC-011 | 4.69 s | 4.83 s |
| innerText 合计 | 2.12 s | 1.67 s |
| 截图合计（4 张） | 1.78 s | 1.83 s |
| context + browser 关闭 | 0.42 s | 0.56 s |
| 页内 `renderAllFromState`（2 次） | 1.61 s（单次最大 0.97） | 1.62 s（0.99） |
| 页内 `validateTB02Candidate`（2 次） | 0.47 s | 0.47 s |
| 页内 `validateState`（2 次） | 0.37 s | 0.37 s |
| Q 冷 worker 首次 verify | **4.54 s** | **4.51 s** |
| Q 热 worker verify（42 次） | 最大 1.18 s，合计 15.9 s | 最大 1.18 s，合计 15.7 s |

**30 s 浏览器帽：两臂都过了，余量 0.45 s 与 1.61 s。** 冻结包 P2 四臂均值 29.49 s（DIAGNOSIS §0），本次在同一量级。Q verify 5 s 期限：冷 worker 4.5 s，余量约 0.5 s。

## 29.5 s 的去向

按打点，浏览器窗主要由三块组成，三者合计约 24 s：

1. **等 S/Q 提交**约 14.8 s：等最终提交 9.8 s，加 AsKnown 那一次 4.8–5.0 s。最终提交等待里包含一次 Q 冷 worker 的首个 verify（4.5 s）。
2. **点击**约 9.1 s：其中首次点击 CC-011 一项就是 4.7–4.8 s，后面每次 1.1–1.5 s，AsKnown 那次 0.4 s。首次点击偏长，与页面第一次整态渲染时间相近（`renderAllFromState` 单次最大约 1 s），其余部分未细分，原因是推断。
3. **读页与截图**约 3.8 s：innerText 1.7–2.1 s，截图 1.8 s。

页内三段 JS 合计只有约 2.5 s（渲染 1.6 s、候选校验 0.47 s、状态校验 0.37 s）。所以浏览器窗的大头在「等提交」和「点击后的页面响应」，页内校验本身不是瓶颈。

## 对 D-F 的含义（推断）

TB-02-D 的 40 根输入已经吃满 30 s 帽的 95–98%。按计划 §4.4，TB-02-E 只点 CC-016/017/022 三轴加 AsKnown 一次，点击次数比 TB-02-D 少一次，但页面对象量随输入根数增长；最少根数输入约 51 根（见根裁 D-E 的设计稿），比 40 根多约 28%，首次点击与渲染会随之变长，余量 0.5–1.6 s 很可能不够。D-F 已批的 F3(a)(c)（离屏 `content-visibility`、索引只建一次）能省的只是渲染与点击响应里的一部分；等提交那约 15 s 与 Q 冷 worker 的 4.5 s 不在 F3 范围内。这些都是推断，要在 TB-02-E 候选上做 F1b 才能定量。
