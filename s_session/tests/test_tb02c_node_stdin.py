"""#1875 TB02C-18：验证脚本交给 node 的输入走管道，不能用 readFileSync(0, 'utf8') 读。

Node 24.22.2 的 readFileSync(fd, 'utf8') 走 C++ 快速路径，读 64 MB 的管道时会破坏堆：Boat 上
1800 次中 27 次以 SIGSEGV 或 `free(): invalid next size` 中止；同样输入用 readFileSync(0).toString('utf8')
1800 次 0 次中止。两种写法解出的字符串相同，这里只禁止前一种写法回到仓内。
"""
import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
UTF8_FD_READ = re.compile(r"readFileSync\(\s*(?:0|process\.stdin\.fd|['\"]/dev/stdin['\"])\s*,\s*(?:['\"]utf-?8['\"]|\{[^}]*encoding)")


class NodeStdinReadTests(unittest.TestCase):
    def test_no_utf8_fd_read_of_stdin(self):
        hits = [f"{p.relative_to(ROOT)}:{n}"
                for p in sorted(ROOT.rglob('*'))
                if p.suffix in ('.py', '.js', '.cjs', '.mjs') and p.is_file() and p != Path(__file__).resolve()
                for n, line in enumerate(p.read_text(errors='replace').splitlines(), 1)
                if UTF8_FD_READ.search(line)]
        self.assertEqual(hits, [])

    def test_pattern_catches_the_crashing_form(self):
        self.assertTrue(UTF8_FD_READ.search("JSON.parse(fs.readFileSync(0,'utf8'))"))
        self.assertTrue(UTF8_FD_READ.search('fs.readFileSync(0, {encoding: "utf8"})'))
        self.assertFalse(UTF8_FD_READ.search("JSON.parse(fs.readFileSync(0).toString('utf8'))"))

    @unittest.skipUnless(shutil.which('node'), 'node not installed')
    def test_buffer_read_decodes_piped_utf8_like_the_old_form(self):
        text = '{"k":"缠论 ✓ \\u00e9"}\n' * 4096
        out = subprocess.run(['node', '-e', "process.stdout.write(require('node:fs').readFileSync(0).toString('utf8'))"],
                             input=text.encode(), capture_output=True, check=True, timeout=60).stdout
        self.assertEqual(out.decode(), text)


if __name__ == '__main__':
    unittest.main()
