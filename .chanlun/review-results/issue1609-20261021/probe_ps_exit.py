import os, sys, time, signal, subprocess, json
sys.path.insert(0, sys.argv[1])
from s_service_control import process_identity
def spawn_orphan(cmd):
    # intermediate exits immediately, like launcher start-s CLI
    r, w = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(r)
        p = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        os.write(w, str(p.pid).encode()); os._exit(0)
    os.close(w); child = int(os.read(r, 64)); os.waitpid(pid, 0); return child
def watch(pid, cmd, label):
    ident = process_identity(pid, cmd)
    os.kill(pid, signal.SIGTERM); t0 = time.monotonic(); seq = []
    while time.monotonic() - t0 < 12:
        raw = subprocess.run(["ps","-p",str(pid),"-o","uid=,stat=,lstart=,command="],capture_output=True,text=True)
        try: st = process_identity(pid, cmd); st = "stopped" if st is None else ("same" if st == ident else "other:"+st)
        except Exception as e: st = "EXC:"+type(e).__name__+":"+str(e)
        seq.append((round(time.monotonic()-t0,3), raw.returncode, raw.stdout.strip()[:80], raw.stderr.strip()[:80], st))
        if st == "stopped": break
        time.sleep(0.05)
    ppid = open(f"/proc/{pid}/stat").read().split()[3] if os.path.exists(f"/proc/{pid}/stat") else None
    print(label, json.dumps({"n": len(seq), "first": seq[:2], "last": seq[-3:], "ppid_after": ppid}, ensure_ascii=False))
cmd = ["/bin/sleep", "300"]
watch(spawn_orphan(cmd), cmd, "orphan-sleep")
p = subprocess.Popen(cmd, start_new_session=True); watch(p.pid, cmd, "own-child-unreaped"); p.wait()
print("pid1:", open("/proc/1/cmdline").read().replace("\0"," "), "self ppid chain:", os.getppid())
print("subreaper:", subprocess.run(["grep","-l","","/proc/self/status"],capture_output=True).returncode)
