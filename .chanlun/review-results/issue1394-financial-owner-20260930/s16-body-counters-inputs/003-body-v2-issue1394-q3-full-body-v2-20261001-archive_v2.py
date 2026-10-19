#!/usr/bin/env python3
"""Private Q3R2 evidence carrier reader; validates raw HTTP bytes, not business semantics."""
import argparse, base64, gzip, hashlib, json, os, struct, subprocess, sys, zlib
from pathlib import Path
if not __debug__: raise SystemExit('optimized Python disables archive validation asserts')

MAGIC = b'Q3R2'
HEADER = struct.Struct('>4sIIII32s32s')  # magic, metadata/request/response lengths, CRC32, two SHA256
MAX_RECORD = 10 * 1024 * 1024
MAX_CHUNK = 16 * 1024 * 1024

def canonical(x): return json.dumps(x, sort_keys=True, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode()
def sha(x): return hashlib.sha256(x).hexdigest()
def valid_meta(meta):
    assert type(meta) is dict and set(meta)=={'serial','complete','error','op','code'}
    assert type(meta['serial']) is int and meta['serial']>0 and type(meta['complete']) is bool
    assert type(meta['op']) is str and meta['op']
    if meta['complete']:
        assert meta['error'] is None and type(meta['code']) is int and 100<=meta['code']<=599
    else:
        assert type(meta['error']) is str and bool(meta['error'])
        assert meta['code'] is None or (type(meta['code']) is int and 100<=meta['code']<=599)

def pack(meta, request, response):
    valid_meta(meta)
    m=canonical(meta)
    assert isinstance(request, bytes) and isinstance(response, bytes)
    assert len(m)<65536 and len(request)+len(response)<=MAX_RECORD
    assert HEADER.size+len(m)+len(request)+len(response)<=MAX_CHUNK
    return HEADER.pack(MAGIC,len(m),len(request),len(response),zlib.crc32(request+response),
                       hashlib.sha256(request).digest(),hashlib.sha256(response).digest())+m+request+response

def unpack(buf):
    off=0
    while off<len(buf):
        assert len(buf)-off>=HEADER.size, 'truncated Q3R2 header'
        magic,ml,ql,rl,crc,qsha,rsha=HEADER.unpack_from(buf,off)
        assert magic==MAGIC and ml<65536 and ql+rl<=MAX_RECORD
        end=off+HEADER.size+ml+ql+rl
        assert end<=len(buf), 'truncated Q3R2 payload'
        at=off+HEADER.size
        m=buf[at:at+ml];q=buf[at+ml:at+ml+ql];r=buf[at+ml+ql:end]
        assert zlib.crc32(q+r)==crc and hashlib.sha256(q).digest()==qsha and hashlib.sha256(r).digest()==rsha
        meta=json.loads(m)
        assert canonical(meta)==m
        valid_meta(meta)
        yield meta,q,r
        off=end

def verify(folder):
    folder=Path(folder);manifest=folder/'archive-manifest.jsonl';assert manifest.is_file()
    manifest_bytes=manifest.read_bytes()
    assert manifest_bytes.endswith(b'\n'),'manifest missing final LF'
    lines=manifest_bytes[:-1].split(b'\n');assert lines and all(lines)
    serial=0;chunk_count=0;total_wire=0;complete=0;partial=0;aggregate=hashlib.sha256()
    expected_files=set()
    for line in lines:
        item=json.loads(line);assert canonical(item)==line
        assert set(item)=={'chunk','file','format','first_serial','last_serial','records','uncompressed_bytes','uncompressed_sha256','compressed_bytes','compressed_sha256'}
        for k in ('chunk','first_serial','last_serial','records','uncompressed_bytes','compressed_bytes'):
            assert type(item[k]) is int and item[k]>0,k
        assert item['records']<=256 and item['uncompressed_bytes']<=MAX_CHUNK
        chunk_count+=1;assert item['chunk']==chunk_count and item['format']=='Q3R2/zstd-independent-256-or-16MiB/1'
        name=f'archive-chunk-{chunk_count:06}.zst';expected_files.add(name)
        assert item['file']==name and item['first_serial']==serial+1
        data=(folder/name).read_bytes();assert len(data)==item['compressed_bytes'] and sha(data)==item['compressed_sha256']
        dec=subprocess.run(['zstd','-q','-dc'],input=data,capture_output=True,check=True,timeout=30)
        raw=dec.stdout;assert len(raw)==item['uncompressed_bytes'] and len(raw)<=MAX_CHUNK and sha(raw)==item['uncompressed_sha256']
        seen=0
        for meta,q,r in unpack(raw):
            serial+=1;seen+=1;total_wire+=len(q)+len(r);aggregate.update(pack(meta,q,r))
            assert meta['serial']==serial
            if meta['complete']:complete+=1
            else:partial+=1
        assert seen==item['records'] and item['last_serial']==serial and 0<seen<=256
    existing={p.name for p in folder.glob('archive-chunk-*.zst')}
    assert existing==expected_files,('unmanifested or missing chunk',existing^expected_files)
    assert not list(folder.glob('archive-chunk-*.part')), 'uncommitted chunk remains'
    return {'status':'PASS_ARCHIVE_BYTES_ONLY','chunks':chunk_count,'records':serial,'complete':complete,'partial':partial,
            'request_plus_response_bytes':total_wire,'packed_sha256':aggregate.hexdigest(),
            'manifest_sha256':sha(manifest.read_bytes())}

def prefix_test(old_gzip,out):
    """Frozen failure prefix only. A missing gzip trailer is expected and never a business PASS."""
    out=Path(out);out.mkdir(exist_ok=False)
    index=0;chunks=[];batch=[];total=0;old_tail=None
    def flush():
        nonlocal batch,total
        if not batch:return
        raw=b''.join(batch);assert len(raw)<=MAX_CHUNK
        enc=subprocess.run(['zstd','-q','-c','-3','--long=27','--check'],input=raw,capture_output=True,check=True,timeout=30)
        dec=subprocess.run(['zstd','-q','-dc'],input=enc.stdout,capture_output=True,check=True,timeout=30)
        assert dec.stdout==raw
        n=len(chunks)+1;name=f'archive-chunk-{n:06}.zst';(out/name).write_bytes(enc.stdout)
        last=sum(x['records'] for x in chunks)+len(batch)
        chunks.append({'chunk':n,'file':name,'format':'Q3R2/zstd-independent-256-or-16MiB/1','first_serial':last-len(batch)+1,
                       'last_serial':last,'records':len(batch),'uncompressed_bytes':len(raw),'uncompressed_sha256':sha(raw),
                       'compressed_bytes':len(enc.stdout),'compressed_sha256':sha(enc.stdout)})
        total+=len(enc.stdout);batch=[]
    try:
        with gzip.open(old_gzip,'rb') as f:
            for line in f:
                x=json.loads(line);index+=1;assert x['serial']==index
                q=base64.b64decode(x['request_base64']);r=base64.b64decode(x['response_base64'])
                assert sha(q)==x['request_sha256'] and sha(r)==x['response_wire_sha256']
                meta={'serial':index,'complete':True,'error':None,'op':x['op'],'code':x['code']}
                packed=pack(meta,q,r)
                if batch and (len(batch)==256 or sum(map(len,batch))+len(packed)>16*1024*1024):flush()
                batch.append(packed)
    except EOFError:old_tail='EOFError'
    flush()
    with (out/'archive-manifest.jsonl').open('xb') as f:
        for item in chunks:f.write(canonical(item)+b'\n')
    checked=verify(out)
    assert checked['records']==index
    return {'scope':'old failed run readable prefix only','old_gzip_tail_error':old_tail,'readable_complete_records':index,
            'compressed_bytes':total,'archive':checked,'business_closure':'NOT_CLAIMED'}

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('--verify');ap.add_argument('--prefix-test',nargs=2,metavar=('OLD_GZIP','OUT_DIR'))
    a=ap.parse_args();assert bool(a.verify)!=bool(a.prefix_test)
    print(json.dumps(verify(a.verify) if a.verify else prefix_test(*a.prefix_test),ensure_ascii=False,sort_keys=True))
