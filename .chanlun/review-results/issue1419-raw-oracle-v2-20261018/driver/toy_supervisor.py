"""仅监督本包内固定Python toy。无任意命令/产品启动接口。

真实子进程、非阻塞限读、总截止/失联/人工取消、TERM→KILL→wait/reap。
只记录直接子PID+本次Popen/进程组；不是任意后代树/OS资源硬帽资格证明。
"""
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time

TOYS={
    'success':"print('toy-success',flush=True)",
    'wait':"import time; print('toy-wait',flush=True); time.sleep(30)",
    'ignore-term':"import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); print('toy-ignore-term',flush=True); time.sleep(30)",
    'flood':"import sys; sys.stdout.write('x'*65536); sys.stdout.flush()",
}


def supervise(name,output,*,seconds,log_bytes,term_grace,kill_grace,owner_alive=lambda:True,cancelled=lambda:False):
    if name not in TOYS:raise ValueError('只允许固定toy')
    if min(seconds,log_bytes,term_grace,kill_grace)<=0:raise ValueError('监督参数须显式正值')
    dest=Path(output);dest.mkdir(parents=True,exist_ok=False)
    started=time.monotonic();deadline=started+seconds
    # 显式最小环境，不读取或传输宿主凭据。
    sel=None;files={};used=0;reason=None;signals=[];term_at=None;kill_at=None;reaped=False
    p=subprocess.Popen([sys.executable,'-I','-u','-c',TOYS[name]],stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                       stdin=subprocess.DEVNULL,start_new_session=True,env={'PYTHONDONTWRITEBYTECODE':'1'})
    # Popen 成功后立即保护；包含 selector、非阻塞、注册、日志打开的部分初始化。
    try:
        identity={'pid':p.pid,'pgid':p.pid,'popen_started_monotonic':started,'supervisor_pid':os.getpid()}
        def send(sig):
            if p.poll() is None:
                if os.getpgid(p.pid)!=p.pid:raise RuntimeError('子进程组身份变化')
                os.killpg(p.pid,sig);signals.append(signal.Signals(sig).name)
        sel=selectors.DefaultSelector()
        for stream,label in [(p.stdout,'stdout'),(p.stderr,'stderr')]:
            os.set_blocking(stream.fileno(),False);sel.register(stream,selectors.EVENT_READ,label)
            files[label]=(dest/(label+'.raw')).open('wb')
        while sel.get_map() or p.poll() is None:
            now=time.monotonic()
            if reason is None:
                if cancelled():reason='人工取消toy'
                elif not owner_alive():reason='owner失联toy'
                elif now>=deadline:reason='总截止toy'
                elif used>=log_bytes and p.poll() is None:reason='日志预算toy'
            if reason and term_at is None:send(signal.SIGTERM);term_at=now
            if term_at is not None and now-term_at>=term_grace and kill_at is None:
                send(signal.SIGKILL);kill_at=now
            if kill_at is not None and now-kill_at>=kill_grace and p.poll() is None:
                raise RuntimeError('KILL后未reap；不得报成功')
            try:ready=sel.select(0.01)
            except InterruptedError:continue
            for key,_ in ready:
                try:data=os.read(key.fileobj.fileno(),min(4096,max(1,log_bytes-used)))
                except BlockingIOError:continue
                if not data:sel.unregister(key.fileobj);key.fileobj.close();continue
                if used+len(data)>log_bytes:
                    reason=reason or '日志预算toy';continue
                files[key.data].write(data);used+=len(data)
                if used>=log_bytes:reason=reason or '日志预算toy'
        returncode=p.wait(timeout=kill_grace);reaped=True
    finally:
        original=sys.exc_info()[1]
        cleanup_errors=[]
        def clean(action):
            try:action()
            except BaseException as error:cleanup_errors.append(error)
        def kill_owned():
            if p.poll() is None:
                try:
                    if os.getpgid(p.pid)!=p.pid:raise RuntimeError('子进程组身份变化')
                    os.killpg(p.pid,signal.SIGKILL);signals.append('SIGKILL')
                except ProcessLookupError:
                    pass
                except BaseException:
                    # 仅 Popen 持有的直接子进程；绝不对别的进程组补发信号。
                    p.kill()
                    raise
        clean(kill_owned)
        if not reaped:
            def reap():
                nonlocal reaped
                p.wait(timeout=kill_grace);reaped=True
            clean(reap)
        # 不依赖注册成功与否，逐个关闭 Popen 返回的管道和已打开日志。
        for stream in (p.stdout,p.stderr,p.stdin):
            if stream is not None:clean(stream.close)
        if sel is not None:clean(sel.close)
        for f in files.values():clean(f.close)
        if cleanup_errors:
            detail='；'.join(type(e).__name__+':'+str(e) for e in cleanup_errors)
            if original is not None:
                if hasattr(original,'add_note'):original.add_note('清理失败：'+detail)
            else:raise RuntimeError('清理失败：'+detail)
    return {'toy':name,'identity':identity,'reason':reason,'signals':signals,'returncode':returncode,
            'reaped':reaped,'log_bytes_saved':used,'logs_complete':reason!='日志预算toy',
            'elapsed_seconds':time.monotonic()-started,'product_run':'NOT_RUN',
            'os_memory_hard_cap':'UNQUALIFIED','disk_quota':'UNQUALIFIED',
            'scope':'仅本Popen直接子进程与其新进程组；不认证产品后代追踪/同步IO硬截止'}
