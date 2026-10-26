//! #1527：mixed旧审计的持费索引；观察者仅由本次调用借入，不进入缓存。
use super::*;
use crate::v3::{
    audit_budget::{LegacyObserver, LegacyRecordRef},
    audit_store::{NativeAdmission, NativeSet, NativeTree},
    business_json::{scoped_legacy, MixedJsonError, PlainJson},
    native_shared::NativeArc,
};
use newchan_rust::s_content::capture_allocation::{native_owned::NativeValue, scratch::WorkScope};

type Text = NativeValue<String>;
type Shared = NativeArc<PlainJson>;
pub(crate) struct Family {
    name: &'static str,
    records: NativeTree<Text, (Shared, Text, i64)>,
    born: NativeTree<i64, NativeSet<Text>>,
    seen: NativeSet<Text>,
}
fn sql_error(error: scoped_legacy::MixedSqlError) -> MixedJsonError {
    match error {
        scoped_legacy::MixedSqlError::Sql(error) => MixedJsonError::Business(error.to_string()),
        scoped_legacy::MixedSqlError::Projection(error) => error,
    }
}
fn identity(value:&Value, family:&str, work:&WorkScope)->Result<Text,MixedJsonError>{
    if family=="relations" {
        return Text::try_format(format_args!("{value}"),Some(work)).map_err(MixedJsonError::Resource);
    }
    let field=match family {"witnesses"=>"witness_id","observations"=>"observation_id",_=>"object_id"};
    Text::try_from_str(value[field].as_str().ok_or("StorageUnavailable：索引身份缺失")?,Some(work))
        .map_err(MixedJsonError::Resource)
}
impl Family {
    pub(crate) fn capture(conn:&Connection,name:&'static str,work:&WorkScope,observer:&dyn LegacyObserver)
        ->Result<Self,MixedJsonError>
    {
        observer.check_work(work).map_err(MixedJsonError::Resource)?;
        let sql=match name {
            "witnesses"=>WITNESSES_SQL,"relations"=>RELATIONS_SQL,"observations"=>OBSERVATIONS_SQL,
            _=>return Err("StorageUnavailable：未知索引族".into()),
        };
        let mut result=Self{name,records:Default::default(),born:Default::default(),seen:Default::default()};
        let mut query=conn.prepare(sql).map_err(|e|MixedJsonError::Business(e.to_string()))?;
        let mut rows=query.query([]).map_err(|e|MixedJsonError::Business(e.to_string()))?;
        let admission=NativeAdmission(Some(work));
        while let Some(row)=rows.next().map_err(|e|MixedJsonError::Business(e.to_string()))? {
            let (id,g,record)=scoped_legacy::project_family_row(row,name,work).map_err(sql_error)?;
            observer.visit_record(name,"index-capture",LegacyRecordRef::Json(&record)).map_err(MixedJsonError::Resource)?;
            if identity(&record,name,work)?!=id {
                return Err(format!("StorageUnavailable：{name} 独立索引身份与其记录不一致").into());
            }
            if result.records.contains_key(id.as_ref().as_str()) {
                return Err(format!("StorageUnavailable：{name} 重复身份").into());
            }
            if !result.born.contains_key(&g) {
                result.born.insert_admitted(g,NativeSet::default(),&admission).map_err(MixedJsonError::Resource)?;
            }
            result.born.get_mut(&g).unwrap().insert_admitted(id.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,&admission)
                .map_err(MixedJsonError::Resource)?;
            let projected=scoped_legacy::legacy_payload(&record,work)?;
            let canonical=Text::try_format(format_args!("{}",projected.as_ref()),Some(work)).map_err(MixedJsonError::Resource)?;
            drop(projected);
            let record=NativeArc::try_new_shallow(record,Some(work)).map_err(MixedJsonError::Resource)?;
            result.records.insert_admitted(id,(record,canonical,g),&admission).map_err(MixedJsonError::Resource)?;
        }
        Ok(result)
    }
    pub(crate) fn verify<'a>(
        &mut self, delta:&Value, g:i64, mode:delta_storage::Mode,
        batch_records:impl IntoIterator<Item=&'a Value>, work:&WorkScope, observer:&dyn LegacyObserver,
    )->Result<(),MixedJsonError>{
        observer.check_work(work).map_err(MixedJsonError::Resource)?;
        let admission=NativeAdmission(Some(work));
        let mut present=NativeSet::default();
        for record in required_array(delta,self.name)? {
            observer.visit_record(self.name,"family-delta",LegacyRecordRef::Json(record)).map_err(MixedJsonError::Resource)?;
            let id=identity(record,self.name,work)?;
            if !present.insert_admitted(id.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,&admission).map_err(MixedJsonError::Resource)? {
                return Err(format!("StorageUnavailable：{} Delta 重复身份",self.name).into());
            }
            let (stored,canonical,born)=self.records.get(id.as_ref().as_str()).ok_or("StorageUnavailable：Delta 没有独立索引端点")?;
            observer.visit_record(self.name,"family-delta-index",LegacyRecordRef::Json(stored.as_ref())).map_err(MixedJsonError::Resource)?;
            let projected=scoped_legacy::legacy_payload(record,work)?;
            let actual=Text::try_format(format_args!("{}",projected.as_ref()),Some(work)).map_err(MixedJsonError::Resource)?;
            if *canonical!=actual || *born>g || (!self.seen.contains(id.as_ref().as_str()) && *born!=g) {
                return Err(format!("StorageUnavailable：{} 内容/首次发布代不一致",self.name).into());
            }
            self.seen.insert_admitted(id,&admission).map_err(MixedJsonError::Resource)?;
        }
        let mut batch_ids=NativeSet::default();
        for record in batch_records {
            observer.visit_record(self.name,"family-batch",LegacyRecordRef::Json(record)).map_err(MixedJsonError::Resource)?;
            let id=identity(record,self.name,work)?;
            if !batch_ids.insert_admitted(id.try_clone_admitted(work).map_err(MixedJsonError::Resource)?,&admission).map_err(MixedJsonError::Resource)? {
                return Err(format!("StorageUnavailable：{} 封存批次重复身份",self.name).into());
            }
            let (stored,canonical,born)=self.records.get(id.as_ref().as_str())
                .ok_or_else(||MixedJsonError::Business(format!("StorageUnavailable：{} 封存批次成员没有独立索引端点",self.name)))?;
            observer.visit_record(self.name,"family-batch-index",LegacyRecordRef::Json(stored.as_ref())).map_err(MixedJsonError::Resource)?;
            // 原短路：未来出生不执行规范投影。
            let differs=if *born>g {true} else {
                let projected=scoped_legacy::legacy_payload(record,work)?;
                let actual=Text::try_format(format_args!("{}",projected.as_ref()),Some(work)).map_err(MixedJsonError::Resource)?;
                *canonical!=actual
            };
            if differs {return Err(format!("StorageUnavailable：{} 封存批次内容/发布代与独立索引不一致",self.name).into());}
        }
        let empty=NativeSet::default();
        let born=self.born.get(&g).unwrap_or(&empty);
        if mode.changes_only() {
            if !present.iter().eq(born.iter()) {
                return Err(format!("StorageUnavailable：{} 本代 Delta 不等于本代首次发布集（漏 birth 或伪旧 member）",self.name).into());
            }
            if !born.iter().all(|id|batch_ids.contains(id.as_ref().as_str())) {
                return Err(format!("StorageUnavailable：{} 封存批次缺本代首次成员",self.name).into());
            }
        } else if !born.iter().all(|id|present.contains(id.as_ref().as_str())) {
            return Err(format!("StorageUnavailable：{} 本代索引出生未出现在 Delta",self.name).into());
        }
        Ok(())
    }
    pub(crate) fn finish(&self)->Result<(),MixedJsonError>{
        if self.seen.len()!=self.records.len() {return Err(format!("StorageUnavailable：{} 存在不可达幽灵行",self.name).into());}
        Ok(())
    }
    pub(crate) fn len(&self)->usize {self.records.len()}
}

pub(crate) fn read_facts(
    conn:&Connection, as_of:Option<i64>, work:&WorkScope, observer:&dyn LegacyObserver,
)->Result<(newchan_rust::s_content::CodecVec<PlainJson>,newchan_rust::s_content::CodecVec<PlainJson>),MixedJsonError>{
    use scoped_legacy::{Field,ProjectionKind,ScopedLegacyRecord};
    use newchan_rust::s_content::CodecVec;
    observer.check_work(work).map_err(MixedJsonError::Resource)?;
    if !tb02a::enabled_admitted(conn,Some(work))? {return Ok((CodecVec::default(),CodecVec::default()));}
    let format=evidence_store::read_format_typed(conn,None,Some(work))?;
    let mut reader=evidence_store::Reader::new_mixed(conn,crate::EVIDENCE_READ_BUDGET,work,observer)?;
    let mut stmt=conn.prepare(tb02a::FACTS_READ_SQL).map_err(|e|MixedJsonError::Business(e.to_string()))?;
    let mut cursor=stmt.query([]).map_err(|e|MixedJsonError::Business(e.to_string()))?;
    let mut rows=CodecVec::default();
    while let Some(row)=cursor.next().map_err(|e|MixedJsonError::Business(e.to_string()))? {
        let row=scoped_legacy::collect_fact_row(row,work).map_err(sql_error)?;
        rows.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
        rows.try_push(row).map_err(MixedJsonError::Resource)?;
    }
    drop(cursor);
    let mut enriched=CodecVec::default();
    for row in rows {
        let row=if format.records_in_graph() {
            let object_id_owner=Text::try_from_str(row["object_id"].as_str().unwrap_or(""),Some(work)).map_err(MixedJsonError::Resource)?;
            let object_id=object_id_owner.as_ref();
            for (column,label) in [("payload_json","payload"),("input_refs_json","input_refs"),("source_coords_json","source_coords")] {
                if !row[column].as_str().unwrap_or("").is_empty() {
                    return Err(format!("StorageUnavailable：图域结构事实 `{object_id}` 行内 {label} 仍带整块原文，拒绝").into());
                }
            }
            let key=evidence_store::fact_root_key_mixed(&object_id,work)?;
            let bytes=reader.read_root_typed(&key,false)?.bytes;
            let what=Text::try_format(format_args!("事实 `{object_id}`"),Some(work)).map_err(MixedJsonError::Resource)?;
            let (payload,refs,coords)=evidence_store::split_fact_root_mixed(&bytes,&what,work)?;
            let mut projected=ScopedLegacyRecord::from_owned(ProjectionKind::LegacyPayload,row,work)?;
            projected.replace_text(Field::PayloadJson,&payload)?;
            projected.replace_text(Field::InputRefsJson,&refs)?;
            projected.replace_text(Field::SourceCoordsJson,&coords)?;
            projected.finish()
        } else {row};
        enriched.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
        enriched.try_push(row).map_err(MixedJsonError::Resource)?;
    }
    let mut active=CodecVec::default();
    let mut withdrawn=CodecVec::default();
    for row in enriched {
        observer.visit_record("objects","fact-lifecycle",LegacyRecordRef::Json(&row)).map_err(MixedJsonError::Resource)?;
        let Some(row)=scoped_legacy::filter_fact_lifecycle(row,as_of,work)? else {continue};
        let target=if row["withdrawn_generation"].is_null(){&mut active}else{&mut withdrawn};
        target.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
        target.try_push(row).map_err(MixedJsonError::Resource)?;
    }
    Ok((active,withdrawn))
}

pub(crate) fn read_objects(
    conn:&Connection, as_of:Option<i64>, work:&WorkScope, observer:&dyn LegacyObserver,
)->Result<(newchan_rust::s_content::CodecVec<PlainJson>,newchan_rust::s_content::CodecVec<PlainJson>),MixedJsonError>{
    use newchan_rust::s_content::CodecVec;
    observer.check_work(work).map_err(MixedJsonError::Resource)?;
    let invalid:i64=conn.query_row(
        "SELECT COUNT(*) FROM objects WHERE withdrawn_generation IS NOT NULL AND withdrawn_generation <= first_known_generation",
        [],|r|r.get(0)).map_err(|e|MixedJsonError::Business(format!("核对象生命周期失败：{e}")))?;
    if invalid!=0 {
        return Err("StorageUnavailable：对象生命周期非法（withdrawn_generation <= first_known_generation）".into());
    }
    let collect=|sql:&str,label:&str|->Result<CodecVec<PlainJson>,MixedJsonError>{
        let mut stmt=conn.prepare(sql).map_err(|e|MixedJsonError::Business(format!("prepare {label} 失败：{e}")))?;
        let mut rows=match as_of {None=>stmt.query([]),Some(n)=>stmt.query(params![n])}
            .map_err(|e|MixedJsonError::Business(format!("query {label} 失败：{e}")))?;
        let mut out=CodecVec::default();
        while let Some(row)=rows.next().map_err(|e|MixedJsonError::Business(format!("collect {label} 失败：{e}")))? {
            let value=scoped_legacy::project_object_row(row,as_of,work).map_err(|error|match error {
                scoped_legacy::MixedSqlError::Sql(e)=>MixedJsonError::Business(format!("collect {label} 失败：{e}")),
                scoped_legacy::MixedSqlError::Projection(e)=>e,
            })?;
            observer.visit_record("objects","object-sql-project",LegacyRecordRef::Json(&value)).map_err(MixedJsonError::Resource)?;
            out.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
            out.try_push(value).map_err(MixedJsonError::Resource)?;
        }
        Ok(out)
    };
    let mut objects=collect(if as_of.is_none(){crate::OBJECT_ACTIVE_SQL}else{crate::OBJECT_ACTIVE_ASOF_SQL},"objects")?;
    let mut withdrawn=collect(if as_of.is_none(){crate::OBJECT_DEAD_SQL}else{crate::OBJECT_DEAD_ASOF_SQL},"withdrawn")?;
    let (typed_active,typed_dead)=read_facts(conn,as_of,work,observer)?;
    for (source,target) in [(&typed_active,&mut objects),(&typed_dead,&mut withdrawn)] {
        // 原collect完整typed投影成功后才extend目标。
        let mut projected=CodecVec::default();
        for value in source {
            observer.visit_record("objects","fact-project",LegacyRecordRef::Json(value)).map_err(MixedJsonError::Resource)?;
            let value=scoped_legacy::project_fact(value,work)?;
            projected.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
            projected.try_push(value).map_err(MixedJsonError::Resource)?;
        }
        target.try_reserve_native(work,projected.len()).map_err(MixedJsonError::Resource)?;
        for value in projected {target.try_push(value).map_err(MixedJsonError::Resource)?;}
    }
    Ok((objects,withdrawn))
}

pub(crate) struct AuditWalk {
    active:NativeTree<Text,Shared>,
    births:NativeTree<i64,newchan_rust::s_content::CodecVec<Shared>>,
    deaths:NativeTree<i64,newchan_rust::s_content::CodecVec<Shared>>,
    witnesses:Family,
    relations:Family,
    observations:Family,
    original_observations:NativeTree<Text,Shared>,
    raw:newchan_rust::s_content::CodecVec<PlainJson>,
    objects:NativeTree<Text,Shared>,
}
fn push_event(
    tree:&mut NativeTree<i64,newchan_rust::s_content::CodecVec<Shared>>,
    generation:i64,value:Shared,work:&WorkScope,
)->Result<(),MixedJsonError>{
    let admission=NativeAdmission(Some(work));
    if !tree.contains_key(&generation) {
        tree.insert_admitted(generation,Default::default(),&admission).map_err(MixedJsonError::Resource)?;
    }
    let list=tree.get_mut(&generation).unwrap();
    list.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
    list.try_push(value).map_err(MixedJsonError::Resource)
}
impl AuditWalk {
    pub(crate) fn capture<'a>(
        conn:&Connection,raw:impl IntoIterator<Item=&'a Value>,work:&WorkScope,observer:&dyn LegacyObserver,
    )->Result<Self,MixedJsonError>{
        let (objects,withdrawn)=read_objects(conn,None,work,observer)?;
        let witnesses=Family::capture(conn,"witnesses",work,observer)?;
        let relations=Family::capture(conn,"relations",work,observer)?;
        let observations=Family::capture(conn,"observations",work,observer)?;
        let mut raw_copy=newchan_rust::s_content::CodecVec::default();
        for record in raw {
            let copied=PlainJson::try_clone_from(record,Some(work)).map_err(MixedJsonError::Resource)?;
            raw_copy.try_reserve_native(work,1).map_err(MixedJsonError::Resource)?;
            raw_copy.try_push(copied).map_err(MixedJsonError::Resource)?;
        }
        let mut result=Self{active:Default::default(),births:Default::default(),deaths:Default::default(),
            witnesses,relations,observations,original_observations:Default::default(),raw:raw_copy,objects:Default::default()};
        let mut ids=NativeSet::default();
        let admission=NativeAdmission(Some(work));
        for value in objects.iter().chain(withdrawn.iter()) {
            observer.visit_record("objects","capture-lifecycle",LegacyRecordRef::Json(value)).map_err(MixedJsonError::Resource)?;
            if !ids.insert_admitted(identity(value,"objects",work)?,&admission).map_err(MixedJsonError::Resource)? {
                return Err("StorageUnavailable：对象重复身份".into());
            }
            let copied=value.try_clone_mixed(work).map_err(MixedJsonError::Resource)?;
            let shared=NativeArc::try_new_shallow(copied,Some(work)).map_err(MixedJsonError::Resource)?;
            result.objects.insert_admitted(identity(value,"objects",work)?,shared.clone(),&admission).map_err(MixedJsonError::Resource)?;
            push_event(&mut result.births,number(value,"first_known_generation")?,shared.clone(),work)?;
            if !value["withdrawn_generation"].is_null() {
                push_event(&mut result.deaths,number(value,"withdrawn_generation")?,shared,work)?;
            }
        }
        Ok(result)
    }
    pub(crate) fn finish(&self)->Result<(),MixedJsonError>{
        if !self.births.is_empty() || !self.deaths.is_empty() {
            return Err("StorageUnavailable：对象生灭代不可达".into());
        }
        self.witnesses.finish()?;
        self.relations.finish()?;
        self.observations.finish()
    }
}
