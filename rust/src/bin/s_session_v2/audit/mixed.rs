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
            let record=NativeArc::try_new(record,Some(work)).map_err(MixedJsonError::Resource)?;
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
