# pytest-fix 报告

## 修复的问题

### 1. `ImportError: cannot import name 'SYMBOL_CATALOG' from 'newchan.data_databento'`

**根因**：两层问题叠加。

- **第一层**：`src/newchan/data_databento.py:19` 在模块顶层 `import databento as db`。当 `databento` 未安装或导入失败时，整个模块无法加载，连带 `SYMBOL_CATALOG`（从 `newchan.core.symbol_catalog` re-export）也不可用。
- **第二层（实际触发原因）**：`tests/test_cli_commands.py` 在模块顶层（第25-30行）注入了一个 stub 模块到 `sys.modules["newchan.data_databento"]`，该 stub 只包含 `fetch_and_cache` 和 `DEFAULT_SYMBOLS`，**不包含 `SYMBOL_CATALOG`**。由于 `test_cli_commands.py` 按字母序先于 `test_data_databento.py` 被 pytest 收集，stub 已经占据了 `sys.modules`，导致后者导入时找不到 `SYMBOL_CATALOG`。

**修复**：
1. `src/newchan/data_databento.py`：将 `import databento as db` 从模块顶层移入 `_get_client()` 函数内部（lazy import）。`databento` 只在实际调用 API 时才需要，模块导入不再依赖它。
2. `tests/test_cli_commands.py`：删除 `data_databento` 的 stub 注入（第23-30行）。lazy import 修复后，真实模块可正常导入，不再需要 stub。

### 2. `pytest.mark.asyncio` 警告（`test_mcp_bridge.py`）

**根因**：`pytest-asyncio` 不在测试依赖中。

**修复**：
1. `pyproject.toml`：在 `[project.optional-dependencies] test` 中添加 `pytest-asyncio>=0.24`。
2. `pyproject.toml`：在 `[tool.pytest.ini_options]` 中添加 `asyncio_mode = "auto"`，消除需要显式标记每个 async 测试的警告。

## 修改的文件

| 文件 | 修改内容 |
|------|---------|
| `src/newchan/data_databento.py` | 删除顶层 `import databento as db`，移入 `_get_client()` 内部 |
| `tests/test_cli_commands.py` | 删除 `data_databento` stub 注入（8行） |
| `pyproject.toml` | 添加 `pytest-asyncio>=0.24` 依赖 + `asyncio_mode = "auto"` |

## 验证结果

```
python -m pytest -m "not slow" --tb=short -q
1494 passed, 8 skipped, 8 deselected, 1 warning in 59.08s
```

0 errors, 0 failures。
