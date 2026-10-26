//! #1527：真实 VFS/旧域共用原 Job；回调失败不构造动态诊断或堆装箱。
use super::*;
use super::mixed_sql::{ExpectedTables, MixedPolicy};
use crate::v3::admission_anchor::terminal::{self, FixedFailure};
use crate::v3::audit_budget::{LegacyObserver, LegacyRecordRef};
use sc::capture_allocation::scratch::{ScratchFailure, WorkScope};
use std::cell::Cell;

struct FixedErrors {
    first: Option<FixedFailure>,
    first_codec: Option<sc::CodecError>,
    later: [Option<FixedFailure>; 7],
    unaccepted_wire: u128,
    overflow: bool,
}
impl FixedErrors {
    fn new() -> Self {
        Self { first:None, first_codec:None, later:[None;7], unaccepted_wire:0, overflow:false }
    }
    fn note(&mut self, phase:ErrorPhase, failure:FixedFailure) {
        if self.first.is_none() {self.first=Some(failure);}
        else if self.later[phase as usize].is_none() {self.later[phase as usize]=Some(failure);}
    }
}
pub(super) struct MixedAccounting<'s,'j> {
    account: RefCell<&'s mut AdmissionAccounting<'j>>,
    pub(super) policy: &'s Cell<MixedPolicy>,
    pub(super) work: &'j WorkScope,
    phase: Cell<ErrorPhase>,
    errors: RefCell<FixedErrors>,
}
impl<'s,'j> MixedAccounting<'s,'j> {
    pub(super) fn new(account:&'s mut AdmissionAccounting<'j>, policy:&'s Cell<MixedPolicy>) -> Option<Self> {
        let work=account.mixed_scope()?;
        account.is_mixed().then(||Self{
            account:RefCell::new(account),policy,work,
            phase:Cell::new(ErrorPhase::Business),errors:RefCell::new(FixedErrors::new()),
        })
    }
    pub(super) fn set_phase(&self,phase:ErrorPhase) {self.phase.set(phase);}
    fn observed<T>(&self,result:Result<T,sc::CodecError>)->Result<T,sc::CodecError>{
        if let Err(error)=&result {
            let mut errors=self.errors.borrow_mut();
            let phase=self.phase.get();
            errors.note(phase,FixedFailure::new(error.code(),phase.name(),error.detail()));
            if errors.first_codec.is_none() {errors.first_codec=Some(error.clone());}
        }
        result
    }
    fn run<T>(&self,run:impl FnOnce(&mut AdmissionAccounting<'_>)->Result<T,sc::CodecError>)->Result<T,sc::CodecError>{
        let result=self.account.try_borrow_mut()
            .map_err(|_|self.work.reject(ScratchFailure::Ledger))
            .and_then(|mut account|run(&mut account));
        self.observed(result)
    }
    pub(super) fn resource_error(&self)->Option<sc::CodecError>{
        self.errors.borrow().first_codec.clone()
    }
    pub(super) fn note_business(&self,code:&'static str,detail:&str){
        let phase=self.phase.get();
        self.errors.borrow_mut().note(phase,FixedFailure::new(code,phase.name(),detail));
    }
    pub(super) fn publish_failure(&self)->Option<FixedFailure>{
        let errors=self.errors.borrow();
        if let Some(first)=errors.first {terminal::note_failure(first);}
        terminal::note_cleanup(errors.later,errors.unaccepted_wire,errors.overflow);
        errors.first
    }
    fn after_read(&self,actual:u64)->Result<(),sc::CodecError>{
        let (result,accepted)=match self.account.try_borrow_mut(){
            Ok(mut account)=>{
                let before=account.snapshot().wire_bytes;
                let result=account.charge_wire_bytes(actual);
                let accepted=account.snapshot().wire_bytes-before;
                (result,accepted)
            }
            Err(_)=>(Err(self.work.reject(ScratchFailure::Ledger)),0),
        };
        let rejected=u128::from(actual).saturating_sub(accepted);
        let mut errors=self.errors.borrow_mut();
        match errors.unaccepted_wire.checked_add(rejected){
            Some(total)=>errors.unaccepted_wire=total,
            None=>{errors.unaccepted_wire=u128::MAX;errors.overflow=true;}
        }
        drop(errors);
        self.observed(result)
    }
}
impl LegacyObserver for MixedAccounting<'_,'_> {
    fn select_legacy_capture(&self,graph:bool,records_in_graph:bool){
        self.policy.set(MixedPolicy::Capture{graph,records_in_graph});
    }
    fn select_legacy_tables(&self,v2:bool,ohlc:bool,delta:bool,graph:bool){
        self.policy.set(MixedPolicy::Selected(ExpectedTables::selected(v2,ohlc,delta,graph)));
    }
    fn precheck(&self)->Result<(),sc::CodecError>{self.run(|a|a.pre_io_check())}
    fn visit_record(&self,_:&'static str,_:&'static str,_:LegacyRecordRef<'_>)->Result<(),sc::CodecError>{
        self.run(|a|a.charge_records(1))
    }
    fn check_work(&self,work:&WorkScope)->Result<(),sc::CodecError>{
        let binding=work.reserve_native(0)?;
        if !binding.belongs_to_parent(self.work){return self.observed(Err(work.reject(ScratchFailure::Ledger)));}
        Ok(())
    }
    fn accept_checked(&self,row:&sc::legacy_evidence::CheckedLegacyEvidenceRow)->Result<(),sc::CodecError>{
        self.run(|a|a.accept_legacy_evidence(row).map(|_|()))
    }
}
#[derive(Debug)]
struct CallbackFailed;
impl std::fmt::Display for CallbackFailed {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("MixedCallbackFailed")}
}
impl std::error::Error for CallbackFailed {}
// Box<ZST> does not allocate; the original typed cause is held in FixedErrors.
fn callback_error(_:sc::CodecError)->Box<dyn std::error::Error>{Box::new(CallbackFailed)}
impl IoAccounting for &MixedAccounting<'_,'_> {
    fn before_io(&mut self)->Result<(),Box<dyn std::error::Error>>{
        LegacyObserver::precheck(*self).map_err(callback_error)
    }
    fn after_read(&mut self,n:u64)->Result<(),Box<dyn std::error::Error>>{
        MixedAccounting::after_read(*self,n).map_err(callback_error)
    }
}

pub(super) fn fixed_metadata_bytes()->usize {
    std::mem::size_of::<MixedAccounting<'static,'static>>()
}
