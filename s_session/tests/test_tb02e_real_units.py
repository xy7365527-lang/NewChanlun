"""#1419 schema v1.3 §1（根裁定 ROOT-RULING-1419-L3）：L0 单元区间 = 成员笔方向极值，真实数据判别例锁。

夹具 fixtures/tb02e-real/btc-tail50k-first600.json：真实 BTC 尾 5 万根前 600 根上 CC-013 的输入合同字段与生产塔 L0 单元
（来源见夹具 provenance）。其中有单元的几何端点不是段内极值。Q（s_tb02_contract._segment_units）与浏览器 JS
（segmentUnits）都须取成员笔极值，与塔逐位相同；几何端点口径在判别单元上必与塔不同（改回几何端点即红）。
"""
import json
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_tb02_contract as c  # noqa: E402

FIXTURE = ROOT / "tests/fixtures/tb02e-real/btc-tail50k-first600.json"
JS = r"""
const fs = require('fs'), vm = require('vm');
const source = fs.readFileSync(process.argv[1], 'utf8'), marker = '  return Object.freeze({TB02_KINDS:';
if (source.split(marker).length !== 2) throw new Error('export marker not unique');
const context = vm.createContext({module: {exports: {}}, TextEncoder, TextDecoder, crypto: require('crypto').webcrypto});
vm.runInContext(source.replace(marker, '  globalThis.__units = segmentUnits;\n' + marker), context);
const records = JSON.parse(fs.readFileSync(process.argv[2], 'utf8')).segments;
const [done, pending] = context.__units(records);
process.stdout.write(JSON.stringify({done: done.map(u => [u[0], u[1], String(u[2]), String(u[3]), u[4]]), pending: [...pending.keys()]}));
"""


def tower(fixture):
    return [[str(u["lo"]), str(u["hi"]), u["dir"]] for u in fixture["tower_units"]]


class RealUnitIntervals(unittest.TestCase):
    def setUp(self):
        self.fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))

    def test_fixture_discriminates_geometric_endpoints(self):
        done = [s["payload"]["data"] for s in self.fixture["segments"] if s["payload"]["data"]["result"] in ("CASE_ONE", "CASE_TWO")]
        geometric = [sorted((int(d["stroke_refs"][0]["start_price"]), int(d["geometric_end"]["price"]))) for d in done]
        disc = [k for k, (g, t) in enumerate(zip(geometric, self.fixture["tower_units"])) if g != [t["lo"], t["hi"]]]
        self.assertEqual(disc, self.fixture["discriminating_units"])
        self.assertGreaterEqual(len(disc), 1, "夹具须含几何端点不是段内极值的段")

    def test_python_units_are_member_stroke_extremes_equal_tower(self):
        done, pending = c._segment_units(self.fixture["segments"])
        self.assertEqual([[str(u[2]), str(u[3]), u[4]] for u in done], tower(self.fixture))
        self.assertEqual(len(pending), 1)

    def test_js_units_are_member_stroke_extremes_equal_tower(self):
        node = shutil.which("node")
        if node is None:
            raise AssertionError("node 不可用：跨语言判别例不得跳过")
        out = subprocess.run([node, "-e", JS, str(ROOT / "browser/tb01c-client.js"), str(FIXTURE)], capture_output=True, text=True, timeout=60)
        self.assertEqual(out.returncode, 0, out.stderr)
        got = json.loads(out.stdout)
        self.assertEqual([[u[2], u[3], u[4]] for u in got["done"]], tower(self.fixture))
        py_done, _ = c._segment_units(self.fixture["segments"])
        self.assertEqual([u[0] for u in got["done"]], [u[0] for u in py_done])


if __name__ == "__main__":
    unittest.main()
