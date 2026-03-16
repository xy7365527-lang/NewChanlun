# Codex diagnose — 2026-03-16 09:50:50 UTC

## 元数据

- **mode**: diagnose
- **subject**: traversal worker 静默崩溃：269步后进程消失，无OOM无异常日志，重启后仍崩溃
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-16 09:50:50 UTC
- **context-file**: tmp/codex-diagnose-silent-crash.md

## Prompt

## 诊断目标

traversal worker 静默崩溃：269步后进程消失，无OOM无异常日志，重启后仍崩溃

## 上下文

# Codex 诊断上下文：traversal worker 静默崩溃（v254 可变 Graph）

## 任务

诊断 traversal worker 子进程在步数 269（300万顶点，1100万边，内存 1.6GB/62GB）后静默消失的根因。

**关键症状**：
- 没有 OOM（内存仅 1.6GB，机器 64GB）
- 没有 Python 异常日志
- 没有 segfault
- 重启后 worker 又崩溃——status API 无响应
- worker 进程直接消失，主进程 HTTP 服务继续运行（但 status 无更新）

**不要提修复方案。只做根因诊断和排序。**

---

## 系统架构（v254）

- **Graph**：纯 Python 可变对象。`add_edge` 原地 append，`add_vertex` 原地修改，返回 `self`
- **k_full / k_active**：共享同一 Graph 对象（`self.k_full = self.k_active`），然后 `self.k_active = self.k_active.copy()`（浅拷贝，独立拷贝 _edges/_adj_out/_adj_in/_active_ids/_edge_keys）
- **multiprocessing.Process**（fork 模式）：traversal worker 是子进程
- **Manager dict**：status_dict / topology_cache 是 SyncManager 管理的共享 dict
- **worker daemon=True**：`worker = Process(..., daemon=True)`

规模：300万顶点，1100万边，内存 1.6GB（正常）

---

## 关键代码片段

### 1. Graph.copy()（engine.py:263）

```python
def copy(self) -> "Graph":
    """Return an independent shallow copy of this graph."""
    g = Graph.__new__(Graph)
    g._vertices = dict(self._vertices)           # 新 dict，共享 Vertex 对象（frozen dataclass）
    g._edges = list(self._edges)                 # 新 list，共享 Edge 对象（frozen dataclass）
    g._adj_out = {k: list(v) for k, v in self._adj_out.items()}   # 新 dict + 新 list
    g._adj_in = {k: list(v) for k, v in self._adj_in.items()}     # 新 dict + 新 list
    g._active_ids = set(self._active_ids)        # 新 set（str，不共享）
    g._edge_keys = set(self._edge_keys)          # 新 set（tuple，不共享）
    return g
```

**关键**：300万顶点 + 1100万边时，`copy()` 的成本：
- `dict(self._vertices)`：复制 300万个 key-value 对
- `list(self._edges)`：复制 1100万个 Edge 引用
- `_adj_out/_adj_in` 内层 list 展开：O(E) 迭代

### 2. daemon.py __init__（daemon.py:310）

```python
self.k_full = self.k_active     # k_full 和 k_active 是同一对象
self.k_active = self.k_active.copy()  # k_active 得到独立拷贝
```

初始化时在小图上 copy()，无问题。但后续：

### 3. daemon._step()（daemon.py:1187）

```python
def _step(self) -> None:
    _need_diff = self._persist or self._shared_layer is not None
    if _need_diff:
        pre_vid_count = len(self.k_full._vertices)
        pre_edge_count = len(self.k_full._edges)
        pre_vid_keys = set(self.k_full._vertices.keys())  # ← 300万顶点时：复制 300万个 key！
        pre_active_statuses = {
            vid: v.status for vid, v in self.k_active.vertices.items()  # ← 遍历 k_active 所有顶点
        }

    log = self.engine.run_step()  # traversal，原地修改 k_active/k_full

    self.k_active = self.engine.k_active
    self.k_full = self.engine.k_full
    self.terrain = self.engine.terrain

    # ... 后续计算 diff ...

    if _need_diff:
        for vid in self.k_full._vertices:
            if vid not in pre_vid_keys:    # ← O(1) set lookup，但构建 pre_vid_keys 是 O(V)
                new_vids.add(vid)
        new_edges_list = self.k_full._edges[pre_edge_count:]  # ← tail slice，O(new_E)
```

**关键问题**：`pre_vid_keys = set(self.k_full._vertices.keys())`
- 300万顶点时：每步创建一个 300万元素的 set（Python str 对象）
- 每个 Python str 引用：64 bytes（Python 对象头部）
- 300万 × 64 bytes ≈ 192MB/步（仅 set 本身，不含字符串内容）
- 但这是步内局部变量，步结束后应被 GC

### 4. traversal worker 主循环（daemon_multiproc.py:263）

```python
while True:
    try:
        daemon._step()
        step_count += 1

        _update_status(daemon, status_dict)    # 向 Manager dict 写入

        if step_count % topology_refresh_interval == 0:
            _update_topology(daemon, topology_cache, _topology_json)  # 序列化整个 topology
            _update_api_caches(...)

        if step_count % 20 == 0:
            cis = getattr(daemon, '_cross_instance_sync', None)
            if cis is not None:
                try:
                    cis.sync()
                except Exception:
                    pass

        _drain_feed_queue(daemon, feed_queue, event_queue)
        _drain_present_queue(daemon, present_queue, present_result_queue)
        time.sleep(0.01)

    except KeyboardInterrupt:
        break
    except Exception as exc:
        # Log error but keep running
        print(f"[traversal-worker] error: {exc}", file=sys.stderr)
        traceback.print_exc(file=sys.stderr)
        time.sleep(1)
```

**关键问题**：`except Exception` 捕获所有异常，打印后继续。但是：
- **SystemExit** 不是 `Exception` 的子类——`except Exception` 不捕获 `SystemExit`
- **MemoryError** 是 `Exception` 子类——应被捕获并继续（但 1.6GB 下不应 OOM）
- **KeyboardInterrupt** 单独捕获
- **SIGKILL / SIGTERM** 信号：不产生 Python 异常，直接终止进程

### 5. worker 进程的 daemon=True 属性（daemon_multiproc.py:892）

```python
worker = Process(
    target=_traversal_worker,
    args=(...),
    name="traversal-worker",
    daemon=True,     # ← daemon=True！
)
```

**daemon=True 的语义**：
> A process can be marked as a daemon process by setting its daemon attribute to True. The daemon process can NOT create child processes. If the main process exits, its daemon processes are terminated automatically.

关键推论：
- 如果主进程（HTTP 服务）崩溃或退出 → daemon worker 自动终止
- 如果主进程的 Manager（SyncManager）崩溃 → worker 可能因写入共享 dict 失败而终止

### 6. SyncManager 共享 dict 的写入风险（daemon_multiproc.py:307）

```python
def _update_status(daemon, status_dict: dict) -> None:
    try:
        s = daemon.status()
        status_dict["total_steps"] = s["total_steps"]        # ← IPC via Manager proxy
        status_dict["total_events"] = s["total_events"]      # ← IPC via Manager proxy
        # ... 等 14 个字段 ...
        status_dict["_updated_at"] = time.time()
    except Exception:
        pass  # 静默吞掉所有异常
```

每步写入 Manager dict 都是 IPC（通过 Manager 进程的 socket）。
如果 Manager 进程崩溃：
- `status_dict[...] = ...` 抛出异常（ConnectionRefusedError 或 BrokenPipeError）
- `except Exception: pass` 静默吞掉
- 下一步继续——但每步都在 `except Exception: pass` 路径执行

### 7. _update_topology 在大图时的序列化负担（daemon_multiproc.py:364）

```python
def _update_topology(daemon, topology_cache: dict, topology_json_fn) -> None:
    try:
        topo = topology_json_fn(daemon)              # ← 序列化整个 topology
        topology_cache["json"] = json.dumps(topo, ensure_ascii=False)  # ← 传给 Manager dict
        topology_cache["_updated_at"] = time.time()
    except Exception:
        pass
```

300万顶点 / 1100万边时，`topology_json_fn` 产出的 JSON 大小？
- 如果 topology 包含所有节点/边：可能数 GB 的 JSON
- `json.dumps` 一个 GB 级别的 dict：极慢且内存峰值巨大
- 通过 Manager dict 传输：序列化后再通过 IPC 发送

### 8. traversal.py TraversalEngine 初始化（traversal.py:123）

```python
self.k_full = graph
self.k_active = graph.copy()  # ← 300万顶点 + 1100万边时：O(V+E) 拷贝
```

---

## 诊断假设

### 候选 A：topology_json 序列化 OOM（无 OOM kill log 也可能崩溃）

- `_update_topology` 每 50 步执行一次
- 300万顶点 + 1100万边时，topology JSON 可能达到 GB 级
- `json.dumps(topo)` 在内存中创建巨大字符串
- 通过 Manager IPC 传输：再次序列化（pickle）该字符串传给 Manager 进程
- 峰值内存可能触发 Linux OOM killer（但 kernel 可能选择 kill Manager 进程而非 worker）
- 如果 Manager 进程被 kill：worker 的所有 IPC 调用失败 → Manager 重新 fork 还是直接终止？

### 候选 B：_update_status 中的 daemon.status() 在大图时 O(V+E)

```python
# daemon.py
def status(self) -> dict:
    active = self.k_active.active_vertex_ids()  # 遍历所有顶点
    ...
```
如果 `active_vertex_ids()` 返回 300万元素的集合，`daemon.status()` 本身就很重。

### 候选 C：pre_vid_keys 构建在 300万顶点时导致内存峰值触发 OOM kill

每步 `set(self.k_full._vertices.keys())` = 300万 str 引用的新 set。
每步 192MB 峰值（见上）。Python GC 若来不及回收，步与步之间积压：
- 步 N 时 pre_vid_keys_N 还未 GC，步 N+1 又创建 pre_vid_keys_N+1
- 积压 K 步 × 192MB = K × 192MB

但步内局部变量在 CPython 中通常在函数返回时立即 decref。

### 候选 D：SyncManager 连接超时导致 worker 进程被 SIGKILL

multiprocessing.Manager 内部有心跳机制。如果 worker 长时间阻塞（如 topology 序列化），Manager 可能判断连接超时并关闭 socket。下次 worker 写入时抛出异常。`except Exception: pass` 吞掉。但如果这导致 Manager 本身退出，主进程退出，daemon worker 自动被 kill（daemon=True）。

### 候选 E：block_topology_persistence 的 JSONL 写入在 300万顶点时无法完成

如果 `_persist.append_vertex/append_edge` 写入非常慢（大图 JSONL flush 延迟），每步持久化阻塞，积累到某个 timeout 触发系统级 kill。

### 候选 F：run_step() 内部无限循环 / 死锁

traversal.py 中的某个循环在特定图状态下不终止（如 _nothing_streak 无法触发 EXPLORATION_INTERVAL 跳跃），导致 worker 永久阻塞，主进程超时 → Manager 终止 → daemon worker 终止。

---

## Codex 诊断任务

请基于以上代码证据：

1. **识别"静默消失"的机制**：哪种 Python 或系统机制能让进程消失但不产生错误日志？
   - `except Exception: pass` 吞掉了什么？
   - daemon=True 进程在何种条件下被 kill？
   - SIGTERM/SIGKILL 是否可能由 Manager 超时触发？

2. **排序候选根因**（主因 / 次因 / 次次因）：
   - 候选 A：topology_json 序列化 OOM kill Manager → daemon worker 连锁终止
   - 候选 B：daemon.status() 每步 O(V) 导致步速急剧降低 → Manager 超时
   - 候选 C：pre_vid_keys 每步 300万元素 set → 内存压力
   - 候选 D：Manager 心跳超时 → worker 连锁终止
   - 候选 E：JSONL 写入阻塞 → 系统 kill
   - 候选 F：run_step() 死锁

3. **代码中遗漏的静默崩溃路径**：
   - 是否有任何路径产生 `SystemExit`（被 `except Exception` 漏过）？
   - `daemon._step()` 的 `except Exception` 是否有任何吞掉后会改变后续行为的情形？

4. **为什么重启后仍然崩溃**：
   - 这指向状态无关的崩溃（如代码路径问题）还是状态相关（如 JSONL 文件损坏）？


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

按你要求，仅做根因诊断与排序，不给修复方案。

---

## 结论（先给排序）

**主因（最高概率）**：**候选 C**（`_step` 每步全量快照：`pre_vid_keys` + `pre_active_statuses`）  
**次因**：**候选 B**（`_update_status -> daemon.status()` 每步全量扫描/集合构造）  
**次次因**：**候选 A**（周期性 `topology_json` 全量序列化，可能触发峰值内存杀进程）  
**低概率/非直接致死**：D、E、F

---

## 1) “静默消失”机制判定（可证据链）

### 失败现象
- worker 进程直接消失
- 无 Python 异常日志
- 无 segfault 迹象
- 主 HTTP 还活着，但 status 无更新/无响应

### 直接机制（可由代码直接推出）
worker 主循环只捕获：
- `KeyboardInterrupt`
- `Exception`

```python
except Exception as exc:
    print(...)
```

因此**不会被记录**的终止路径只有两类：
1. **`BaseException` 非 Exception 分支**（典型：`SystemExit`）  
2. **进程被信号直接终止**（`SIGTERM`/`SIGKILL`），Python 无 traceback

> 这就是“静默消失”的唯一代码级解释路径。  
> 普通 `Exception`（含 `MemoryError`）本应打印日志，不符合现象。

---

## 2) 候选根因排序与否定

## 主因：候选 C（`_step` 全量快照内存峰值）
关键行：

```python
pre_vid_keys = set(self.k_full._vertices.keys())      # 300万级 set 新建
pre_active_statuses = {vid: v.status for ...}         # 全量 dict 新建
```

这是**每步**执行（只要 `_need_diff` 为真），不是周期任务。  
在 300 万顶点规模下，属于**稳定触发的 O(V) 大对象分配**，最符合“跑到某步后 deterministic 消失”。

### 直接原因
- 每步创建超大临时 set/dict，产生瞬时内存峰值。
- 若触发 OS 级 kill（`SIGKILL`），不会进入 Python `except Exception`，所以“无异常日志”。

### 根本原因
- 在热路径上采用“全量快照 diff”实现，复杂度/内存模型与大图规模不匹配。
- 属于实现层面的规模化错误，不是定义冲突。

---

## 次因：候选 B（`daemon.status()` 每步 O(V)）
`_update_status` 每步调用 `daemon.status()`；你给出的 `status()` 里有：

```python
active = self.k_active.active_vertex_ids()  # 可能是全量集合
```

若这里返回/构造大集合，也会形成额外峰值与 CPU 压力。  
它与 C 叠加，增加“某一步骤被 SIGKILL”的概率。

### 直接原因
- 每步状态采集本身不是 O(1)，而是 O(V) 级别。

### 根本原因
- 可观测性接口（status）设计为全量统计，侵入主循环关键路径。

---

## 次次因：候选 A（topology JSON 全量序列化）
`_update_topology` 确实是高风险（`json.dumps` + Manager IPC）。  
但它是**按 interval 触发**，而非每步。若 interval=50，则 269 不是刷新点，和“269 消失”时间对齐较弱。

### 判定
- 作为系统不稳定因素成立；
- 作为“这次 269 静默消失”的**第一触发点证据不足**，故排在 C/B 后。

---

## 低概率或非直接致死路径

### 候选 D（Manager 心跳超时）
- Manager 连接坏掉会在 proxy 调用抛异常；
- 这些异常在 `_update_status/_update_topology` 被 `except Exception: pass` 吞掉；
- **会导致静默失联，但不会直接杀死 worker 进程**。

因此 D 更像“观测失真放大器”，不是直接致死机制。

### 候选 E（JSONL 写阻塞）
- 解释“变慢/卡住”可以；
- 解释“进程消失且无日志”不强，除非外部 watchdog/信号介入（未见证据）。

### 候选 F（run_step 死锁）
- 死锁应表现为“进程仍在但不前进”，不是“进程消失”。
- 与现象不匹配。

---

## 3) 代码里遗漏的静默崩溃路径

1. **`SystemExit` 漏捕获**  
   主循环只捕获 `Exception`，`SystemExit` 会直接结束进程，无日志。

2. **信号终止无捕获**  
   `SIGKILL/SIGTERM` 直接退出进程，不走 Python 异常链。

3. **异常吞噬导致前置信号丢失**  
   `_update_status/_update_topology` 的 `except Exception: pass` 会吞掉 Manager 断链等关键错误，导致你只看到“最后进程没了”，看不到前驱异常。

> `daemon._step()` 在你给出的片段里没有对应的全局吞异常；当前“吞异常改变行为”主要发生在 status/topology 更新函数。

---

## 4) 为什么重启后仍崩溃

这更指向**规模触发的确定性路径**，不是一次性脏状态。  
即：图规模达到某阈值后，C/B 路径的每步全量分配再次出现，故重启后复现。

- 若是“状态无关代码路径问题”：会在相近规模/相近步数重复（与你现象一致）。
- 若是“持久化文件损坏”：更常见表现是固定读取/解析异常；而你的现象是无异常日志的直接消失，不匹配优先级较低。

---

## 类型判定

- **主因 C / 次因 B / 次次因 A**：均为**实现错误**（实现复杂度与内存行为失控，且观测链被吞异常掩盖）。
- **非定义冲突**：不存在“定义 A 与定义 B 不可同时满足”的语义冲突。
