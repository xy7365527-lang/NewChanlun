R12 实际 GUI 证据核对（#1371）
================================

现有原件支持三组有界链：从 A3 种子出发，拒绝错误响应并保留 A3，再在同页健康重试；两组 Delta 到 A4，Gap 合法重建为新会话 B1。共九个有效阶段，另保留一次失败 UI 操作。字节、来源、控制与请求配对未发现完整性异常。

报告身份是输送工具作者的**证据核对**，不是独立产品审查。实际界面已由根任务操作完成；本次只读核对原件，没有重跑浏览器、服务、测试、native 或数据库。详细逐请求来源、字节哈希、时间及 AX 行号见 [GUI-RESULT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/GUI-RESULT.json)；全部 220 个运行文件见 [GUI-MANIFEST.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/GUI-MANIFEST.json)（SHA-256 `82b9606d9630573c85cc75018126260aa64ffe3ffb57721c75e24fbbf011aa66`）。

运行与字节绑定
--------------

实际返回的 HTML 来自提交 `e5b03629adde165610161b4df3e2d62d22163568` 的 `s_session/browser/index.html`，完整 42,793 字节，SHA-256 `dbd5465fb74bd45f5df71b9c7b824124f91130606392d89c2a8a752e42e20afe`。本次只读固定提交 blob，并与实际三次 GET / 的响应体核对一致。输送工具 SHA-256 为 `24af6b7fbf3fee4593a169b2ef5ba2b42941ce96dd8391503418945ddcd7c170`，冻结输入 SHA-256 为 `cb60cb84b7506d1788e7c850b24d3ab9917ce7ccdda5f6399e29c73fbfcba4b4`。

[RUN.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/RUN.json) 记载 loopback `127.0.0.1:18962`，2026-09-12 12:17:43.676120 UTC 启动；[STOPPED.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/STOPPED.json) 记载 12:26:49.638539 UTC 停止、28 个请求。逐一核对了 28 组 received/completed：25 个 HTTP 200，3 个 favicon 204，28 个 `sendall_completed`。每次完整响应均满足 raw = header + body，长度、Content-Length、SHA-256、状态码与控制快照一致。控制序号 0–8 连续。停止记录本身不证明 GUI 结果。

三组有效 GUI 链
---------------

A 表示会话 `s-session-testonly-001`，B 表示 `s-review-b`。下表各阶段链接到完整 AX；同名 JSON 描述符及 JPEG 均在该目录中，完整路径、SHA、时间已收入机器报告。

| 场景 | 有效种子 | 错误拒绝 | 同页健康重试 | 实际请求 |
|---|---|---|---|---|
| Delta 的 replaces 清空 | [empty-seed3](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/empty-seed3.ax.txt)：A3 / cut-3 / AsKnown | [empty-rejected](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/empty-rejected.ax.txt)：提示 Delta.replaces 错误，保留 A3 | [empty-retry4](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/empty-retry4.ax.txt)：A4 / cut-4 | 种子 [000005](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000005/completed.json)、[000006](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000006/completed.json)；拒绝 [000007](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000007/completed.json)；重试 [000008](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000008/completed.json)、[000009](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000009/completed.json) |
| Delta 的 old 端点错误 | [wrong-old-seed3-confirmed](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/wrong-old-seed3-confirmed.ax.txt)：A3 / cut-3 / AsKnown | [wrong-old-rejected](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/wrong-old-rejected.ax.txt)：提示 Delta.replaces 错误，保留 A3 | [wrong-old-retry4](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/wrong-old-retry4.ax.txt)：A4 / cut-4 | 种子 [000014](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000014/completed.json)、[000015](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000015/completed.json)；拒绝 [000016](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000016/completed.json)；重试 [000017](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000017/completed.json)、[000018](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000018/completed.json) |
| Gap 宣告 B1，错送 A1 | [gap-seed3](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/gap-seed3.ax.txt)：A3 / cut-3 / AsKnown | [gap-wrong-session-rejected](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/gap-wrong-session-rejected.ax.txt)：提示 Gap/Snapshot.session_id 不一致、状态未提交，保留 A3 | [gap-retry-new-session-b1](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/gap-retry-new-session-b1.ax.txt)：B1 / cut-1，并显示已重建 | 种子 [000023](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000023/completed.json)、[000024](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000024/completed.json)；拒绝 [000025](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000025/completed.json)、[000026](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000026/completed.json)；重试 [000027](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000027/completed.json)、[000028](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000028/completed.json) |

三组分别新载页面，再建立 A3 种子。组内拒绝与重试均在 tab `304196083`、同一 URL，中间没有 document/current 加载；错误后健康请求仍使用 `after_generation=3`。这支持同页重试且外部请求未越过 3 的结论。九个有效阶段的相关请求均在各自 GUI 描述符区间内接收并完整发送。十份 AX、十份 JPEG 和十份描述符的哈希与顺序均已核对。

保留但排除的操作
----------------

[wrong-old-seed3](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/wrong-old-seed3.ax.txt) 文件名虽写 seed3，但 12:20:07.680–12:20:07.946 UTC 的 AX 实际仍为 **A1、cut-1、RecomputedWithRevision，as_of 为 —**，截图也显示 A1。相邻 [000013](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000013/completed.json) 只是 current 加载。真正 after0 的 [000014](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/requests/000014/completed.json) 到 12:20:20.720745 UTC 才发生，之后才形成 [wrong-old-seed3-confirmed](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/runs/gui-1/gui/wrong-old-seed3-confirmed.ax.txt)。前者按失败 UI 操作完整保留，**不计入九个有效阶段**。

已目视核对六份不同字节的截图；其余四份与相同状态画面逐字节一致：三个 A3 种子截图相同、两种 Delta 拒绝截图相同、两个 A4 成功截图相同。相同内容在机器报告中明确分组，不视为额外独立样本。截图只含当时视口，session/generation/cut 等完整字段以 AX 对照。

来源与结论边界
--------------

A 的 HTTP 来源是已恢复 implementation-r11 的 api-cuts，B 的来源是已恢复 review-r6 的 additional-results；每条来源容器、JSON 指针、完整原始正文与 SHA 均已逐项核对。种子将原 g1..7 列表限定至真实 g1..3；健康 Delta 将原 g4..7 限定至真实 g4，连同外层 generation/cut 调整并记录；两个负例分别改为空 replaces 或错误 old_object_id。Gap 的原 after99 正文不改字节，在 after3 请求上 TestOnly 重送；错送 A1 与健康 B1 是各自完整原正文，来源请求不等于当前目标的情况已明确记为 false。

因此，这是**本轮实际浏览器对已恢复 HTTP 原件的 TestOnly 输送补证**，不属于新 native 后台运行，也不证明真实 TCP 故障。页面自述的六类缓存对拍是页面输出；本报告没有独立读取内部六个 Map，不由有限 GUI 结果推出整个 B、8AC/17 义务或最终关票通过。

[SERVICE-STOP-RECEIPT.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/R12-GUI-PREP/SERVICE-STOP-RECEIPT.json) 记录根任务从实际工具返回转录的停止结果：Ctrl-C 的 chunk `09224c`、exit 0；监听检查 chunk `37cfcf`、exit 1、空输出。其 802 字节 SHA-256 为 `778abfd7e6277eaab09f354da0596537fb4281773a1366040e07c1c3cd8184d1`。该文件明确是结构化转录，不是平台签名原会话；本次没有再次执行停止或监听检查。
