//! #1394：Q 实际调用方显式持有的局部分配原型；不生成或替代 Q21/消费者 Job 资源。
use newchan_rust::s_content::capture_allocation::OwnedBudget;

#[derive(Clone)]
pub(crate) struct Profile {
    pub budget: OwnedBudget,
    pub overlay_slots: usize,
    pub overlay_bytes: usize,
}
impl Profile {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.overlay_slots as u128 > newchan_rust::s_content::MAX_FRAME_LEN as u128
            || self.overlay_bytes as u128 > newchan_rust::s_content::MAX_FRAME_LEN as u128
        {
            return Err("QAllocationInvalid：显式 overlay 范围超过规范数域".into());
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        return Err("prototype_audit_allocation_incomplete：此构建不支持实际分配原型".into());
        #[cfg(feature = "s_allocation_json_testonly")]
        Ok(())
    }
}
