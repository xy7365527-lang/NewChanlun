"""#1372：正式 v2 writer/init/serve → Q 新鲜审计、HTTP分页/Watch 的小域集成。

单独的工程回归，不代表 160 输入、2/s、故障双跑或 GUI 八项 AC 已验收。
"""

import argparse
from contextlib import closing
import hashlib
import http.client
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time

from tb01c_query_tests import resources, reader, query, audit, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="tqv2-", dir="/tmp") as directory:
        base = Path(directory)
        db, endpoint = base/"s.sqlite", base/"s.sock"
        profile = ROOT/"s_session/profiles/testonly_tick_1_1_ohlc.json"
        clock = {"schema_revision":"s-clock-plan/1", "clock_plan_id":"q-integration-clock",
                 "origin_utc":"2000-01-01T00:00:00Z", "unit":"ns", "events":{
                     f"op-{i}": {"accept_ns":str(i*10+1),"attempts":[{"begin_ns":str(i*10+2),"commit_ns":str(i*10+3)}]}
                     for i in range(6)}}
        clock["events"]["recover-q-test"] = {"recover_ns":"60"}
        clock_path = base/"clock.json"
        clock_path.write_bytes(audit.canonical(clock))
        env = dict(os.environ, S_SESSION_PRAGMA_SIDECAR_PATH=str(base/"pragma.txt"), S_CONTROL_INSTANCE_ID="q-test-writer")
        binary = str(args.binary.resolve())
        result = subprocess.run([binary,"init","--db",str(db),"--session","q-v2-test","--catalog",
            str(ROOT/"s_session/catalog/signed-catalog.json"),"--profile",str(profile),"--session-generation","1",
            "--clock-plan",str(clock_path),"--delivery-retain-generations","2"],env=env,capture_output=True,text=True,timeout=15)
        assert result.returncode == 0, result.stderr
        qreader = query.AuditReader(db, resources(), vars(reader))
        server = None
        process = None
        log = (base/"server.log").open("wb")
        calls = 0

        def envelope(payload, message):
            return dict(schema_revision="s-session/2",session_id="q-v2-test",session_generation="1",
                source_namespace="q-v2-input",source_epoch="1",message_id=message,producer_id="q-tests",
                producer_epoch="1",causal_refs=[],payload=payload,payload_hash=hashlib.sha256(audit.canonical(payload)).hexdigest())

        def send(payload, message):
            nonlocal calls
            with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as sock:
                sock.settimeout(10)
                sock.connect(str(endpoint))
                sock.sendall(audit.canonical(envelope(payload,message))+b"\n")
                sock.shutdown(socket.SHUT_WR)
                data = bytearray()
                while not data.endswith(b"\n"):
                    chunk = sock.recv(65536)
                    assert chunk, "S reply truncated"
                    data.extend(chunk)
                    assert len(data)<1024*1024
            response = audit.parse_json(bytes(data))
            assert "payload_hash" in response, response
            assert response["payload_hash"] == hashlib.sha256(audit.canonical(response["payload"])).hexdigest()
            assert response["payload"].get("ok") is not False, response["payload"]
            calls += 1
            return response["payload"]

        def ingest(i,price):
            source = dict(schema_revision="1",session_id="q-v2-test",source_namespace="q-v2-input",
                source_epoch="1",instrument="TEST.TICK",profile="testonly_tick_1_1_ohlc",
                events=[dict(event_id=f"e{i}",revision="1",seq=str(i),price=str(price),timestamp=str(i),
                             volume="1",received_at="2000-01-01T00:00:00Z",raw_text=str(price))])
            return send(dict(op="ingest",target_session_id="q-v2-test",target_session_generation="1",
                             writer_epoch="1",clock_event_id=f"op-{i}",raw_input=source),f"input-{i}")

        def post(payload, message):
            conn = http.client.HTTPConnection(*server.server_address,timeout=10)
            try:
                conn.request("POST", "/api/v2/"+payload["op"], audit.canonical(envelope(payload,message)), {"Content-Type":"application/json"})
                response = conn.getresponse()
                value = audit.parse_json(response.read())
            finally:
                conn.close()
            assert response.status == 200, value
            assert value["payload_hash"] == hashlib.sha256(audit.canonical(value["payload"])).hexdigest()
            assert value["producer_epoch"] == "3" and value["producer_id"] == "Q:q-v2-test"
            assert "control_instance_id" not in value
            return value["payload"]

        try:
            initial = qreader.capture_verified()
            assert initial["generation"] == 0 and initial["protocol"]["logical_phase_frontier"] == "-1"
            zero,_ = query.project_fixed_cut(initial,0,"AsKnown",vars(reader))
            assert zero["fixed_cut"]["profile_hash"] == initial["meta"]["profile_hash"]
            assert zero["fixed_cut"]["profile_id"] == initial["meta"]["profile_id"]
            process = subprocess.Popen([binary,"serve","--db",str(db),"--socket",str(endpoint),"--writer-epoch","1",
                "--profile",str(profile),"--clock-plan",str(clock_path),"--max-frame-bytes","1048576",
                "--read-timeout-ms","2000","--write-timeout-ms","2000","--queue-capacity","8","--max-connections","8"],
                env=env,stdout=log,stderr=log,start_new_session=True)
            deadline = time.monotonic()+5
            while not endpoint.exists():
                assert process.poll() is None, "S exited at startup"
                assert time.monotonic()<deadline, "S startup timeout"
                time.sleep(0.01)
            send({"op":"ready"},"ready")
            for i, price in enumerate((10000,11000,10500,12000)):
                ingest(i,price)
                assert qreader.capture_verified()["generation"] == i+1
            proof = qreader.capture_verified()
            assert proof["protocol"]["delivery_policy"]["first_available_generation"] == 3
            class Quiet(reader.Handler):
                db_path = str(db)
                browser_path = str(ROOT/"s_session/browser/index.html")
                def log_message(self,*_args):
                    pass
            server = reader.QueryHTTPServer(("127.0.0.1",0),Quiet,db,resources(),3)
            thread = threading.Thread(target=server.serve_forever,daemon=True)
            thread.start()
            page = post(dict(op="snapshot",session_id="q-v2-test",session_generation="1",scope=reader._scope(proof["meta"]),
                history_mode="AsKnown",as_of_generation="4",page_size="2",order_version=query.ORDER_VERSION),"page-first")
            first_digest, old_token = page["cut_projection_digest"], page["next_token"]
            rows = list(page["rows"])
            ingest(4,11500)
            while not page["done"]:
                page = post({"op":"snapshot","snapshot_token":page["next_token"]},"page-next-"+page["offset"])
                assert page["cut_projection_digest"] == first_digest and page["fixed_cut"]["cut_generation"] == "4"
                rows.extend(page["rows"])
            expected,_ = query.project_fixed_cut(qreader.capture_verified(),4,"AsKnown",vars(reader))
            assert rows == expected["rows"]
            assert old_token["issued_capture_digest"] != page["validated_capture_digest"]
            current = qreader.capture_verified()
            gap = post(dict(op="watch",cursor=query.cursor_at(current,0,vars(reader)),max_batches="2",client_id="stale"),"watch-gap")
            assert gap["gap"]["missing_range"] == {"from_generation":"1","to_generation":"3"}
            normal = post(dict(op="watch",cursor=query.cursor_at(current,3,vars(reader)),max_batches="1",client_id="normal"),"watch-one")
            assert normal["next_cursor"]["after_generation"] == "4" and normal["observed_head"]["cut_generation"] == "5"
            # 正式 S 进程退出后，Q 仍从已发布存储返回完整分页切面。
            process.terminate(); process.wait(timeout=5)
            after_stop = post({"op":"snapshot","snapshot_token":old_token},"after-S-stop")
            assert after_stop["cut_projection_digest"] == first_digest
            recovery = subprocess.run([binary,"recover","--db",str(db),"--new-epoch","2",
                "--clock-plan",str(clock_path),"--clock-event-id","recover-q-test"],
                env=env,capture_output=True,text=True,timeout=15)
            assert recovery.returncode == 0, recovery.stderr
            recovered = qreader.capture_verified()
            assert recovered["meta"]["writer_epoch"] == "2"
            assert recovered["tables"]["writer_epoch_history"][0]["advance_state_at_transition"] == "idle"
            assert post({"op":"snapshot","snapshot_token":old_token},"after-recover")["cut_projection_digest"] == first_digest
            corruption_cases = {
                "begin_wrong_publication": "UPDATE s_clock_events SET generation=2 WHERE phase='begin' AND clock_event_id='op-0'",
                "attempt_count_huge": "UPDATE s_input_messages SET attempt_count=9223372036854775807 WHERE clock_event_id='op-0'",
                "missing_message": "DELETE FROM s_input_messages WHERE clock_event_id='op-0'",
                "missing_accept_phase": "DELETE FROM s_clock_events WHERE phase='accept' AND clock_event_id='op-0'",
                "missing_profile_definition": "DELETE FROM meta WHERE key='profile_definition'",
                "wrong_phase_frontier": "UPDATE s_protocol_meta SET logical_phase_frontier='999'",
                "v2_writer_epoch_zero": "UPDATE meta SET value='0' WHERE key='writer_epoch'",
                "v2_epoch_history_state": "UPDATE writer_epoch_history SET advance_state_at_transition='unverified'",
            }
            for name, sql in corruption_cases.items():
                broken = base/(name+".sqlite")
                with closing(reader.open_readonly(db)) as source, closing(sqlite3.connect(broken)) as target:
                    source.backup(target)
                check = query.AuditReader(broken, resources(), vars(reader))
                try:
                    check.capture_verified()
                    with closing(sqlite3.connect(broken)) as conn, conn:
                        conn.execute(sql)
                    try:
                        check.capture_verified()
                    except ValueError:
                        pass
                    else:
                        raise AssertionError(name+" accepted after healthy cache")
                finally:
                    check.close()
            with closing(sqlite3.connect(db)) as conn, conn:
                conn.execute("DELETE FROM s_delivery_refs WHERE generation=4")
            try:
                qreader.capture_verified()
            except ValueError:
                pass
            else:
                raise AssertionError("missing durable delivery ref accepted")
            print(json.dumps({"status":"PASS_BOUNDED_QUERY_V2", "s_requests":calls,"real_ingests":5,
                "fixed_cut_rows":len(rows),"historical_token_after_commit_gc_s_stop":True,
                "missing_delivery_ref_rejected":True,"fresh_control_corruptions_rejected":len(corruption_cases),
                "actual_recover_preserves_historical_token":True,
                "full_c_acceptance":"NOT_RUN"},ensure_ascii=False))
        finally:
            if process is not None and process.poll() is None:
                process.terminate(); process.wait(timeout=5)
            if server is not None:
                server.shutdown(); thread.join(timeout=5); server.server_close()
            qreader.close()
            log.close()


if __name__ == "__main__":
    main()
