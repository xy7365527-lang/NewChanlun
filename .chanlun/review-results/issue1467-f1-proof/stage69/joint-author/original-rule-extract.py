def force(seg, pens):
    a,z=seg['members']; first,last=pens[a],pens[z-1]
    fv=(first['end_price']-first['start_price'])/(first['end']-first['start'])
    lv=(last['end_price']-last['start_price'])/(last['end']-last['start'])
    return {'first_pen':a,'last_pen':z-1,'first_v':fv,'last_v':lv,'L':lv-fv}

def meet(ranges):
    return [max(x[0] for x in ranges), min(x[1] for x in ranges)]

def f2_contract(path, allow_pp):
    """Frozen necessary F2 interface relation, no production implementation claim."""
    pp=[(a['id'],b['id']) for a,b in zip(path,path[1:])
        if a['grade']==b['grade'] and a['kind']==b['kind']=='P']
    row={'delta':[x['id'] for x in path], 'kinds':[x['kind'] for x in path],
         'no_pp':not pp, 'pp_violations':pp, 'has_three':len(path)>=3,
         'counterfactual_no_pp_removed':allow_pp,
         'candidate_member_table':[], 'candidate_center':None,
         'candidate_interface_pass':False, 'original_interface_pass':False,
         'original_completion_status':'unknown', 'reasons':[]}
    if not path: row['reasons'].append('no_nonempty_candidate_delta')
    if not allow_pp and pp: row['reasons'].append('NoPP')
    if len(path)<3: row['reasons'].append('fewer_than_three_completed_candidate_children')
    else:
        first=path[:3]; core=meet([x['whole'] for x in first])
        continuous=all(a['end']==b['start'] for a,b in zip(first,first[1:]))
        alternating=first[0]['technical_up']!=first[1]['technical_up'] and first[1]['technical_up']!=first[2]['technical_up']
        samegrade=len({x['grade'] for x in first})==1
        row['candidate_member_table']=[{'position':i,'id':x['id'],'whole':x['whole'],
            'own':x['own'],'grade':x['grade'],'candidate_completed':x['JEnd68'],
            'original_completed':None,'MemberNext':first[i+1]['id'] if i<2 else None}
            for i,x in enumerate(first)]
        row['candidate_center']={'core':core,'strict':core[0]<core[1],
            'continuous':continuous,'directions_alternate':alternating,'same_grade':samegrade,
            'original_qualified':None}
        for name,ok in [('positive_core',core[0]<core[1]),('continuous',continuous),
                        ('direction_alternation',alternating),('same_grade',samegrade)]:
            if not ok: row['reasons'].append(name)
        row['candidate_interface_pass']=not row['reasons']
    row['original_reasons']=['OriginalCompleted(child) is unknown',
        'RootArm68 source compatibility unknown',
        'Outer0=core interpretation as original outer unknown',
        'JEnd68 => whole MovementCompleted not proved']
    return row

windows=[]
for s in range(1,21):
    k=1
    while s+4*k<21:
        z=s+4*k+1; parts=segments[s:z]; c=parts[-1]; bref=parts[-5]
        cores=[]
        for j in range(k):
            members=parts[4*j+1:4*j+4]; core=meet([x['range'] for x in members])
            dirs=[x['up'] for x in members]
            cores.append({'id':f'IC68:{s}:{j}', 'members':[x['id'] for x in members],
                'core':core,'Outer0':core,'Outer0_original_DD_GG':None,
                'own':[members[0]['start']+1,members[-1]['end']],
                'strict':core[0]<core[1],
                'direction_alternates':dirs[0]!=dirs[1] and dirs[1]!=dirs[2],
                'happened':members[-1]['end'], 'raw_known':max(x['known_at'] for x in members)})
        up_all=all(b['core'][0]>a['core'][1] for a,b in zip(cores,cores[1:]))
        down_all=all(b['core'][1]<a['core'][0] for a,b in zip(cores,cores[1:]))
        kind='P' if k==1 else ('U' if up_all else 'D' if down_all else None)
        typed=kind is not None and (k==1 or (kind=='U')==c['up'])
        core=cores[-1]['core']
        if c['up']:
            entry_cross=bref['start_price']<core[0]<=bref['end_price']
            exit_cross=c['start_price']<=core[1]<c['end_price']
        else:
            entry_cross=bref['start_price']>core[1]>=bref['end_price']
            exit_cross=c['start_price']>=core[0]>c['end_price']
        past=obs[parts[0]['start']:c['start']+1]
        extreme=max(o['price'] for o in past) if c['up'] else min(o['price'] for o in past)
        is_extreme=c['end_price']>extreme if c['up'] else c['end_price']<extreme
        weak=c['force']['L']<bref['force']['L']
        structural=all(x['strict'] and x['direction_alternates'] for x in cores) and typed
        roles=bref['up']==c['up'] and entry_cross and exit_cross
        relation_checks={'strict_cores':all(x['strict'] for x in cores),
            'member_direction_alternates':all(x['direction_alternates'] for x in cores),
            'coherent_type':typed,'same_direction_comparison_arms':bref['up']==c['up'],
            'entry_cross':entry_cross,'exit_cross':exit_cross,'whole_extreme':is_extreme,
            'signed_force_weakening':weak}
        accepted=structural and roles and is_extreme and weak
        owned=list(range(parts[0]['start']+1,c['end']+1))
        union=[n for x in parts for n in range(x['start']+1,x['end']+1)]
        assert union==owned and len(union)==len(set(union))
        record={'id':f'W68:S{s}-S{z-1}', 'raw_start':s,'raw_stop':z,'k':k,'grade':0,
            'kind':kind,'conceptual_direction':None if kind=='P' else kind,
            'technical_up':c['up'],'raws':[x['id'] for x in parts], 'cores':cores,
            'start':parts[0]['start'],'end':c['end'],'own':[owned[0],owned[-1]],
            'whole':[min(o['price'] for o in obs[parts[0]['start']:c['end']+1]),max(o['price'] for o in obs[parts[0]['start']:c['end']+1])],
            'b_ref':bref['id'],'c':c['id'],'b_force':bref['force'],'c_force':c['force'],
            'prior_whole_extreme':extreme,'end_price':c['end_price'],
            'candidate_happened':c['end'],'candidate_known':max(x['known_at'] for x in parts),
            'structural_pass':structural,'role_pass':roles,'CycleDiv68':structural and roles and is_extreme and weak,
            'RawEnd68':True,'JEnd68':accepted,
            'OriginalGeneralDiv':None,'OriginalMovementCompleted':None,
            'original_happened':None,'original_known':None,'original_published':None,
            'checks':relation_checks,'rejections':[name for name,ok in relation_checks.items() if not ok],
            'owns_each_event_once':True}
        windows.append(record);k+=1
