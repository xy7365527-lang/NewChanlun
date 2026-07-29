#!/usr/bin/env python3
"""#613（收 #609 F3）：L2 归因子桶复算——判定规则显式化，可第三方复算。

规则（本脚本即规则的可执行形式）：
1. 身份集 = `COMPLETION_SIGNAL level=2` 行去重，键 = (side, seg_a, seg_c_full, b_center_start)。
   同键多行取 as_of 最小者（首见）。
2. A 桶「存在 earlier Live」：存在 `PAN_LIVE_HIT level=2` 行满足
   b_center_start == 身份.b_center_start 且 c_start == 身份.seg_c_full[0] 且
   seg_a == 身份.seg_a 且 as_of < 身份.as_of。
   #618：完整锚含 seg_a——粗键（仅 b_center_start+c_start）会把同一 C 位置上先后出现的多个不同
   身份（不同 seg_a 解释）的 B 观测历史误合并成一条时间线（#618 §2.1 实证 `c=17170`：3 个不同
   seg_a 身份共用同一 C，若不按 seg_a 拆分则「earlier Live」判定会跨身份误配）。旧 dump（无
   `seg_a=` 字段）向后兼容：诊断行 seg_a 缺失时退化为粗键匹配（不拒绝旧产物，但归因精度回退）。
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
    r"b_center_start=(\d+) c_start=(\d+)(?: gap_len=\d+)?(?: seg_a=(none|\(\d+,\d+\)))?"
)


def parse_seg_a(raw):
    """#618：`seg_a=(a,b)` 或 `seg_a=none`；字段缺失（旧 dump）返回 None（粗键回退标记）。"""
    if raw is None:
        return None
    if raw == "none":
        return "none"
    a, b = raw.strip("()").split(",")
    return (int(a), int(b))


identities = {}
diag_rows = []          # (as_of, reason)
hits = defaultdict(list)  # (b_center_start, c_start, seg_a_or_None) -> [as_of]

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
                    "seg_a": (int(a0), int(a1)),
                }
        elif line.startswith("PAN_LIVE_"):
            m = diag_re.match(line)
            if not m:
                continue
            kind, as_of, _fs, reason, b, c_start, seg_a_raw = m.groups()
            diag_rows.append((int(as_of), reason))
            if kind == "HIT":
                seg_a = parse_seg_a(seg_a_raw)
                hits[(int(b), int(c_start), seg_a)].append(int(as_of))

diag_rows.sort()
seg_a_field_present = any(key[2] is not None for key in hits)

buckets = Counter()
detail = []
for key, ident in sorted(identities.items(), key=lambda kv: kv[1]["as_of"]):
    hit_key = (ident["b"], ident["c_start"], ident["seg_a"] if seg_a_field_present else None)
    earlier = [a for a in hits.get(hit_key, []) if a < ident["as_of"]]
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
