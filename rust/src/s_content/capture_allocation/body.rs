//! #1394 封闭 captured verifier 全部节点读取的真实 requested Layout owner。
//! 仅 bytes 的实际分配计费；没有 Vec/clone/into_vec 所有权逃逸。
use super::*;

pub(crate) struct MeteredBody {
    owner: OwnedLayout,
}
impl MeteredBody {
    pub(super) fn copy_from(bytes: &[u8], budget: &OwnedBudget) -> Result<Self, CodecError> {
        let layout = Layout::array::<u8>(bytes.len()).map_err(|_| refused())?;
        // OwnedLayout 在实际 alloc_zeroed 前核额度并借记同一计数器。
        let owner = OwnedLayout::new(layout, budget)?;
        if !bytes.is_empty() {
            // SAFETY: sole owner of exactly bytes.len() initialized u8 slots; input cannot alias it.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), owner.pointer.as_ptr(), bytes.len());
            }
        }
        let m = &budget.0;
        m.copied_rows.set(m.copied_rows.get() + 1);
        m.copied_bytes
            .set(m.copied_bytes.get() + bytes.len() as u64);
        Ok(Self { owner })
    }
}
impl std::ops::Deref for MeteredBody {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        // SAFETY: OwnedLayout is alive, initialized, non-null even for zero length, and privately owned.
        unsafe { std::slice::from_raw_parts(self.owner.pointer.as_ptr(), self.owner.layout.size()) }
    }
}
