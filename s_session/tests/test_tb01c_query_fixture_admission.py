"""#1768：查询造数夹具只允许 reset 本次新建的空载体。"""
from contextlib import closing
from pathlib import Path
import sqlite3
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import tb01c_query_tests as base

class QueryFixtureAdmissionTests(unittest.TestCase):
    def test_only_fresh_carrier_selects_reset(self):
        for existing in (False, True):
            with self.subTest(existing=existing), tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                db = root / "input.sqlite"
                if existing:
                    with closing(sqlite3.connect(db)) as connection:
                        connection.execute("CREATE TABLE preserved(value TEXT)")
                        connection.commit()
                with patch.object(base.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "", "")) as run:
                    base.QueryTests.call.__func__(SimpleNamespace(base=root), db, "init")
                self.assertEqual(run.call_args.args[0][1], "init" if existing else "reset")
                self.assertIn("--admission-job-limits", run.call_args.args[0])
                if existing:
                    with closing(sqlite3.connect(db)) as connection:
                        self.assertEqual(connection.execute("SELECT count(*) FROM preserved").fetchone()[0], 0)
