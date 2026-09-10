#!/usr/bin/env python3
"""#1371 R9：真实 CLI/HTTP 的有界根、载荷和前缀回归；证据输出到显式私有目录。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import time
import urllib.request
import urllib.error

R = Path(__file__).resolve().parents[2]
B = R / "rust/target/debug/s_structure_session"
P = R / "s_session/profiles/testonly_tick_1_1_ohlc.json"
C = R / "s_session/catalog/signed-catalog.json"
LOG = []

def digest(b):
    return hashlib.sha256(b).hexdigest()

def run(argv):
    p = subprocess.run(list(map(str, argv)), cwd=R, capture_output=True, text=True, timeout=60)
    rec = dict(argv=list(map(str, argv)), exit=p.returncode, stdout=p.stdout, stderr=p.stderr)
    LOG.append(rec)
    return rec

def cli(db, cmd, *args):
    return run([B, cmd, "--db", db, *args])

def value(rec):
    assert rec["exit"] == 0, rec
    return json.loads(rec["stdout"])

def fingerprint(db):
    with sqlite3.connect(db) as c:
        schema = c.execute("SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name").fetchall()
        out = {"schema": schema, "schema_sha256": digest(repr(schema).encode()), "tables": {}}
        for (table,) in c.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"):
            cols = c.execute('PRAGMA table_info("%s")' % table).fetchall()
            rows = c.execute('SELECT * FROM "%s"' % table).fetchall()
            out["tables"][table] = {"columns": cols, "rows": len(rows), "sha256": digest(repr(rows).encode()),
                "column_sha256": {col[1]: digest(repr([r[i] for r in rows]).encode()) for i,col in enumerate(cols)}}
        return out

def backup(src, dst):
    with sqlite3.connect(Path(src).as_uri()+"?mode=ro", uri=True) as a, sqlite3.connect(dst) as b:
        a.backup(b)

def ev(i, price, rev=1):
    return dict(event_id="e"+str(i), revision=str(rev), seq=str(i), received_at="2026-09-09T00:00:00.000Z",
                raw_text=str(price), price=str(price), timestamp=str(i), volume="1")

def input_file(w, name, events):
    v = dict(schema_revision="1", session_id="s-session-testonly-001", source_namespace="testonly.tick.ohlc",
             source_epoch="1", instrument="TEST.TICK", profile="testonly_tick_1_1_ohlc", events=events)
    path = w / (name+".json")
    path.write_text(json.dumps(v))
    LOG.append({"input": name, "payload": v, "sha256": digest(path.read_bytes())})
    return path

class Server:
    def __init__(self, db):
        self.db = db
    def __enter__(self):
        with socket.socket() as s:
            s.bind(("127.0.0.1", 0)); self.port = s.getsockname()[1]
        argv = [sys.executable, R/"s_session/s_readonly_server.py", "--db", self.db, "--port", self.port,
                "--browser", R/"s_session/browser/index.html"]
        self.p = subprocess.Popen(list(map(str,argv)), cwd=R, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.owner = {"argv":list(map(str,argv)), "pid":self.p.pid, "db":str(self.db)}
        for _ in range(100):
            try:
                with socket.create_connection(("127.0.0.1",self.port),.1): return self
            except OSError: time.sleep(.02)
        raise RuntimeError("HTTP未就绪")
    def get(self, path):
        url="http://127.0.0.1:%d%s" % (self.port,path)
        try: q=urllib.request.urlopen(url,timeout=30)
        except urllib.error.HTTPError as e: q=e
        with q: body=q.read(); status=q.status
        rec={"url":url,"status":status,"body":body.decode(),"sha256":digest(body)}
        LOG.append(rec)
        return rec
    def __exit__(self,*args):
        cmdline=Path("/proc/%d/cmdline" % self.p.pid).read_bytes().replace(b"\0",b" ").decode()
        assert str(self.db) in cmdline and "s_readonly_server.py" in cmdline
        self.p.terminate(); out,err=self.p.communicate(timeout=10)
        LOG.append({**self.owner,"cmdline":cmdline,"signal":"SIGTERM","wait_exit":self.p.returncode,"stdout":out,"stderr":err})

def exercise(w):
    result={"normal":[], "corrupt":{}}
    db=w/"prefix.sqlite"
    value(cli(db,"init","--session","s-session-testonly-001","--catalog",C))
    with Server(db) as server:
        result["gen0"]={p:server.get(p) for p in ["/api/state","/api/delta?after_generation=0"]}
        assert all(r["status"]==200 for r in result["gen0"].values())
        stable=None
        for name,events in [("one",[ev(0,10000)]),("two",[ev(0,10000),ev(1,11000)]),
                ("three",[ev(2,10500)]),("correction",[ev(2,11500,2)]),("four",[ev(3,10800)]),
                ("large1",[ev(2,11501,9007199254740993)]),("large2",[ev(2,11502,9007199254740994)])]:
            inp=input_file(w,name,events)
            value(cli(db,"accept","--input",inp,"--profile",P,"--writer-epoch","1"))
            value(cli(db,"advance","--writer-epoch","1"))
            state=server.get("/api/state"); watch=server.get("/api/delta?after_generation=0")
            assert state["status"]==watch["status"]==200, (state,watch)
            snap=json.loads(state["body"])["snapshot"]
            ds=json.loads(watch["body"])["deltas"]
            observations={o["observation_id"]:o for d in ds for o in d["delta"]["observations"]}
            actual={o["observation_id"]:o for o in snap["observations"]}
            result["normal"].append({"name":name,"state":state,"watch":watch,"observation_equal":observations==actual})
            if stable is None: stable=value(cli(db,"snapshot","--as-of","1"))
            assert stable==value(cli(db,"snapshot","--as-of","1"))
            if name=="correction": backup(db,w/"control.sqlite")
    mutations={
        "wrong_current_root":"UPDATE meta SET value=(SELECT index_frontier FROM structure_deltas WHERE generation=1) WHERE key='index_frontier'",
        "wrong_cut":"UPDATE meta SET value='cut-99' WHERE key='structure_cut'",
        "inner_frontier":"UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.input_frontier','99') WHERE generation=4",
        "inner_seq":"UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.seq_range.to','99') WHERE generation=4",
        "inner_evidence":"UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.catalog_evidence.withdrawals','99') WHERE generation=4",
        "bad_scope":"DELETE FROM meta WHERE key='scope'",
        "empty_scope":"UPDATE meta SET value='{}' WHERE key='scope'",
        "wrong_scope": "UPDATE meta SET value='%s' WHERE key='scope'" % json.dumps({"structure":"CompleteCut","economic":"started"}),
    }
    for key in ("upserts","withdrawals","replaces","witnesses","relations","observations","raw_history_added"):
        mutations["missing_"+key]="UPDATE structure_deltas SET delta_json=json_remove(delta_json,'$.%s') WHERE generation=4" % key
        mutations["member_"+key]="UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.%s[0]',json('null')) WHERE generation=4" % key
    mutations["missing_witness_member"]="UPDATE structure_deltas SET delta_json=json_remove(delta_json,'$.witnesses[0]') WHERE generation=4"
    mutations["dangling_relation"]="UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.relations[0].object','missing-endpoint') WHERE generation=4"
    for column, badvalue in {"generation":99,"session_id":"wrong","catalog_revision":"wrong",
            "base_cut":"cut-99","next_cut":"cut-99","seq_range_json":'{"from":"99","to":"99"}',
            "index_frontier":"batch-missing","input_frontier":99,"catalog_run_status":"wrong",
            "catalog_evidence_json":'{"bad":true}'}.items():
        mutations["outer_"+column]="UPDATE structure_deltas SET %s=%s WHERE generation=4" % (column,
            str(badvalue) if type(badvalue) is int else "'"+badvalue+"'")
    for key in ("session_id","generation","catalog_revision","base_cut","next_cut","index_frontier","catalog_run_status"):
        mutations["inner_"+key]="UPDATE structure_deltas SET delta_json=json_set(delta_json,'$.%s','bad') WHERE generation=4" % key
    mutations["scope_wrong_type"]="UPDATE meta SET value='[]' WHERE key='scope'"
    mutations["scope_null"]="UPDATE meta SET value='null' WHERE key='scope'"
    mutations["catalog_evidence"]="UPDATE catalog SET evidence_json='{}' WHERE catalog_id='CC-006'"
    next_input=input_file(w,"next",[ev(3,10800)])
    for name,sql in mutations.items():
        bad=w/(name+".sqlite"); backup(w/"control.sqlite",bad)
        with sqlite3.connect(bad) as conn: conn.execute(sql)
        before=fingerprint(bad)
        rec={"sql":sql,"before":before,"http":{},"cli":{}}
        with Server(bad) as server:
            for path in ["/api/state","/api/snapshot","/api/catalog","/api/delta?after_generation=0",
                         "/api/state?as_of=1","/api/snapshot?as_of=1","/api/catalog?as_of=1"]:
                rec["http"][path]=server.get(path)
            for cmd,args in [("snapshot",[]),("watch",["--after-generation","0"]),("catalog",[]),
                    ("query",["--identity-key","testonly.tick.ohlc|1|TEST.TICK|e2"]),
                    ("accept",["--input",next_input,"--profile",P,"--writer-epoch","1"]),
                    ("advance",["--writer-epoch","1"]),("recover",["--new-epoch","2"])]:
                pre=fingerprint(bad); out=cli(bad,cmd,*args); post=fingerprint(bad)
                rec["cli"][cmd]={**out,"before":pre,"after":post}
        rec["after"]=fingerprint(bad)
        rec["rejected_zero_write"]=all(r["status"]==503 for r in rec["http"].values()) and all(
            r["exit"]!=0 and "StorageUnavailable" in r["stderr"] and r["before"]==r["after"] for r in rec["cli"].values()) and before==rec["after"]
        result["corrupt"][name]=rec
    return result

def main():
    ap=argparse.ArgumentParser();ap.add_argument("out",type=Path);args=ap.parse_args();args.out.mkdir(exist_ok=True)
    result={}
    try:
        result=exercise(args.out)
        assert all(r["observation_equal"] for r in result["normal"]), "正常前缀 observation 不等"
        assert all(r["rejected_zero_write"] for r in result["corrupt"].values()), "损坏读写门不一致"
    finally:
        (args.out/"results.json").write_text(json.dumps({"results":result,"log":LOG},ensure_ascii=False,indent=2))
    print("R9 CLI/HTTP有界回归通过")
if __name__=="__main__":main()
