---
title: 票号引用解析器
allowed_prefixes:
  - experiments/pilot1742_demo/
---
在 experiments/pilot1742_demo/ 新建 ticket_refs.py 与 test_ticket_refs.py。

实现 extract_ticket_refs(text: str) -> list[int]：

- 识别 `#123` 形式的票号引用，按首次出现顺序返回去重后的整数列表；
- 忽略 Markdown 行内代码（成对单反引号之间）与围栏代码块（以 ``` 开头的行之间）里的引用；
- `#` 前紧邻字母、数字或下划线时不算引用，例如 `abc#12`、`C#7`；
- 票号是 1 到 7 位十进制数字，后面紧跟字母、数字或下划线时不算，例如 `#12ab`；
- 以 0 开头的数字不算，例如 `#0`、`#012`。

测试只用 Python 标准库 unittest，覆盖上面每条规则，用
`python3 -m unittest discover -s experiments/pilot1742_demo -p 'test_*.py'` 运行。
不得编译 Rust，不得改动写集之外的文件。
