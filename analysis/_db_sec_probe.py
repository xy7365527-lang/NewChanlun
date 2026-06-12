"""Databento REST 元数据探针：秒级bar可用性 + 范围 + 费用估算。"""
import base64
import json
import os
import urllib.request

KEY = os.environ.get("DATABENTO_KEY", "db-4Cxk3Q35QPXqFFj8snSbqENcwqr4G")
AUTH = base64.b64encode(f"{KEY}:".encode()).decode()
BASE = "https://hist.databento.com/v0"


def get(path: str, **params) -> object:
    qs = "&".join(f"{k}={v}" for k, v in params.items())
    url = f"{BASE}/{path}" + (f"?{qs}" if qs else "")
    req = urllib.request.Request(url, headers={"Authorization": f"Basic {AUTH}"})
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        return {"error": e.code, "body": e.read().decode()[:500]}


print("== GLBX.MDP3 schemas ==")
print(get("metadata.list_schemas", dataset="GLBX.MDP3"))
print("== IFEU.IMPACT schemas ==")
print(get("metadata.list_schemas", dataset="IFEU.IMPACT"))
print("== GLBX.MDP3 range ==")
print(get("metadata.get_dataset_range", dataset="GLBX.MDP3"))
print("== IFEU.IMPACT range ==")
print(get("metadata.get_dataset_range", dataset="IFEU.IMPACT"))

# 费用估算：CL 连续合约 1 个月秒级
print("== cost: CL ohlcv-1s 1mo ==")
print(get("metadata.get_cost", dataset="GLBX.MDP3", symbols="CL.v.0",
          stype_in="continuous", schema="ohlcv-1s",
          start="2025-04-01", end="2025-05-01"))
print("== cost: CL ohlcv-1s 1yr ==")
print(get("metadata.get_cost", dataset="GLBX.MDP3", symbols="CL.v.0",
          stype_in="continuous", schema="ohlcv-1s",
          start="2024-05-01", end="2025-05-01"))
print("== cost: BRN ohlcv-1s 1mo ==")
print(get("metadata.get_cost", dataset="IFEU.IMPACT", symbols="BRN.v.0",
          stype_in="continuous", schema="ohlcv-1s",
          start="2025-04-01", end="2025-05-01"))
print("== cost: BRN ohlcv-1s 1yr ==")
print(get("metadata.get_cost", dataset="IFEU.IMPACT", symbols="BRN.v.0",
          stype_in="continuous", schema="ohlcv-1s",
          start="2024-05-01", end="2025-05-01"))
