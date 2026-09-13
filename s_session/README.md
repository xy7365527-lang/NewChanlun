# 正式结构会话 S（TB-01-A/B/C）

正式 launcher 管理独立 Rust S（SQLite/WAL 单写者）和独立只读查询进程 Q；浏览器通过 Q 观察结构事实。受测域仅为具名 TestOnly 原始逐笔事件一对一精确退化 OHLC（O=H=L=C）的 `CC-006/local_shape` 四分支。不启动 E/B/X、不建其库、不加载经济政策；结构 scope 为 `CompleteCut`，经济 scope 为 `not_started`。

v1 保留 `init/accept/advance/catalog/snapshot/meta` CLI 与旧 GET 读口；v2 增加常驻 S、持久消息收据、语义时钟、恢复、固定 cut 分页和 Watch/Gap。v2 当前从新库初始化，不提供 v1 库自动迁移。以下命令从仓库根目录执行，工作原件保存在自选的仓外目录；这份说明不授予 C 最终验收。

## v2：构建与显式配置

需要 Rust/Cargo、可链接 `libpython3.11` 的 Python 3.11；HTTP 采集和浏览器测试另需支持本仓脚本所用 Node API 的 Node.js。Python 与动态库必须来自同一运行时。以下使用独立构建目录，不覆盖已有二进制；也可直接把 `S_BINARY` 指向已构建并保留 SHA-256 的同源二进制。

```bash
export S_REPO="$PWD"
export S_PYTHON="$(command -v python3.11)"
export S_RUN_ROOT="$(mktemp -d /tmp/tb01c.XXXXXX)"
export PYO3_PYTHON="$S_PYTHON"
S_LIBDIR="$("$S_PYTHON" -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR") or "")')"
export LD_LIBRARY_PATH="$S_LIBDIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$S_LIBDIR${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export CARGO_TARGET_DIR="$S_RUN_ROOT/build"
cargo build --manifest-path "$S_REPO/rust/Cargo.toml" --release \
  --features s_session --bin s_structure_session
export S_BINARY="$CARGO_TARGET_DIR/release/s_structure_session"
"$S_PYTHON" -c 'import hashlib,os,pathlib; p=pathlib.Path(os.environ["S_BINARY"]); print(p,hashlib.sha256(p.read_bytes()).hexdigest())'
```

下面生成三套配置：`manual` 用于手工操作，`a/b` 用于两次完整轨迹。端口须空闲，Unix socket 路径不得超过 100 字节。各配置、数据库和控制目录互相隔离；相同业务身份用于同源双跑。启动配置中的资源整数为规范十进制**字符串**；Q 的内部资源 JSON 使用正整数 **number**。代码从 fixture 读取资源数值，不另造默认。

```bash
"$S_PYTHON" - <<'PY'
import json, os
from pathlib import Path
repo = Path(os.environ['S_REPO']).resolve()
work = Path(os.environ['S_RUN_ROOT']).resolve()
fixture = repo / 's_session/tests/fixtures/tb01c'
plan = json.loads((fixture / 'runtime-plan.json').read_text())
first = json.loads((fixture / 'messages.jsonl').read_text().splitlines()[0])
for name, port in [('manual', 18787), ('a', 18788), ('b', 18789)]:
    run = work / name
    run.mkdir(exist_ok=False)
    config = {
        'schema_revision': 's-launcher/2',
        **{key: first[key] for key in ('session_id', 'session_generation',
            'source_namespace', 'source_epoch', 'producer_id', 'producer_epoch')},
        'writer_epoch': '1', 'query_epoch': '1',
        'delivery_retain_generations': plan['delivery_retain_generations'],
        'port': str(port), 'startup_timeout_ms': str(plan['startup_timeout_ms']),
        'db': str(run / 'session.sqlite'), 'socket': str(run / 's.sock'),
        'binary': str(Path(os.environ['S_BINARY']).resolve()),
        'python': str(Path(os.environ['S_PYTHON']).resolve()),
        'catalog': str(repo / 's_session/catalog/signed-catalog.json'),
        'profile': str(fixture / 'profile.json'),
        'clock_plan': str(fixture / 'clock-plan.json'),
        'browser': str(repo / 's_session/browser/index.html'),
        'query_resource_config': str(fixture / 'query-resources.json'),
        's_resources': plan['s_resources'],
    }
    with (run / 'config.json').open('x') as output:
        json.dump(config, output, ensure_ascii=False, indent=2)
        output.write('\n')
PY
```

所有路径字段按配置文件所在目录解析；示例写成绝对路径以便核对。`state-dir` 与该数据库绑定，不能换一个控制目录采用未知进程。配置中的 `producer_id/producer_epoch` 是控制请求生产者身份；S 的 `writer_epoch` 和 Q 的 `query_epoch` 分别管理，不能互相代替。

## 独立启动、停止与恢复

```bash
./s_session/launch_s.sh service start-s \
  --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
./s_session/launch_s.sh service start-q \
  --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
./s_session/launch_s.sh service status \
  --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
curl --fail http://127.0.0.1:18787/api/state
# 浏览器打开 http://127.0.0.1:18787/
```

`start` 顺序启动 S/Q；`start-s` 在数据库不存在时先按配置初始化 v2，再启动 Unix socket 服务。Q 使用 SQLite `mode=ro`，不依赖 S socket，不写 S 库。GET `/api/state` 是已核健康/发现读口；公共头 POST 为 `/api/v2/snapshot` 和 `/api/v2/watch`。控制器把 PID、启动时间、完整命令和本次实例 nonce 一起核对后才返回 Ready；`status` 的 running 仅表示登记进程仍匹配。

手工输送可取一份冻结原请求，不能自行更换消息身份重试：

```bash
"$S_PYTHON" - <<'PY'
import os
from pathlib import Path
source = Path(os.environ['S_REPO']) / 's_session/tests/fixtures/tb01c/messages.jsonl'
target = Path(os.environ['S_RUN_ROOT']) / 'manual/first-request.json'
with target.open('xb') as output:
    output.write(source.read_bytes().splitlines()[0] + b'\n')
PY
"$S_PYTHON" s_session/s_socket_client.py \
  --socket "$S_RUN_ROOT/manual/s.sock" --request "$S_RUN_ROOT/manual/first-request.json" \
  --timeout-ms 10000 --max-frame-bytes 1048576
```

`Received`/退出 0 只证明完整输送，业务成功还须核响应公共头、原因果身份和 payload。`DeliveryUnknown` 表示发送开始后未收到完整回执，不能解释为未发生；应使用原 source/message/payload_hash 查询持久收据。冻结 driver 已按原身份执行收据采集。

独立重启 Q 不停止 S；Q 代际要明确递增。以下手工示例以 Q 从 1 重启到 2 为准：

```bash
./s_session/launch_s.sh service stop-q --signal TERM \
  --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
./s_session/launch_s.sh service start-q --query-epoch 2 \
  --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
./s_session/launch_s.sh service stop \
  --config "$S_RUN_ROOT/manual/config.json" --state-dir "$S_RUN_ROOT/manual/control"
```

`stop-s` 只停止 S，`stop-q` 只停止 Q，`stop` 按 Q→S 停止；默认 TERM，显式 `--signal KILL` 才发送 SIGKILL。控制器只操作已登记且身份仍匹配的进程，停止超时不自动升级信号。恢复须先确认同一控制目录记录的 S 已停止，再调用 `recover`；它只完成持久恢复，随后须 `start-s --writer-epoch <新代际>`。恢复事件必须存在于该库绑定的 clock plan，且与当前故障轨迹匹配。

下面是冻结 op95 故障点的恢复入口形式，**不接在上述手工首条输入后执行**；完整 driver 会在真实 after_begin 标记核验后执行它，并保留原消息身份与 pending 恢复证据：

```bash
# S_CONFIG/S_STATE 指向已到冻结 op95 故障点的同一配置和控制目录。
./s_session/launch_s.sh service stop-s --signal KILL --config "$S_CONFIG" --state-dir "$S_STATE"
./s_session/launch_s.sh service recover --new-epoch 2 --clock-event-id recover-001 \
  --config "$S_CONFIG" --state-dir "$S_STATE"
./s_session/launch_s.sh service start-s --writer-epoch 2 --config "$S_CONFIG" --state-dir "$S_STATE"
```

## 冻结 fixture、完整运行与比较

`tests/fixtures/tb01c/` 的 `MANIFEST.json` 绑定原始输入，`RUNTIME-MANIFEST.json` 绑定 runtime plan、完整 `messages.jsonl`、profile、clock plan 和 Q 资源原字节。其 `NOT_RUN`/`FROZEN_INPUT_BEFORE_EXECUTION_NOT_ACCEPTANCE` 是输入的名分，不是运行结果。正式 driver 开始前再次核长度/SHA，复制候选源码、二进制、配置与输入原件，结束时检查源未变化。

冻结轨迹为 160 个输入，每 500ms 提供一个；op0–94 后，在 op95 的 after_begin 注入真实 S SIGKILL，恢复到 writer epoch2；op96–159 继续输入。Q 重启在线程首次观察到 generation≥109 时触发，独立发送线程继续，Q epoch 变为 2。最后一个 offer 后最多排空 30s。实际触发时刻与观察 generation 留在诊断记录，不伪造精确 Commit 屏障。

使用上面尚未启动的 `a/b` 配置顺序运行；`evidence` 与目标数据库必须不存在。每轮结束后显式停止其服务。失败保留全部原件和进程记录，按相同控制目录查状态/停止，不覆盖重跑。

```bash
"$S_PYTHON" s_session/tests/tb01c_runtime.py \
  --config "$S_RUN_ROOT/a/config.json" --state-dir "$S_RUN_ROOT/a/control" \
  --output "$S_RUN_ROOT/a/evidence"
./s_session/launch_s.sh service stop \
  --config "$S_RUN_ROOT/a/config.json" --state-dir "$S_RUN_ROOT/a/control"
"$S_PYTHON" s_session/tests/tb01c_runtime.py \
  --config "$S_RUN_ROOT/b/config.json" --state-dir "$S_RUN_ROOT/b/control" \
  --output "$S_RUN_ROOT/b/evidence"
./s_session/launch_s.sh service stop \
  --config "$S_RUN_ROOT/b/config.json" --state-dir "$S_RUN_ROOT/b/control"
"$S_PYTHON" s_session/tests/tb01c_compare.py \
  "$S_RUN_ROOT/a/evidence/semantic-core.jsonl" "$S_RUN_ROOT/b/evidence/semantic-core.jsonl" \
  --expected-records 495 > "$S_RUN_ROOT/COMPARE.json"
"$S_PYTHON" s_session/tests/tb01c_load_report.py \
  --events "$S_RUN_ROOT/a/evidence/events.jsonl" --output "$S_RUN_ROOT/a/LOAD.json"
"$S_PYTHON" s_session/tests/tb01c_load_report.py \
  --events "$S_RUN_ROOT/b/evidence/events.jsonl" --output "$S_RUN_ROOT/b/LOAD.json"
```

495 条包括 160 个实际输送结果、160 个最终持久收据、160 代完整历史记录，以及其余 13 张权威表、schema 和故障语义。批次/Delta 全表必须与 160 代引用闭合；BLOB 以完整 base64、字节长度、SHA 和 SQLite 类型保全。比较器不排除已采集语义字段、不重命名身份；缺字段、类型、数组次序、中间历史或 `first_known` 差异仍为差异。

| 输出 | 含义 |
|---|---|
| runtime `PROCESS_TRAJECTORY_CAPTURED_REQUIRES_COMPARISON_AND_REVIEW` | 已完成该 driver 声明的采集，仍需完整比较、消费者和独立审阅 |
| compare `PASS` / exit0 | 两份输入恰为要求的 495 条且逐字段相同；不自行证明采集范围完整 |
| compare `FAIL` / exit1；`NOT_VERIFIED` / exit2 | 实际差异；或输入无法按合同完整比较 |
| load `OBSERVED_BOUNDED_PROGRESS_REQUIRES_TRAJECTORY_AND_CONSUMER_REVIEW` | 仅在实际采样窗口内记录负载进展，不是生产容量或 C PASS |

运行 driver 不覆盖 GUI、v2 多页/Watch/断连/积压、Q 完整性扰动，也不会因为输出目录存在或进程退出 0 就补齐这些验收。

## HTTP 原件采集入口

`tb01c_http_capture.cjs` 对真实 loopback HTTP 发请求，使用生产 `Client` 做完整分页/提交，并保存每个请求、响应、metadata、命令 RESULT 和完整候选。它是独立消费者入口，不启动或停止 S/Q，也不调节 160 条输入节奏。下例要求已有 Q 位于该端口；输出目录必须全新，stdin 每条 JSON 都要以 LF 结束。同一 helper 内相同 `client` 保留已提交 cursor。

```bash
node s_session/tests/tb01c_http_capture.cjs \
  --base-url http://127.0.0.1:18787 --output-dir "$S_RUN_ROOT/http-example" <<'JSONL'
{"id":"initial","client":"reader","op":"load","mode":"RecomputedWithRevision","slowPageDelayMs":1000}
{"id":"next","client":"reader","op":"watch"}
JSONL
```

这是入口示例；立即连续发送两命令不会证明断连增长或更正。指定历史读可用 `{"id":"known","client":"history","op":"load","mode":"AsKnown","generation":"96"}`，前提是实际库已有该代。固定当前投影也可用 `mode:"RecomputedWithRevision", generation:"96"`。`rawWatch` 要显式给出实际 `cursor`、`client_id`、`max_batches`（字符串）；`snapshotToken` 的 `token` 必须取自真实分页回包，不手编 token。

每命令最多 30s（含 discovery/页间等待/读取），每响应含头最多 8MiB、HTTP 头 16KiB，每命令请求+响应累计 64MiB，输入帧 1MiB，最多 4 个 client、8 个排队命令和 4096 个请求。EOF、截断、超界和失败原件均保留。`captured_validated_commit` 表示生产 Client 安装了该完整候选；`captured_envelope_validated` 只验证 raw 回包的公共头/请求身份，`full_state_installed=false`。逐命令 `not_verified` 可能与 helper 正常退出并存，必须读每份 RESULT；采集成功不等于语义验收。

任何分页中断都保持未覆盖；不能用另一轮较长的前缀或完整运行轨迹的比较 PASS 补成该消费者成功。本 README 不宣布未完成的 C/HTTP 跨更正补验通过。

## 资源边界

以下为当前冻结 TestOnly 配置值；改值意味着改变受测资源条件，不能用更大预算代替原条件下的证据。

| 组件 | 当前限制 |
|---|---|
| S | 帧 1MiB；读 2s、写 1s；actor 回执等待 10s；队列 32；连接 48；审计源缓存 256MiB；交付窗口 8 代 |
| Q 捕获/审计 | 新鲜捕获 256MiB/2s；核验 5s；2 个 worker、排队 8；2 个 image、总保留缓存 256MiB |
| Q 网络/客户 | 帧 1MiB；pending connection 16；读 2s/写 1s；回包 8MiB；原子批 4MiB；poll 500ms |
| Q 分页/Watch | 每页最多 31；每次 Watch 最多 4 批；16 个客户租约、每租约 10s；每客户待交付最多 4 批 |

Q 每个变化版本新鲜捕获 schema、全部 raw（含 pending）、独立索引与控制数据再核验；缓存预算计入保留 proof 和 memo。缓存大小是保留对象/源字节预算，**不是进程 RSS 上限**，解码和审计仍有临时内存。资源拒收/Gap 局限于客户或请求，不删除永久历史、不停止 S；`delivery_floor` 是交付窗口，不是历史删除线。旧 snapshot token 由固定投影摘要约束；新捕获摘要变化本身不表示旧 cut 过期。Watch 的 `next_cursor` 是实际交付位置，`observed_head` 可能更前，不能互换。

## 既有验证入口

下面分别验证单位行为、隔离库整合及 Rust 门；需要二进制的用例使用自建临时库。它们不替代以上真实 160 条双进程轨迹或独立消费者验收；不要在正在取证的源码上同时修改或重建候选。

```bash
"$S_PYTHON" -m unittest discover -s s_session/tests -p 'test_tb01c_*.py' -v
node s_session/tests/test_tb01c_browser.cjs
"$S_PYTHON" s_session/tests/tb01c_query_tests.py --binary "$S_BINARY"
"$S_PYTHON" s_session/tests/tb01c_query_v2.py --binary "$S_BINARY"
cargo test --manifest-path rust/Cargo.toml --release --features s_session \
  --bin s_structure_session
```

## v1 接口与旧启动路径

原 v1 Rust 接口保留（`--writer-epoch` 的写入示例显式给 1）：

```bash
"$S_BINARY" init --db "$S_RUN_ROOT/legacy.sqlite" --session legacy-testonly \
  --catalog s_session/catalog/signed-catalog.json
# accept 的输入 session/profile 必须属于该库；使用自己的具名输入与 profile。
"$S_BINARY" accept --db "$S_RUN_ROOT/legacy.sqlite" --input "$S_INPUT" \
  --profile "$S_PROFILE" --writer-epoch 1
"$S_BINARY" advance --db "$S_RUN_ROOT/legacy.sqlite" --writer-epoch 1
"$S_BINARY" catalog --db "$S_RUN_ROOT/legacy.sqlite"
"$S_BINARY" snapshot --db "$S_RUN_ROOT/legacy.sqlite"
"$S_BINARY" meta --db "$S_RUN_ROOT/legacy.sqlite"
```

旧 launcher 保留 `--input <输入.json> --profile <profile.json>`、显式 `--testonly`、`--build`、`--bin`、`--python` 以及 `stop --port`。启动 Q 必须同时显式提供 `--resource-config` 与 `--producer-epoch`，`--testonly` 也不例外；资源与 epoch 在构建、reset 或写库之前预校验。使用已构建二进制的旧一键入口如下（省略 `--bin` 并加 `--build` 可按 launcher 的构建流程执行）：

```bash
./s_session/launch_s.sh --testonly --bin "$S_BINARY" --python "$S_PYTHON" \
  --db "$S_RUN_ROOT/legacy-quickstart.sqlite" --port 18790 \
  --resource-config s_session/tests/fixtures/tb01c/query-resources.json --producer-epoch 1
curl --fail http://127.0.0.1:18790/api/catalog
curl --fail http://127.0.0.1:18790/api/snapshot
./s_session/launch_s.sh stop --port 18790
```

已存在的 v1 库也可显式启动 Q（前台进程以 Ctrl-C 停止）：

```bash
"$S_PYTHON" s_session/s_readonly_server.py \
  --db "$S_RUN_ROOT/legacy.sqlite" --port 18791 --browser s_session/browser/index.html \
  --resource-config s_session/tests/fixtures/tb01c/query-resources.json --producer-epoch 1
# 在另一终端读取：http://127.0.0.1:18791/api/catalog 或 /api/snapshot
```

## 目录

- `launch_s.sh` / `s_service_control.py`：正式入口与 S/Q 独立生命周期控制。
- `s_socket_client.py`：一次有界输送与公共响应头校验；不自动重发业务。
- `s_readonly_server.py` / `s_query.py` / `s_query_integrity.py`：只读 HTTP、分页/Watch、共享全域完整性核验。
- `browser/index.html` / `browser/tb01c-client.js`：同源只读浏览器和完整候选提交；精确整数保留为字符串。
- `catalog/signed-catalog.json`：从签署字节派生的已签 116 项目录；profile 和受测规则不能随演示自行换名。
- `profiles/` / `inputs/`：v1 具名 TestOnly profile 与原始四分支、追加、大整数和冲突输入。
- `tests/fixtures/tb01c/`：冻结 v2 输入/clock/资源/运行计划；`tests/tb01c_runtime.py`、`tb01c_compare.py`、`tb01c_load_report.py`、`tb01c_http_capture.cjs` 分别负责轨迹采集、完整比较、负载事实与 HTTP 原件。
