## 现象

CI 的 `test` job（pytest）红在**一条**用例上：

```
tests/test_morse_landscape.py:155: in test_real_data_hard_constraint
    if _relations_is_lfs_pointer(REAL_RELATIONS.parent):
TestRealData.test_real_data_hard_constraint FAILED
（同跑：1 failed, 5004 passed, 106 skipped）
```

根因在 `.github/workflows/ci.yml` 的 LFS 段：`git lfs pull` 那步标了 `continue-on-error: true`（#324 为省 LFS 流量额度而设），失败时 `.chanlun/block-topology/relations.jsonl`（125 MB LFS 对象）**停留为 pointer 文件**。`tests/conftest.py` 的 `real_relations_usable` 守卫本应据此显式 skip 并打印原因——**但本用例没走到那道守卫，而是在自己的 `_relations_is_lfs_pointer` 分支里炸了**。

⟹ 要么是守卫漏了这条用例，要么是该用例的 pointer 分支写法本身有问题。**两条都是本票要查的。**

## 为什么现在才暴露

CI 触发分支 `main-rewritten` 此前**落后主线 545 个提交**，pytest 一直在旧树上跑。2026-07-30 镜像追上（`61ed615393 → 60ff450ea7`）后，这条失败首次可见。成因链见 [#810](https://github.com/xy7365527-lang/NewChanlun/issues/810) 的复核评论。

同批暴露的还有 `cargo fmt --check` 的 6 文件 11 处漂移（已由 `cargo fmt --all` 机械复位）。

## 要做的

1. 先判性质：是 **CI 环境问题**（LFS 没拉下来，本地跑得过）还是**用例本身的问题**（pointer 分支逻辑错）。**本地跑一遍 `pytest tests/test_morse_landscape.py -k real_data_hard_constraint` 即可分开**——本地 LFS 对象在，若本地绿则是前者。
2. 前者 ⟹ 补守卫，让它像 `real_relations_usable` 一样**显式 skip 并打印原因**，不静默红也不静默过。
3. 后者 ⟹ 修用例。
4. **不许用 `-k not ...` 或删用例的方式让 CI 变绿。**

## 验收

CI 上 `test` job 转绿，且若是 skip，日志里能看到显式的 skip 原因（`-rs` 已在命令里）。
