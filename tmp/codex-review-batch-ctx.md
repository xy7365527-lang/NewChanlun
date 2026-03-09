# Codex Review Context: analyze_levels_batch() in scripts/brn_level_analysis.py

## 审查目标

`scripts/brn_level_analysis.py` 中新增的 `analyze_levels_batch()` 函数（+245行），
以及辅助函数 `_recursive_levels()`。

## 参考实现：RecursiveOrchestrator（streaming 版）

### RecursiveOrchestrator.process_bar() 调用链（`src/newchan/orchestrator/recursive.py`）
```python
def process_bar(self, bar: Bar) -> RecursiveOrchestratorSnapshot:
    bi_snap = self._bi_engine.process_bar(bar)
    seg_snap = self._seg_engine.process_snapshot(bi_snap)
    zs_snap = self._zs_engine.process_segment_snapshot(seg_snap)
    move_snap = self._move_engine.process_zhongshu_snapshot(
        zs_snap, num_segments=len(seg_snap.segments),
    )
    bsp_snap = self._bsp_engine.process_snapshots(
        move_snap, zs_snap, seg_snap,
    )
    recursive_snaps = self._recursive_stack.process_level1_move_snapshot(move_snap)
    ...
```

### RecursiveStack.process_level1_move_snapshot()（`src/newchan/core/recursion/recursive_stack.py`）
```python
def process_level1_move_snapshot(self, move_snap):
    snapshots = []
    current_move_snap = move_snap
    current_level = 1

    while current_level < self._max_levels:
        next_level = current_level + 1
        # 懒创建引擎
        if next_level not in self._engines:
            self._engines[next_level] = RecursiveLevelEngine(
                level_id=next_level, stream_id=self._stream_id
            )
        engine = self._engines[next_level]
        snap = engine.process_move_snapshot(current_move_snap)
        snapshots.append(snap)

        if len(snap.moves) < 3:
            break

        current_move_snap = MoveSnapshot(
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            moves=snap.moves,
            events=snap.move_events,   # ← 注意：events 来自 snap.move_events
        )
        current_level = next_level

    return snapshots
```

### RecursiveLevelEngine.process_move_snapshot()（`src/newchan/core/recursion/recursive_level_engine.py`）
```python
def _compute_zhongshus(self, move_snap):
    settled_moves = [m for m in move_snap.moves if m.settled]
    components = adapt_moves(settled_moves, level_id=self._level_id - 1)  # ← level_id - 1
    return zhongshu_from_components(components), components
```

## 关键差异：adapt_moves 的 level_id 参数

**RecursiveLevelEngine** 传入 `level_id=self._level_id - 1`：
- level=2 引擎调用 `adapt_moves(moves, level_id=1)`
- level=3 引擎调用 `adapt_moves(moves, level_id=2)`
- 以此类推

**_recursive_levels() 批量版**：
```python
def _recursive_levels(move_snap, max_levels=6):
    current_level = 1
    while current_level < max_levels:
        next_level = current_level + 1
        settled_moves = [m for m in current_move_snap.moves if m.settled]
        components = adapt_moves(settled_moves, level_id=current_level)  # ← current_level
        ...
```

注意：
- RecursiveLevelEngine(level_id=2) 调用 `adapt_moves(moves, level_id=2-1=1)`
- _recursive_levels 在第一轮（current_level=1, next_level=2）调用 `adapt_moves(moves, level_id=1)`
- RecursiveLevelEngine(level_id=3) 调用 `adapt_moves(moves, level_id=3-1=2)`
- _recursive_levels 在第二轮（current_level=2, next_level=3）调用 `adapt_moves(moves, level_id=2)`

两者 level_id 值相同。

## adapt_moves 函数签名（`src/newchan/a_level_protocol.py`）
```python
def adapt_moves(moves: list[Move], level_id: int) -> list[MoveAsComponent]:
    return [
        MoveAsComponent(_move=m, _component_idx=i, _level_id=level_id)
        for i, m in enumerate(moves)
    ]
```
level_id 存储到 MoveAsComponent._level_id，在 zhongshu_from_components 中：
```python
level_id = completed[0].level_id + 1
```
所以中枢的 level_id = adapter 的 level_id + 1。

## MoveSnapshot 构造的差异

**RecursiveStack** 向上传递时：
```python
current_move_snap = MoveSnapshot(
    bar_idx=snap.bar_idx,
    bar_ts=snap.bar_ts,
    moves=snap.moves,
    events=snap.move_events,  # ← 实际事件列表
)
```

**_recursive_levels** 向上传递时：
```python
current_move_snap = MoveSnapshot(
    bar_idx=snap.bar_idx,
    bar_ts=snap.bar_ts,
    moves=curr_moves,
    events=[],  # ← 空列表
)
```

## BiEngineSnapshot 构造

**RecursiveOrchestrator** 中 BiEngineSnapshot 由 BiEngine.process_bar() 自动创建，包含所有字段。

**analyze_levels_batch()** 中手动构造：
```python
bi_snap = BiEngineSnapshot(
    bar_idx=bar_idx, bar_ts=bar_ts, strokes=all_strokes,
    events=[], n_merged=len(df_merged), n_fractals=len(fractals),
)
```
`events=[]` — batch 模式没有增量事件，这是预期行为。

## moves_from_zhongshus 调用

**analyze_levels_batch() L1 层**：
```python
moves = moves_from_zhongshus(zhongshus, num_segments=len(segments))
```
这是 L1 层（`newchan.a_move_v1`）的 moves，不是 level 递归层的。

**递归层**：
```python
curr_moves = moves_from_level_zhongshus(curr_zhongshus)
```
这是 level 层（`newchan.a_zhongshu_level`）的 moves。

两者正确分离。

## RecursiveLevelSnapshot 的 level_id 赋值

**RecursiveLevelEngine** 产生的 snapshot：`level_id=self._level_id`（引擎自身的 level_id）。

**_recursive_levels** 产生的 snapshot：`level_id=next_level`。

两者相同：引擎的 level_id = next_level（从 current_level+1 创建）。

## 边界情况检查

1. `bars=[]`：第347行有 `if not bars: return {"error": "no bars"}`。
2. 单根 bar：merge_inclusion、fractals_from_merged、strokes_from_fractals 均可处理单行。
3. 无笔：`all_strokes=[]` → `segments=[]` → `zhongshus=[]` → `moves=[]` → `_recursive_levels` 内 `settled_moves=[]` → `components=[]` → `curr_zhongshus=[]` → `len(curr_moves)=0 < 3` → 立即 break。

## 性能分析

**analyze_levels()** 对每根 bar 调用 `orch.process_bar(bar)`，每次都重跑全量管线：
- merge_inclusion O(n)
- fractals_from_merged O(n)
- strokes_from_fractals O(n)
- 合计每 bar O(n)，总体 O(n²)

**analyze_levels_batch()** 一次性调用全量管线：
- merge_inclusion 一次 O(n)
- fractals_from_merged 一次 O(n)
- strokes_from_fractals 一次 O(n)
- 合计 O(n)，正确。

但注意：BiEngine.process_bar() 在每次调用时重建 DataFrame（`_build_df`），这是 O(n²) 的来源。
batch 模式绕过了 BiEngine，直接调用纯函数，因此真正达到 O(n)。

## 审查要点（来自任务说明）

1. merge_inclusion/fractals/strokes 纯函数调用是否正确
2. BiEngineSnapshot 构造是否保留了下游引擎需要的所有字段
3. _recursive_levels() 纯函数递归是否正确模拟了 RecursiveStack 行为
4. 递归算法是否被完整保留（adapt_moves → zhongshu_from_components → moves_from_level_zhongshus）
5. 性能是否真的从 O(n²) 降到了 O(n)
6. 边界情况：空数据、单根bar、无笔等

## 待审查代码（完整）

### _recursive_levels()（brn_level_analysis.py:300-337）

```python
def _recursive_levels(
    move_snap: MoveSnapshot, max_levels: int = 6,
) -> list[RecursiveLevelSnapshot]:
    """Run recursive stack logic without engine state — pure function chain."""
    snapshots: list[RecursiveLevelSnapshot] = []
    current_move_snap = move_snap
    current_level = 1

    while current_level < max_levels:
        next_level = current_level + 1
        settled_moves = [m for m in current_move_snap.moves if m.settled]
        components = adapt_moves(settled_moves, level_id=current_level)
        curr_zhongshus = zhongshu_from_components(components)
        curr_moves = moves_from_level_zhongshus(curr_zhongshus)

        snap = RecursiveLevelSnapshot(
            bar_idx=current_move_snap.bar_idx,
            bar_ts=current_move_snap.bar_ts,
            level_id=next_level,
            zhongshus=curr_zhongshus,
            moves=curr_moves,
            zhongshu_events=[],
            move_events=[],
        )
        snapshots.append(snap)

        if len(curr_moves) < 3:
            break

        current_move_snap = MoveSnapshot(
            bar_idx=snap.bar_idx,
            bar_ts=snap.bar_ts,
            moves=curr_moves,
            events=[],
        )
        current_level = next_level

    return snapshots
```

### analyze_levels_batch() 核心段（brn_level_analysis.py:340-504）

```python
def analyze_levels_batch(bars: list[Bar], stream_id: str = "BRN") -> dict:
    if not bars:
        return {"error": "no bars"}

    # 1. DataFrame
    arr = np.array([[b.open, b.high, b.low, b.close] for b in bars], dtype=np.float64)
    df = pd.DataFrame(arr, columns=["open","high","low","close"],
                      index=pd.DatetimeIndex([b.ts for b in bars], name="time"))

    # 2. Inclusion → Fractals → Strokes
    df_merged, merged_to_raw = merge_inclusion(df)
    fractals = fractals_from_merged(df_merged)
    all_strokes = strokes_from_fractals(
        df_merged, fractals, mode="wide", min_strict_sep=5,
        merged_to_raw=merged_to_raw,
    )

    # 3. Segments
    segments = segments_from_strokes_v1(all_strokes)

    # 4. Zhongshu
    zhongshus = zhongshu_from_segments(segments)

    # 5. Moves
    moves = moves_from_zhongshus(zhongshus, num_segments=len(segments))

    # 6. Divergences + BSP
    divergences = divergences_from_moves_v1(segments, zhongshus, moves, 1)
    bsps = buysellpoints_from_level(segments, zhongshus, moves, divergences, 1)

    # 7. Snapshots
    last_bar = bars[-1]
    bar_idx = len(bars) - 1
    bar_ts = _dt_to_epoch(last_bar.ts)

    move_snap = MoveSnapshot(bar_idx=bar_idx, bar_ts=bar_ts, moves=moves, events=[])

    # 8. Recursive levels
    recursive_snaps = _recursive_levels(move_snap)

    # 9. BiEngineSnapshot (手动构造)
    bi_snap = BiEngineSnapshot(
        bar_idx=bar_idx, bar_ts=bar_ts, strokes=all_strokes,
        events=[], n_merged=len(df_merged), n_fractals=len(fractals),
    )
    ...
    orch_snap = RecursiveOrchestratorSnapshot(...)
    orch_snap.lstar = select_lstar_from_recursive_snapshot(orch_snap, last_bar.close)
    ...
```
