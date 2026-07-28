#!/usr/bin/env python3
"""#613（收 #609 F3）：L2 归因子桶复算——判定规则显式化，可第三方复算。

规则（本脚本即规则的可执行形式）：
1. 身份集 = `COMPLETION_SIGNAL level=2` 行去重，键 = (side, seg_a, seg_c_full, b_center_start)。
   同键多行取 as_of 最小者（首见）。
2. A 桶「存在 earlier Live」：存在 `PAN_LIVE_HIT level=2` 行满足
   b_center_start == 身份.b_center_start 且 c_start == 身份.seg_c_full[0] 且 as_of < 身份.as_of。
3. 否则 B 桶。子桶 = 该身份 C 活跃期 W=[seg_c_full[0], completed_at] 内全部 level=2 诊断行
   （PAN_LIVE_HIT/MISS，按 as_of 取闭区间）的 reason 的**众数**；并列取 W 内**最早**出现者。
   - reason == "window" 的行代表「该 run 定位成功但落在别的 λ_C」⟹ 记为 window_other_c。
   - W 内零 level=2 诊断行 ⟹ no_diag_row_in_window（该桶非空即归因未闭合）。
"""
import re
import sys
from collections import Counter, defaultdict

path = sys.argv[1]

comp_re = re.compile(
    r"^COMPLETION_SIGNAL as_of=(\d+) level=2 side=(\w+) kind=(\w+) "
    r"seg_a=\((\d+), (\d+)\) seg_c_full=\((\d+), (\d+)\) b_center_start=(\d+) "
    r"lower_id=\((\d+), (\d+)\) completed_at=(\d+)"
)
diag_re = re.compile(
    r"^PAN_LIVE_(HIT|MISS) as_of=(\d+) level=2 frontier_start=(\d+) reason=(\w+) "
    r"b_center_start=(\d+) c_start=(\d+)"
)

identities = {}
diag_rows = []          # (as_of, reason)
hits = defaultdict(list)  # (b_center_start, c_start) -> [as_of]

with open(path) as fh:
    for line in fh:
        if line.startswith("COMPLETION_SIGNAL"):
            m = comp_re.match(line)
            if not m:
                continue
            as_of, side, _kind, a0, a1, c0, c1, b, _l0, _l1, completed_at = m.groups()
            key = (side, int(a0), int(a1), int(c0), int(c1), int(b))
            prev = identities.get(key)
            if prev is None or int(as_of) < prev["as_of"]:
                identities[key] = {
                    "as_of": int(as_of),
                    "c_start": int(c0),
                    "completed_at": int(completed_at),
                    "b": int(b),
                }
        elif line.startswith("PAN_LIVE_"):
            m = diag_re.match(line)
            if not m:
                continue
            kind, as_of, _fs, reason, b, c_start = m.groups()
            diag_rows.append((int(as_of), reason))
            if kind == "HIT":
                hits[(int(b), int(c_start))].append(int(as_of))

diag_rows.sort()

buckets = Counter()
detail = []
for key, ident in sorted(identities.items(), key=lambda kv: kv[1]["as_of"]):
    earlier = [a for a in hits.get((ident["b"], ident["c_start"]), []) if a < ident["as_of"]]
    if earlier:
        buckets["A. 存在 earlier Live"] += 1
        detail.append((key, "A", ident["as_of"] - max(earlier)))
        continue
    lo, hi = ident["c_start"], ident["completed_at"]
    window = [r for (a, r) in diag_rows if lo <= a <= hi]
    if not window:
        buckets["B. no_diag_row_in_window"] += 1
        detail.append((key, "no_diag_row_in_window", None))
        continue
    counts = Counter(window)
    top = max(counts.values())
    tied = {r for r, n in counts.items() if n == top}
    first = next(r for r in window if r in tied)     # 并列取最早出现者
    label = "window_other_c" if first == "window" else first
    buckets["B. " + label] += 1
    detail.append((key, label, counts[first]))

print(f"== {path} ==")
print(f"L2 完成身份（去重）= {len(identities)}")
for name, n in sorted(buckets.items(), key=lambda kv: (-kv[1], kv[0])):
    print(f"  {name:42s} {n:3d}")
print(f"  {'合计':42s} {sum(buckets.values()):3d}")
unclosed = buckets.get("B. no_diag_row_in_window", 0)
print(f"归因闭合：{'PASS（零「不知道」）' if unclosed == 0 else f'BREAK（{unclosed} 只无诊断行）'}")
if len(sys.argv) > 2 and sys.argv[2] == "-v":
    for key, label, extra in detail:
        print(f"    {key} -> {label} ({extra})")
