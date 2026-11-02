#!/usr/bin/env python3
"""
#1621 时钟合同 golden 值生成器——Unix epoch 纳秒 → YYYY-MM-DD 对拍。

使用标准库 datetime 生成边界值，验证 Rust 实现的精确性。
覆盖：Unix epoch、负数、月底、闰年、政策生效/过期边界。
"""
import datetime
import json

NS_PER_DAY = 86_400_000_000_000

def ns_to_date(ns: int) -> str:
    """将 Unix epoch 纳秒转换为 YYYY-MM-DD（UTC）。"""
    # 转换为秒（Python datetime 不直接支持纳秒，先转秒）
    seconds = ns // 1_000_000_000
    # datetime.fromtimestamp 处理 Unix 时间戳
    date = datetime.datetime.utcfromtimestamp(seconds).date()
    return date.strftime("%Y-%m-%d")

# 边界值：Unix epoch、负数、月底、闰年、政策边界
test_cases = [
    # (描述, 纳秒时间戳)
    ("Unix epoch", 0),
    ("Unix epoch +1ns", 1),
    ("Unix epoch -1ns", -1),
    ("Unix epoch +1天", NS_PER_DAY),
    ("Unix epoch -1天", -NS_PER_DAY),

    # 政策生效日（2026-10-20）
    ("政策生效日 2026-10-20", (datetime.date(2026, 10, 20) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("政策生效日前1天", (datetime.date(2026, 10, 19) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("政策生效日后1天", (datetime.date(2026, 10, 21) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),

    # 有效期（2027-10-20）
    ("有效期截止 2027-10-20", (datetime.date(2027, 10, 20) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("有效期后1天", (datetime.date(2027, 10, 21) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),

    # 月底边界
    ("2024-02-29 闰年2月末", (datetime.date(2024, 2, 29) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2024-02-28", (datetime.date(2024, 2, 28) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2023-02-28 平年2月末", (datetime.date(2023, 2, 28) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2023-03-01", (datetime.date(2023, 3, 1) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2026-01-31", (datetime.date(2026, 1, 31) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2026-04-30", (datetime.date(2026, 4, 30) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2026-12-31", (datetime.date(2026, 12, 31) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),

    # 边界年份
    ("2000-01-01 闰年", (datetime.date(2000, 1, 1) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2100-01-01 非闰世纪年", (datetime.date(2100, 1, 1) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("2000-02-29", (datetime.date(2000, 2, 29) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),

    # 负数（1970年前）
    ("1969-12-31", (datetime.date(1969, 12, 31) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("1969-01-01", (datetime.date(1969, 1, 1) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
    ("1968-02-29 闰年", (datetime.date(1968, 2, 29) - datetime.date(1970, 1, 1)).days * NS_PER_DAY),
]

golden = []
for desc, ns in test_cases:
    expected = ns_to_date(ns)
    golden.append({
        "description": desc,
        "ns": str(ns),
        "expected_date": expected
    })

output = {"test_cases": golden}
print(json.dumps(output, indent=2, ensure_ascii=False))
