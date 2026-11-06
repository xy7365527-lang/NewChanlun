"""#1875 TB02C-18：交给 node 的输入走管道时，不能用 readFileSync(<stdin>, utf8) 读。

Node 24.21–24.22.2 的 readFileSync(fd, 'utf8') 快速路径在管道短读之后会把累计数据复制进固定 64 KiB 的堆块
（nodejs/node#66341，上游修复 nodejs/node#66939）。Boat 上 1800 次中 27 次以 SIGSEGV 或
`free(): invalid next size` 中止；同样输入改用 readFileSync(0).toString('utf8')，1800 次 0 次中止。
两种写法解出的字符串相同。这里禁止前一种写法出现在受版本控制的源码里，包括 .rs/.py 中内嵌的 JS 字符串。
"""
import re
import shutil
import subprocess
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SOURCE_SUFFIXES = {'.py', '.rs', '.js', '.cjs', '.mjs', '.ts', '.mts', '.cts', '.sh'}
_Q = r"""\\?['"`]"""
UTF8_STDIN_READ = re.compile(
    r"readFileSync\s*\(\s*(?:0|process\s*\.\s*stdin\s*\.\s*fd|" + _Q + r"/dev/stdin" + _Q + r")\s*,\s*"
    r"(?:" + _Q + r"utf-?8" + _Q + r"|\{[^}]*encoding\s*:\s*" + _Q + r"utf-?8" + _Q + r")",
    re.IGNORECASE)


def tracked_sources():
    out = subprocess.run(['git', '-C', str(REPO), 'ls-files', '-z'], capture_output=True, check=True).stdout
    for rel in out.decode().split('\0'):
        p = REPO / rel
        if rel and p.suffix in SOURCE_SUFFIXES and p.is_file() and p != Path(__file__).resolve():
            yield p


def hits_in(text):
    return [text.count('\n', 0, m.start()) + 1 for m in UTF8_STDIN_READ.finditer(text)]


class NodeStdinReadTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('git') and (REPO / '.git').exists(), 'needs a git checkout')
    def test_no_utf8_read_of_stdin_in_tracked_sources(self):
        hits = [f"{p.relative_to(REPO)}:{n}" for p in tracked_sources()
                for n in hits_in(p.read_text(errors='replace'))]
        self.assertEqual(hits, [])

    def test_pattern_catches_the_crashing_forms(self):
        for bad in ("JSON.parse(fs.readFileSync(0,'utf8'))",
                    'fs.readFileSync( 0 , "utf-8" )',
                    "fs.readFileSync(\n  0,\n  'utf8'\n)",
                    'fs.readFileSync(process.stdin.fd, {encoding: "utf8"})',
                    "fs.readFileSync('/dev/stdin', 'utf8')",
                    r'"const s=fs.readFileSync(0,\"utf8\")"'):
            self.assertTrue(hits_in(bad), bad)
        self.assertEqual(hits_in("a\nb\nfs.readFileSync(\n0,'utf8')"), [3])

    def test_pattern_allows_buffer_reads(self):
        for ok in ("JSON.parse(fs.readFileSync(0).toString('utf8'))",
                   'fs.readFileSync(0, {flag: "r"})',
                   'fs.readFileSync(0, {encoding: null})',
                   "fs.readFileSync(path, 'utf8')"):
            self.assertEqual(hits_in(ok), [], ok)

    @unittest.skipUnless(shutil.which('node'), 'node not installed')
    def test_buffer_read_decodes_piped_utf8(self):
        text = '{"k":"缠论 ✓ \\u00e9"}\n' * 4096
        out = subprocess.run(['node', '-e', "process.stdout.write(require('node:fs').readFileSync(0).toString('utf8'))"],
                             input=text.encode(), capture_output=True, check=True, timeout=60).stdout
        self.assertEqual(out.decode(), text)


if __name__ == '__main__':
    unittest.main()
