#!/usr/bin/env python3
"""DP68-v1: reuse reviewed prefixes, never execute R_W. Explicit data-dependency audit."""
from __future__ import annotations
import hashlib, json, resource, sys, time
from collections import Counter
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = HERE.parents[1]
R = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')
SOURCE = E/'stage66/nested-author/run/source.json'
PREFIX = E/'stage66/nested-review/independent-prefixes.json'
ROWS67 = E/'stage67/live-review/independent-prefix-results.jsonl'
FINALS = [E/'stage66/nested-review'/n for n in ('independent-pens.json','independent-objects.json','independent-parent.json')]
CERTS = E/'stage67/live-review/independent-certificates.json'
NAMES = ['P0','P1','P2','c0']
OUT = Path(sys.argv[1]).resolve() if len(sys.argv)>1 else HERE/'run'
OUT.mkdir(parents=True, exist_ok=False)
READS=[]

def dump(name, value):
    (OUT/name).write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')

def sha(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(65536),b''): h.update(b)
    return h.hexdigest()

def load(path, phase):
    READS.append({'path':str(path),'action':'whole-json-deserialize','phase':phase})
    return json.loads(path.read_text())

def items(path):
    READS.append({'path':str(path),'action':'array-item-stream','phase':'online','byte_buffer':65536})
    d=json.JSONDecoder(); buf=''; started=False; eof=False
    with path.open() as f:
        while True:
            buf=buf.lstrip()
            if not started and buf:
                assert buf[0]=='[';buf=buf[1:];started=True
            buf=buf.lstrip().lstrip(',').lstrip()
            if buf.startswith(']'): return
            try:v,n=d.raw_decode(buf)
            except json.JSONDecodeError:
                if eof:raise
                chunk=f.read(65536);eof=not chunk;buf+=chunk;continue
            buf=buf[n:];yield v

def pack(v):
    return None if v is None else {'num':v.numerator,'den':v.denominator}

def sig(p):return (p['start'],p['end'],p['start_price'],p['end_price'],p['up'])
def key(p):return ':'.join(map(str,sig(p)))

def signed_direction(up):return 1 if up else -1

class Machine:
    def __init__(self):
        self.prices=[];self.times=[];self.previous_stable=[];self.previous_active=None
        self.versions={};self.lookup={};self.events=[];self.current=None;self.anchor=None
        self.first_stable={};self.confirmed={};self.stable={};self.geometry=[]
        self.raw={};self.wholes={};self.weak_current={};self.weak_events=[];self.relations=[]
        self.init=[];self.counts=Counter();self.births=[];self.readback=[]
    def velocity(self,p):return Fraction(p['end_price']-p['start_price'],self.times[p['end']]-self.times[p['start']])
    def emit(self,t,kind,**kw):self.events.append({'knownAt':t,'kind':kind,**kw})
    def version(self,a,t):
        start=a['raw'];up=not a['top'];duration=self.times[t]-self.times[start]
        disp=self.prices[t]-self.prices[start];sgn=signed_direction(up)
        span=self.prices[start:t+1];is_extreme=sgn*self.prices[t]==max(sgn*x for x in span)
        if duration<=0: state='zero-duration'
        elif sgn*disp<=0:state='direction-invalid'
        elif not is_extreme:state='endpoint-not-current-extreme'
        elif t-start<4:state='waiting-spacing'
        else:state='pending-opposite-fractal'
        p={'start':start,'end':t,'start_price':self.prices[start],'end_price':self.prices[t],'up':up}
        return {'id':f'DP:{start}@{a["known_at"]}:v{t}', 'family':f'DP:{start}@{a["known_at"]}',
                'version':t,'observedAt':t,'anchorKnownAt':a['known_at'],'startFractal':dict(a),
                **p,'direction':'Up' if up else 'Down','evidenceState':state,
                'extendable':state in ('waiting-spacing','pending-opposite-fractal'),
                'terminalFractalKnownAt':None,'duration':duration,
                'v':pack(Fraction(disp,duration)) if duration>0 else None,
                'StructuralPenNow_original':None,'source_pointbar_mapping_proven':None}
    def certify_geometry(self,p,t,anchors):
        a,e=p['start'],p['end'];a0=next(x for x in anchors if x['raw']==a);e0=next(x for x in anchors if x['raw']==e)
        def fractal(x,top):
            return (self.prices[x]>self.prices[x-1] and self.prices[x]>self.prices[x+1]) if top else (self.prices[x]<self.prices[x-1] and self.prices[x]<self.prices[x+1])
        assertions={'start_strict_fractal':fractal(a,a0['top']),'end_strict_fractal':fractal(e,e0['top']),
                    'opposite':a0['top']!=e0['top'],'direction':p['up']==(not a0['top']),
                    'nonshared_support':a+1<e-1,'independent_pointbar':e-a>=4,
                    'raw_between_at_least_three':e-a-1>=3,
                    'adjacent_surviving_anchors':anchors.index(e0)==anchors.index(a0)+1,
                    'all_pointbars_distinct_adjacent':all(self.prices[i]!=self.prices[i-1] for i in range(a,e+2)),
                    'end_known_by_cut':e0['known_at']<=t and e+1<=t,
                    'range_extremes':sorted((p['start_price'],p['end_price']))==[min(self.prices[a:e+1]),max(self.prices[a:e+1])]}
        assert all(assertions.values()),(t,p,assertions)
        return {'pen':p,'knownAt':t,'start_support':[a-1,a,a+1],'end_support':[e-1,e,e+1],
                'pointbar_source_shape':assertions,'source_admission':None}
    def step(self,obs,prefix,oldrows):
        t=prefix['cut'];assert t==len(self.prices)==obs['index']==obs['time']
        self.prices.append(obs['price']);self.times.append(obs['time'])
        st,act,anchors=prefix['stable'],prefix['active'],prefix['anchors']
        assert st[:len(self.previous_stable)]==self.previous_stable
        assert prefix['stable_count']==len(st)
        for a in anchors:
            assert a['raw']+1<=t and a['known_at']<=t and self.prices[a['raw']]==a['price']
        newly_stable=st[len(self.previous_stable):]
        for p in newly_stable:
            k=key(p);assert k in self.confirmed
            immutable={'pen':p,'knownAt':t,'candidateVersion':self.confirmed[k]['candidateVersion']}
            self.stable[k]=immutable;self.first_stable.setdefault(p['start'],t)
            self.emit(t,'stable-rw',**immutable)
        if self.previous_active and (not act or sig(act)!=sig(self.previous_active)) and key(self.previous_active) not in self.stable:
            self.emit(t,'retracted-confirmed-rw',pen=self.previous_active,reason='active-identity-replaced')
        if act and key(act) not in self.confirmed:
            k=key(act);v=self.lookup.get(k);assert v is not None,(t,act)
            assert self.versions[v]['evidenceState']=='pending-opposite-fractal'
            geom=self.certify_geometry(act,t,anchors);self.geometry.append(geom)
            cert={'pen':dict(act),'knownAt':t,'candidateVersion':v,'geometryIndex':len(self.geometry)-1}
            self.confirmed[k]=cert;self.emit(t,'confirmed-rw-active',**cert)
        if self.current:
            previous=self.versions[self.current];k=key(previous)
            if k not in self.confirmed:
                reason='anchor-replaced' if anchors and anchors[-1]['raw']!=previous['start'] else 'endpoint-version-superseded'
                self.emit(t,'retracted-proposal',version=self.current,reason=reason)
        if not anchors:
            current={'state':'no-known-anchor','v':None,'reason':'no-known-anchor','StructuralPenNow_original':None}
            self.init.append({'cut':t,**current})
        else:
            a=anchors[-1]
            if self.anchor!=a['raw']:
                self.births.append({'knownAt':t,'anchor':a['raw'],'end':a['raw'],'state':'zero-duration','v':None,
                                    'reason':'initial-end-equals-start-before-attaching-current-observation'})
                self.anchor=a['raw']
            current=self.version(a,t);self.current=current['id'];self.versions[self.current]=current
            self.lookup[key(current)]=self.current;self.emit(t,'proposed',version=self.current)
            self.counts[current['evidenceState']]+=1
        # A completed endpoint occupies an immutable old span. Current residual gets its own template slot.
        pens=st+([act] if act else [])
        for block in range((len(pens)+4)//5):
            first=5*block
            if first>=len(anchors):continue
            start=anchors[first]['raw'];chunk=pens[first:first+5]
            obj=self.raw.setdefault(block,{'id':f'Raw:{block}','start':start,'candidateFirstKnownAt':t,'end':None,'state':'open'})
            assert obj['start']==start
            if len(chunk)==5 and obj['end'] is None:
                obj.update(end=chunk[-1]['end'],endpointKnownAt=t,state='ended-awaiting-stability',terminalCandidate=self.lookup[key(chunk[-1])])
            if len(st)>=first+5 and 'frozenL' not in obj:
                force=self.velocity(chunk[-1])-self.velocity(chunk[0]);obj.update(state='frozen-pens-awaiting-raw-proof',frozenL=pack(force),frozenAt=t)
                self.emit(t,'freeze-raw-pen-span',raw=dict(obj))
        for g,name in enumerate(NAMES):
            first=5+25*g
            if first>=len(anchors):continue
            start=anchors[first]['raw'];chunk=pens[first:first+25]
            obj=self.wholes.setdefault(name,{'id':name,'start':start,'candidateFirstKnownAt':t,'end':None,'state':'open',
                                            'Completed_original':None,'Owner_original':None,'Next_original':None,'F2_original':None})
            if len(chunk)==25 and obj['end'] is None:
                obj.update(end=chunk[-1]['end'],endpointKnownAt=t,state='ended-awaiting-stability',terminalCandidate=self.lookup[key(chunk[-1])])
                self.emit(t,'whole-endpoint-proposal-confirmed-rw',whole=dict(obj))
            if len(st)>=first+25 and 'wholeFrozenL' not in obj:
                obj.update(state='frozen-pens-awaiting-raw-proof',frozenAt=t,
                           wholeFrozenL=pack(self.velocity(chunk[-1])-self.velocity(chunk[0])),
                           cFrozenL=pack(self.velocity(chunk[-1])-self.velocity(chunk[-5])))
                self.emit(t,'freeze-whole-pen-span',whole=dict(obj))
            row=oldrows[name]
            if row['lifecycle']=='closed-local-certificate' and 'rawEvidenceKnownAt' not in obj:
                assert obj['end'] is not None and 'cFrozenL' in obj
                obj.update(state='selected-raw-evidence-complete',rawEvidenceKnownAt=t)
                self.raw[5+5*g].update(rawEvidenceKnownAt=t,state='selected-raw-evidence-complete')
                self.emit(t,'selected-whole-raw-evidence-complete',whole=dict(obj),basis='reviewed-Stage67-current-row')
        # Domain-aware proposal: once the selected whole endpoint is known, do not put later residual in it.
        force_rows=[]
        for g,name in enumerate(NAMES+['parent']):
            row=oldrows[name];base=25+25*g if name!='parent' else 80
            start=anchors[base]['raw'] if len(anchors)>base else None
            first=next((p for p in st if p['start']==start),None)
            obj=self.wholes.get(name if name!='parent' else 'c0')
            ended=bool(obj and obj['end'] is not None)
            if start is None:reason='start-anchor-not-known';value=None
            elif first is None:reason='first-pen-not-stable';value=None
            elif ended:reason='old-span-ended-new-residual-has-separate-template-slot';value=None
            elif not anchors or current.get('start',-1)<start:reason='no-current-candidate-in-span';value=None
            elif current.get('duration',0)<=0:reason='zero-duration';value=None
            else:reason=None;value=Fraction(current['v']['num'],current['v']['den'])-self.velocity(first)
            proposed=pack(value)
            eligible=bool(row['local_role_gate'] and not ended and value is not None and row['extreme'] is True and value<Fraction(**{'numerator':row['b_L']['num'],'denominator':row['b_L']['den']}))
            version=current.get('id') if value is not None else None
            old=self.weak_current.get(name)
            if old and (not eligible or old['version']!=version):
                self.weak_events.append({'cut':t,'id':name,'kind':'cancel-endpoint-completion-candidate','candidate':old,
                                         'reason':'selected-span-ended' if ended else ('endpoint-version-superseded' if eligible else 'predicate-no-longer-true')})
                self.weak_current.pop(name,None)
            if eligible:
                new={'version':version,'end':t,'knownAt':t,'L':proposed,'Completed_original':None}
                self.weak_current[name]=new;self.weak_events.append({'cut':t,'id':name,'kind':'propose-endpoint-completion-candidate','candidate':new})
            frozen=(obj.get('wholeFrozenL') if name=='parent' else obj.get('cFrozenL')) if obj else None
            force_rows.append({'id':name,'cStart':start,'firstStableAt':self.first_stable.get(start),'proposedL':proposed,
                               'reason':reason,'unitVersion':version,'unitState':current.get('evidenceState'),
                               'jointCandidate':eligible,'oldSpanEnd':obj.get('end') if obj else None,
                               'frozenL':frozen,'frozenAt':obj.get('frozenAt') if obj else None,
                               'sourceL':None,'StructuralPenNow_original':None,'Completed_original':None,
                               'rawEvidenceKnownAt':obj.get('rawEvidenceKnownAt') if obj else None,
                               'stage67R':row['R'].get('L'),'stage67JointCandidate':row['R']['complete_candidate']})
        out={'cut':t,'price':self.prices[t],'current':current,'active':act,'stableCount':len(st),
             'activeCandidateVersion':self.confirmed[key(act)]['candidateVersion'] if act else None,
             'currentSameAsActive':bool(act and current.get('id') and sig(act)==sig(current)),
             'currentSameAsAnyStable':bool(current.get('id') and key(current) in self.stable),
             'templateCurrentRawSlot':len(anchors)-1 if anchors else None,
             'force':force_rows,'wholes':json.loads(json.dumps(self.wholes))}
        self.relations.append(out);self.previous_stable=st;self.previous_active=act
        return out

def main():
    started=time.perf_counter();inputs=[SOURCE,PREFIX,ROWS67,HERE/'WorkCard.md',HERE/'check_pen.py']+FINALS+[CERTS]
    before={str(p):sha(p) for p in inputs};dump('inputhash-before.json',before)
    READS.append({'action':'sha256-byte-precheck','phase':'pre-online','paths':[str(p) for p in inputs],
                  'final_files_bytes_read':True,'final_files_json_deserialized':False})
    source=load(SOURCE,'pre-online');machine=Machine();rows_count=0
    READS.append({'path':str(ROWS67),'action':'jsonl-row-stream','phase':'online','future_json_rows_deserialized':False})
    with ROWS67.open() as prior,(OUT/'all-prefixes.jsonl').open('w') as output:
        for t,prefix in enumerate(items(PREFIX)):
            rows={}
            for _ in range(5):
                row=json.loads(next(prior));assert row['cut']==t;rows[row['id']]=row
            assert set(rows)==set(NAMES+['parent'])
            observation=source['observations'][t]
            result=machine.step(observation,prefix,rows);output.write(json.dumps(result,ensure_ascii=False,separators=(',',':'))+'\n');rows_count+=1
        assert not prior.read().strip()
    assert rows_count==443
    for name,data in [('events.json',machine.events),('versions.json',list(machine.versions.values())),
                      ('completion-candidates.json',machine.weak_events),('pointbar-geometry.json',machine.geometry),
                      ('initialization.json',{'noAnchor':machine.init,'zeroDurationBirths':machine.births}),
                      ('raw-spans.json',list(machine.raw.values())),('whole-spans.json',machine.wholes)]:dump(name,data)
    seal={'prefixes':443,'hashes':{n:sha(OUT/n) for n in ['all-prefixes.jsonl','events.json','versions.json','completion-candidates.json','pointbar-geometry.json','raw-spans.json','whole-spans.json']},
          'final_json_deserialized_before_seal':False,'final_bytes_prehashed':True,'driver_source_array_preloaded':True,
          'prefix_array_byte_read_ahead':65536,'stage67_current_rows_streamed':True,
          'step_input':'one observation, one reviewed prefix, five reviewed Stage67 current-cut rows',
          'claim':'conditional step dependency only; no claim of physically absent future bytes or blind author'}
    dump('online-seal.json',seal);READS.append({'action':'seal-written','phase':'online-end','hash':sha(OUT/'online-seal.json')})
    pens,objects,parent=[load(p,'post-seal') for p in FINALS];certs=load(CERTS,'post-seal')
    assert len(pens)==110
    # Prefix final value equality is purely a post-seal oracle comparison.
    pen_fields=['up','start','end','start_price','end_price']
    comparable=lambda p:{k:p[k] for k in pen_fields}
    assert [comparable(p) for p in machine.previous_stable]+([comparable(machine.previous_active)] if machine.previous_active else [])==[comparable(p) for p in pens]
    compares=[]
    for old in objects:
        obj=machine.wholes[old['id']];assert obj['end']==old['end']
        assert Fraction(obj['cFrozenL']['num'],obj['cFrozenL']['den'])==old['c_force']['L']
        cert=next(c for c in certs if c['block']==5+5*NAMES.index(old['id']))
        assert cert['known_at']==obj['rawEvidenceKnownAt'] and cert['end']==obj['end']
        compares.append({'id':old['id'],'end':obj['end'],'endpointKnownAt':obj['endpointKnownAt'],'frozenAt':obj['frozenAt'],'rawEvidenceKnownAt':obj['rawEvidenceKnownAt'],'cFrozenL':obj['cFrozenL'],'wholeFrozenL':obj['wholeFrozenL'],'match':True})
    assert machine.wholes['c0']['wholeFrozenL']=={'num':-3500,'den':1}
    # Exact elementary lemma, instantiated at a real opposing turn.
    a,r,t=113,117,121;p=machine.prices;vold=Fraction(p[r]-p[a],r-a);vr=Fraction(p[t]-p[r],t-r);va=Fraction(p[t]-p[a],t-a)
    weighted=Fraction(r-a,t-a)*vold+Fraction(t-r,t-a)*vr;assert va==weighted and va!=vr
    lemma={'a':a,'r':r,'t':t,'oldVelocity':pack(vold),'residualVelocity':pack(vr),'chordVelocity':pack(va),'weighted':pack(weighted),'equal':va==vr}
    keys=[0,1,2,5,6,10,102,105,106,109,110,117,118,119,120,121,122,126,138,217,218,220,221,222,226,238,417,418,420,421,422,426,438,442]
    dump('key-trajectories.json',[machine.relations[t] for t in keys]);dump('post-seal-comparison.json',compares);dump('weighted-lemma.json',lemma)
    after={str(p):sha(p) for p in inputs};dump('inputhash-after.json',after);assert before==after
    dump('reading-order.json',READS)
    rss=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss*(1 if sys.platform=='darwin' else 1024);assert rss<=96*1024*1024
    status={'status':'finite-state-machine-author-self-check-pass','prefixes':443,'new_histories':0,'rw_runs':0,
            'confirmed_count':len(machine.confirmed),'stable_count':len(machine.stable),'version_count':len(machine.versions),
            'proposal_states':dict(machine.counts),'retraction_events':sum(e['kind']=='retracted-proposal' for e in machine.events),
            'active_retraction_events':sum(e['kind']=='retracted-confirmed-rw' for e in machine.events),
            'pointbar_geometry_checked':len(machine.geometry),'no_anchor_states':len(machine.init),'zero_duration_births':len(machine.births),
            'current_equals_active_count':sum(x['currentSameAsActive'] for x in machine.relations),
            'current_equals_stable_count':sum(x['currentSameAsAnyStable'] for x in machine.relations),
            'whole_comparison':compares,'peak_rss_bytes':rss,'elapsed_seconds':time.perf_counter()-started,
            'original_StructuralPenNow':None,'original_GeneralDiv':None,'original_Completed':None,'RootArm67':None,'Owner':None,'Next':None,'F2':None,
            'source_pointbar_bridge':None,'full_F1':None,'independent_review':'pending',
            'scope':'one selected template on one existing synthetic history; finite conditional geometry and lifecycle only'}
    dump('machine-status.json',status);print(json.dumps(status,ensure_ascii=False))

if __name__=='__main__':main()
