# #1419 第 5 项：F1a 打点诊断与 S0b 规模探针（树外诊断，不进准入证据）

两个探针都在仓外副本上跑，补丁会改源字节，所以产物只用于定位耗时，不能当作 #1414 或 #1419 的准入证据。每份计划文件都写了 `"admission_evidence": false`。

F1a 回答「冻结包里浏览器窗口那 29.49 s 花在哪」：在 TB-02-D 冻结包（#1414 P2 rerun，源 `179412c33d`、写者 sha `58a13510…`）的副本上，给 Q、页面和浏览器 harness 打 F1 标记，重跑两个臂，产出逐阶段时间线。S0b 回答「#1419 的 S 写者在段层规模上能不能在帽内跑完」：用设计好的段层流生成笔和原始 K 线，喂给 #1419 头的 release S 二进制，记录每条消息的摄入往返和查询耗时，拟合增长曲线并对照各道帽。

## 文件

| 文件 | 作用 |
|---|---|
| `instrument.py` | 给副本打 F1 补丁，每处替换断言恰好命中一次。Q 经 `F1_Q_SIDECAR` 旁路记录 capture、verify、cache_hit 与 worker 号；`index.html` 用 `performance.measure` 包住 `validateState`、`renderAllFromState`；`tb01c-client.js` 包住 `validateTB02Candidate`；`tb02d_browser.cjs` 逐条写 `F1-MARKS.jsonl`，结束写 `F1-MARKS.json`。 |
| `f1a.py` | `prepare` 用 `git archive 179412c33d` 导出副本，与冻结包 RUN-PLAN 的 101 个源逐个比 SHA，打补丁，造 `second_open_up`/`second_open_down` 两臂（端口从 19410 起）；`run` 逐臂跑副本的 `tests/tb02d_runtime.py`；`extract` 产出 `F1A-TIMELINE.json`。 |
| `s0b_gen.py` | 从段层流（S0 oracle `worked-examples.json` 的流，或 11 单元短流 `SHORT11`）生成笔端点和 TB-02-D anchor 形状的原始 K 线（每笔 4 根，`O=C=q`、`L=q−1`、`H=q+1`）。不 import 项目实现。 |
| `s0b_probe.py` | `prepare` 导出 #1419 头（`48e2386463`）的 `s_session`，打 Q 旁路补丁，按 per-bar（一根一条）与 batched（默认每 16 根一条）两种消息造臂（端口从 19430 起）；`run` 用正式 launcher 起 S/Q，逐条 accept+advance；`extract` 产出 `S0B-PROBE.json`。 |
| `probe_slot.py` | 探针槽进程回收与收据（同 #1405 浏览器那一轮，需提权 ps/lsof）。`snapshot <out>` 记运行前已有的 Chrome/S/Q；`reap <out> --ports a-b --socket-prefix P [--kill] [--kill-orphan-chrome]` 只认本票进程（自身或祖先命令行含 `<out>` 或 socket 前缀），孤儿 playwright Chrome 单列，残留 `.sock` 一并清，写 `SLOT-REAP.json`。 |
| `dryrun/` | 演练产物。`dryrun/f1a` 的 prepare 已通过（101 个源比对，消息与冻结包相同）；`dryrun/s0b` 是 L3 之前用旧头 `9cd64da5cd` 演练的，已改名 `s0b-OBSOLETE-pre-L3-9cd64da5cd` 作废。 |

## 运行前门

两个 `run` 都先过同一道门，任一不过就退出，不开跑：

1. `peer_build_busy.py` 对全部工位 target（含 `tb1481-target`）返回 `IDLE`；
2. 计划里的端口空闲；
3. 二进制 SHA 与 prepare 时记下的值一致（F1a 绑冻结写者 `58a13510…`，S0b 绑 prepare 时 release 二进制的 SHA）。

帽沿用原值，一律不放宽：臂窗 900 s；F1a 浏览器 30 s、清理 5 s、查询 30 s、`collect_core` 120 s、`frozen_oracle` 180 s；S0b 每条回复 10 s（`Tb02aRun` 原值）、Q verify 5 s。浏览器或计时运行期间不得并行 cargo（规则 D-H）。

## 命令

```sh
# F1a（需要浏览器槽；无需 cargo）
python3 f1a.py prepare            # 默认输出 /Volumes/AgentStorage/issue1419-f1a-20261014
python3 f1a.py run [--only second_open_up]
python3 f1a.py extract

# S0b（需要一个 cargo 槽构建 release 二进制；运行不需要浏览器）
cd /private/tmp/goal1323-native-20261012/tb1419-wip-wt/rust
CARGO_TARGET_DIR=/private/tmp/goal1323-native-20261012/tb1419-target \
  nice -n 10 cargo build --release --features s_session --bin s_structure_session
cd /private/tmp/goal1323-native-20261012/tb1419-impl/f1-s0b
python3 s0b_gen.py --stream UP --out <dir>/s0b-UP.json
python3 s0b_gen.py --stream SHORT11 --out <dir>/s0b-SHORT11.json
python3 s0b_probe.py prepare --inputs <dir>/s0b-UP.json <dir>/s0b-SHORT11.json
python3 s0b_probe.py run          # 默认输出 /Volumes/AgentStorage/issue1419-s0b-20261014
python3 s0b_probe.py extract
```

`prepare` 用 `mkdir(exist_ok=False)` 和 `open("x")`，输出目录已存在就失败，不会覆盖旧产物。

## 估时（推断，依据见下）

| 探针 | 资源 | 估时 |
|---|---|---|
| F1a | 浏览器槽 | prepare 约 1 分钟 + 两臂各约 6 分钟，共约 13 分钟 |
| S0b 构建 | cargo 槽 | release 构建，约 10–14 分钟（参照：组合树上 release 测试构建加运行约 14 分钟） |
| S0b 运行 | 无浏览器、无 cargo | batched 两流各数分钟；per-bar 预计在帽前中止 |

依据：冻结包四臂共 1491.7 s，约 373 s/臂，浏览器窗均值 29.49 s。TB-02-D 单输入摄入拟合为 `ms ≈ 1.5175·n^2.13`（第 40 条 3858 ms），查询均值 1.8 s、最大 7.06 s。按这条曲线外推，per-bar 约在第 62 条超过 10 s 回复帽，跑不完 212 根，所以能跑完的只有 batched。这是外推，S0b 的 per-bar 臂本身就是对这条外推的实测。

> **订正（2026-10-14 实测后）**：「只有 batched 能跑完」不成立——S 协议要求一条 ingest 只带一个事件（`InvalidDomain：ingest必须一事件`），batched 两臂在第 0 条即被拒，属探针前提错误。per-bar 实测在第 48 条（下标 47）过 10 s 帽，比拟合外推的第 62 条早，原因是第 46 条附近有一次跳变。读数见 `S0B-RESULT.md`、`F1A-RESULT.md`、`min12/MIN12-RESULT.md`。

## 已知风险

- 补丁改源字节，结果只作诊断。
- S0b 第一道门（已发布 CC-013 的完成段个数与端点 = 设计段端点 `segment_endpoint_prices`）还没实测；不符先报，不改断言。根裁定 L3 之后单元区间取成员笔极值，设计流里段端点就是段内极值，预计不受影响，但要由实测确认。
- UP/DOWN 流有 5 个单元、SHORT11 有 4 个单元不满足 3 笔相邻约束，退为 5 笔阶梯，`s0b_gen.py` 输出里点名。

## 正式准备（2026-10-14 07:2x，根排槽第 1 步）

- S0b：release 二进制在 `48e2386463` 上构建（`receipts/closeout/s0b-1-release-build.txt`），固定副本 `tb1419-impl/artifacts/s_structure_session-48e2386463`，sha256 `38c61eee…`；`tb1419-target` 增量已清。输入 `inputs-48e2386463/`（与旧演练逐字节相同，生成器确定性）。`prepare` 输出 `/Volumes/AgentStorage/issue1419-s0b-20261014`，四臂 UP/SHORT11 × per-bar/batched，端口 19430–19433，socket 前缀 `/private/tmp/tb1419s0b-`。
- F1a：`prepare` 输出 `/Volumes/AgentStorage/issue1419-f1a-20261014`，101 个源与冻结包相同、两臂消息与冻结包相同；端口 19410–19411，socket 前缀 `/private/tmp/tb1419f1a-`；playwright 模块取根指定路径；`run` 时 `TB02D_CHROMIUM_EXECUTABLE` 指向 `/Applications/Google Chrome.app`（冻结包 DEPENDENCY 声明的真 Chrome，本机现为 155.0.8059.40，冻结时为 154.0.8037.99），`chromium.launch` 非持久上下文 ⟹ 隔离临时 profile。
- 跑法（等根「探针槽开」，S0b → F1a，各一次不重试）：`probe_slot.py snapshot <out>` → `<probe> run` → `<probe> extract` → `probe_slot.py reap <out> --ports … --socket-prefix … --kill`。
