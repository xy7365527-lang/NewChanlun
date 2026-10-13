"""#1414 discover路由/预算反例；测试替身只测接口，不替代正式S/Q验收。"""
import copy
import json
from pathlib import Path
from types import SimpleNamespace
import sys
import unittest
from unittest.mock import Mock, patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_readonly_server as server

class DiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.proof = dict(meta=dict(session_id="session", profile_id="ohlc_integer_tb02a_v1", protocol_revision="s-session/2"),
                          protocol=dict(session_generation="1", protocol_revision="s-session/2"))
        self.capture = Mock(side_effect=lambda: self.proof)
        self.discard_projection = Mock()
        self.handler = server.Handler.__new__(server.Handler)
        self.handler.server = SimpleNamespace(query_reader=lambda: SimpleNamespace(capture_verified=self.capture, discard_projection=self.discard_projection),
            resources=dict(max_reply_bytes=8388608), producer_epoch=7, control_instance_id="nonce")
        self.replies = []
        self.handler._send_bytes = lambda body, status=200, **kwargs: self.replies.append((status, json.loads(body)))
        self.handler._send_json = lambda value, status=200: self.replies.append((status, value))

    def get(self, path):
        self.handler.path=path; self.handler.do_GET(); return self.replies[-1]

    def test_discover_single_proof_without_projecting_oversize_state(self):
        with patch.object(server._query_integrity(), "project_state", return_value={"cut":{},"snapshot":{"payload":"x"*8388608}}):
            status, body=self.get("/api/state")
        self.assertEqual((status,body["error"]),(503,"QueryBudgetExceeded"))
        self.discard_projection.assert_called_once_with()
        self.capture.reset_mock()
        with patch.object(server._query_integrity(), "project_state", side_effect=AssertionError("discover must not project")):
            status, body=self.get("/api/discover")
        self.assertEqual(status,200);self.capture.assert_called_once_with()
        self.assertEqual(body,dict(ok=True,protocol_revision="s-session/2",cut=dict(session_id="session",session_generation="1"),snapshot=dict(profile_id="ohlc_integer_tb02a_v1"),producer_epoch="7"))
        self.assertLess(len(json.dumps(body)),512)

    def test_discover_rejects_query_before_capture(self):
        for path in ("/api/discover?as_of=1","/api/discover?ignored=1","/api/discover?as_of=01"):
            with self.subTest(path=path):
                status,body=self.get(path);self.assertEqual((status,body["error"]),(400,"InvalidQuery"))
        self.capture.assert_not_called()

    def test_bad_identity_profile_protocol_fail_closed(self):
        for field,values in (("session_generation",[None,"","01",1,"0"]),("protocol_revision",[None,"s-session/3"]),("profile_id",[None,"","unknown"])):
            for value in values:
                with self.subTest(field=field,value=value):
                    original=copy.deepcopy(self.proof)
                    target=self.proof["meta"] if field=="profile_id" else self.proof["protocol"]
                    target[field]=value
                    status,body=self.get("/api/discover");self.assertEqual((status,body["error"]),(503,"StorageUnavailable"))
                    self.proof=original

    def test_legacy_is_explicit_and_storage_budget_failure_is_not_legacy(self):
        self.proof["protocol"]=None;self.proof["meta"].pop("protocol_revision")
        status,body=self.get("/api/discover");self.assertEqual(body,dict(ok=True,protocol_revision=None,producer_epoch="7"))
        for error in (ValueError("corrupt"),server._query_integrity().QueryBudgetExceeded("budget")):
            self.capture.side_effect=error;status,body=self.get("/api/discover")
            self.assertEqual(status,503);self.assertIs(body["ok"],False);self.assertNotIn("protocol_revision",body)

    def test_discover_obeys_same_reply_cap(self):
        self.handler.server.resources["max_reply_bytes"]=1
        status,body=self.get("/api/discover");self.assertEqual((status,body["error"]),(503,"QueryBudgetExceeded"))

if __name__=="__main__":unittest.main(verbosity=2)
