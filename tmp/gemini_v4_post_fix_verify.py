"""Gemini v4 post-fix verification script (new google-genai SDK).

Calls Gemini API to perform heterogeneous verification of v4 corrections.
"""
import os
import sys
from pathlib import Path

# Load .env
env_path = Path("C:/Users/hanju/NewChanlun/.env")
if env_path.exists():
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            os.environ.setdefault(k.strip(), v.strip())

api_key = os.environ.get("GOOGLE_API_KEY") or os.environ.get("GEMINI_API_KEY")
if not api_key:
    print("ERROR: No API key found", file=sys.stderr)
    sys.exit(1)

from google import genai
from google.genai import types

client = genai.Client(api_key=api_key)

ROOT = Path("C:/Users/hanju/NewChanlun")

def read_file(rel_path: str) -> str:
    p = ROOT / rel_path
    return p.read_text(encoding="utf-8")

# ── Read all context files ──
v4_text = read_file("tmp/global_capital_flow_v4.txt")
codex_inquiry = read_file("tmp/codex-inquiry-v4.md")
opus_report = read_file(".chanlun/review-results/v4-post-fix-verification-20260227.md")
dengjia_def = read_file(".chanlun/definitions/dengjia.md")
liuzhuan_def = read_file(".chanlun/definitions/liuzhuan.md")
level_recursion_def = read_file(".chanlun/definitions/level_recursion.md")
equivalence_py = read_file("src/newchan/equivalence.py")
capital_flow_py = read_file("src/newchan/capital_flow.py")
matrix_topology_py = read_file("src/newchan/matrix_topology.py")
a_macd_py = read_file("src/newchan/a_macd.py")

# ── Construct prompt ──
prompt = f"""你是一位数学和金融工程领域的严格审查者。请对以下 v4 修正后的理论文本进行全面审查。

# 审查背景

v4 文本经过以下修正：
1. §1.1 补充比价序列构造原则（逐时刻 A(t)/B(t)）
2. §2.3 守恒措辞修正（乘法恒等式 vs 加法守恒，明确区分两层）
3. §2.4 "可靠"→"紧"，C1-C3 重新定义为操作性判据并显式声明非充分条件
4. §4.1 EMA 参数一致性约束补充（时间索引对齐 + 初始化窗口 + 周期一致）
5. §5.5 新增 27 种配置完整枚举表 + K3 三角形子图附录

# 审查任务

## 审查1：修正验证
对每项修正逐一评估：
- 修正是否完整？
- 是否引入新问题？
- 是否与定义基线一致？

## 审查2：数学严格性再验证

### 2.1 §5.5 枚举表验证
27 种配置的派生边约束推导是否正确？请独立推导以下 8 条配置的派生边约束，并与表对比：
- #1 (+,+,+)
- #4 (+,+,-)
- #7 (+,-,-)
- #10 (+,+,0)
- #16 (+,-,0)
- #19 (0,+,-)
- #22 (+,0,0)
- #3 (0,0,0)

推导方法：在 log 空间 ln(E/C) = ln(E/$) - ln(C/$)，根据两端符号判断差的符号：
- 异号 → 方向确定
- 同号 → 方向不确定 {{+,-,0}}
- 一端为0 → 部分约束 {{s,0}}

### 2.2 log-MACD 可加性
EMA 线性性的证明是否严格？新增的参数一致性前提是否充分？
请验证：EMA(x+y) = EMA(x) + EMA(y) 的归纳证明。

### 2.3 两种恒等式的区分
§2.3 中乘法恒等式和加法守恒的区分是否数学上精确？

## 审查3：交叉验证 Opus 报告
Opus 报告发现了 4 个新问题：
1. §7 配置编号引用错误（#14→#3）
2. §5.4 命名模式与 §5.5 编号无交叉引用
3. §2.3 加法守恒"约束"措辞轻微误导
4. §1.1 构造原则与 make_ratio_kline 代码差距

对每个发现：你是否同意？有无补充？

## 审查4：独立发现
从你的视角独立发现的、Opus 和 Codex 均未覆盖的问题。

# 输出格式

请按以下结构输出：

```
## 一、修正验证（逐项）
### 1.1 §1.1 比价序列构造原则
[评估]
### 1.2 §2.3 守恒措辞修正
[评估]
### 1.3 §2.4 C1-C3 重定义
[评估]
### 1.4 §4.1 EMA 参数一致性
[评估]
### 1.5 §5.5 枚举表 + K3 附录
[评估]

## 二、数学严格性再验证
### 2.1 枚举表独立推导（8条）
[逐条推导与对比]
### 2.2 log-MACD 可加性
[EMA 线性性证明验证]
### 2.3 两种恒等式区分
[数学精确性评估]

## 三、Opus 报告交叉验证
### 3.1 §7 配置编号错误
[同意/不同意 + 理由]
### 3.2 §5.4-§5.5 交叉引用缺失
[同意/不同意 + 理由]
### 3.3 §2.3 "约束"措辞
[同意/不同意 + 理由]
### 3.4 §1.1 与代码差距
[同意/不同意 + 理由]

## 四、独立发现
[Gemini 视角独立发现的问题]

## 五、最终判定
[v4 是否可进入策略层构建 + 条件]
```

---

# v4 完整文本

{v4_text}

---

# 定义基线

## dengjia.md（等价关系定义）
{dengjia_def}

## liuzhuan.md（流转关系定义）
{liuzhuan_def}

## level_recursion.md（级别递归定义）
{level_recursion_def}

---

# 已有质询报告

## Codex 代码层异质审查
{codex_inquiry}

---

# Opus 验证报告
{opus_report}

---

# 代码基线

## equivalence.py
{equivalence_py}

## capital_flow.py
{capital_flow_py}

## matrix_topology.py
{matrix_topology_py}

## a_macd.py
{a_macd_py}
"""

print(f"Prompt length: {len(prompt)} chars")
print("Calling Gemini 2.5 Pro...")

MODEL = "gemini-2.5-pro"

try:
    response = client.models.generate_content(
        model=MODEL,
        contents=prompt,
        config=types.GenerateContentConfig(
            max_output_tokens=16384,
            temperature=0.2,
        ),
    )
    raw_text = response.text
except Exception as e:
    print(f"gemini-2.5-pro failed: {e}")
    print("Trying gemini-2.5-flash...")
    MODEL = "gemini-2.5-flash"
    response = client.models.generate_content(
        model=MODEL,
        contents=prompt,
        config=types.GenerateContentConfig(
            max_output_tokens=16384,
            temperature=0.2,
        ),
    )
    raw_text = response.text

print(f"Model used: {MODEL}")
print(f"Response length: {len(raw_text)} chars")

# Save raw response
raw_path = ROOT / "tmp" / "gemini-v4-verify-raw.md"
raw_path.write_text(raw_text, encoding="utf-8")
print(f"Raw response saved to {raw_path}")

# Save structured report
report_path = ROOT / ".chanlun" / "review-results" / "gemini-v4-post-fix-verify-20260227.md"
report_header = f"""# Gemini v4 修正后异质验证报告

**验证者**: Gemini（异质视角）
**日期**: 2026-02-27
**输入**: 修正后 v4 文本 + 定义基线 + 代码基线 + Codex/Opus 已有报告
**模型**: {MODEL}

---

"""
report_path.write_text(report_header + raw_text, encoding="utf-8")
print(f"Structured report saved to {report_path}")
print("Done.")
