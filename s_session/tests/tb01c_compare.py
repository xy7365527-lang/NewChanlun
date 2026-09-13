#!/usr/bin/env python3
"""#1372：逐记录硬比较语义轨迹；不排除字段、不重命名身份、不排序数组。

采集器从采集起把主机耗时/PID等诊断放在另一个文件，本比较器没有排除参数。
expected_records 必须来自事前轨迹，不能从待比较结果反推；相同的截断不算通过。
这里只证明所给记录相等，完整 AC6 还要求驱动器证明采集覆盖每次输入/提交/恢复。
"""

import argparse
import hashlib
import json
from itertools import zip_longest
from pathlib import Path


MAX_RECORD_BYTES = 16 * 1024 * 1024


class InvalidTrace(ValueError):
    """缺失、重复键或非精确 JSON 不可作为已验语义轨迹。"""


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise InvalidTrace("重复 JSON 对象键：" + key)
        result[key] = value
    return result


def _reject_number(value):
    raise InvalidTrace("轨迹含浮点或非有限数：" + value)


def read_records(path):
    """有限单行读取；空行/非对象记录也拒绝，不静默丢样本。"""
    with Path(path).open("rb") as stream:
        line = 0
        while raw := stream.readline(MAX_RECORD_BYTES + 1):
            line += 1
            if len(raw) > MAX_RECORD_BYTES:
                raise InvalidTrace(f"第 {line} 行超过具名记录字节上限")
            try:
                value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object,
                                   parse_float=_reject_number, parse_constant=_reject_number)
            except (ValueError, UnicodeError, RecursionError) as exc:
                raise InvalidTrace(f"第 {line} 行：{exc}") from exc
            if type(value) is not dict or not value:
                raise InvalidTrace(f"第 {line} 行必须是非空对象记录")
            yield value


def _pointer(path, key):
    return path + "/" + str(key).replace("~", "~0").replace("/", "~1")


def first_difference(left, right, path=""):
    """对象键次序不是 JSON 语义；键集合、类型、数组次序、重复成员和值均硬比。"""
    if type(left) is not type(right):
        return path, "type", left, right
    if isinstance(left, dict):
        for key in left:
            if key not in right:
                return _pointer(path, key), "missing_right", left[key], None
            difference = first_difference(left[key], right[key], _pointer(path, key))
            if difference is not None:
                return difference
        for key in right:
            if key not in left:
                return _pointer(path, key), "missing_left", None, right[key]
    elif isinstance(left, list):
        for index, (a, b) in enumerate(zip(left, right)):
            difference = first_difference(a, b, _pointer(path, index))
            if difference is not None:
                return difference
        if len(left) != len(right):
            return path, "array_length", len(left), len(right)
    elif left != right:
        return path, "value", left, right
    return None


def _value_evidence(value):
    encoded = json.dumps(value, ensure_ascii=True, separators=(",", ":"), allow_nan=False)
    return {"type": type(value).__name__, "preview": encoded[:256],
            "preview_truncated": len(encoded) > 256,
            "sha256": hashlib.sha256(encoded.encode()).hexdigest()}


def compare_traces(left, right, expected_records):
    """PASS 是具名记录数内全量逐字段相等；读取失败与双边截断是 NOT_VERIFIED。"""
    if type(expected_records) is not int or expected_records <= 0:
        raise ValueError("expected_records 必须是事前指定的正整数")
    result = {"schema": "tb01c.exact_trace_comparison.v1", "status": "NOT_VERIFIED",
              "expected_records": expected_records, "compared_records": 0,
              "excluded_fields": [], "identity_renaming": [], "first_difference": None}
    missing = object()
    try:
        for index, (a, b) in enumerate(zip_longest(read_records(left), read_records(right),
                                                 fillvalue=missing)):
            if a is missing or b is missing:
                result["status"] = "FAIL"
                result["first_difference"] = {"record": index, "path": "",
                                              "kind": "missing_record",
                                              "missing_side": "left" if a is missing else "right"}
                return result
            if index >= expected_records:
                result["detail"] = "实际轨迹超过事前记录数；不能只比较共同前缀"
                return result
            difference = first_difference(a, b)
            if difference is not None:
                path, kind, av, bv = difference
                result["status"] = "FAIL"
                result["first_difference"] = {"record": index, "path": path, "kind": kind,
                                              "left": _value_evidence(av), "right": _value_evidence(bv)}
                return result
            result["compared_records"] += 1
    except (InvalidTrace, OSError, RecursionError) as exc:
        result["detail"] = str(exc)
        return result
    if result["compared_records"] != expected_records:
        result["detail"] = "两边均未达到事前轨迹长度；最终状态相同不能替代完整轨迹"
        return result
    result["status"] = "PASS"
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    parser.add_argument("--expected-records", required=True, type=int)
    args = parser.parse_args()
    if args.expected_records <= 0:
        parser.error("--expected-records 必须为正数")
    result = compare_traces(args.left, args.right, args.expected_records)
    print(json.dumps(result, ensure_ascii=True, indent=2))
    return {"PASS": 0, "FAIL": 1, "NOT_VERIFIED": 2}[result["status"]]


if __name__ == "__main__":
    raise SystemExit(main())
