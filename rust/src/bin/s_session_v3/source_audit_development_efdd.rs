// #1419: frozen source from efddc74e182c26fb2ca414823940961e38889ad8.
// The older source_audit_oracle has count-before-root ordering; do not change it.
use super::*;

fn audit_development_unit(
    s: &mut (impl CodecRead + ?Sized),
    unit: &RefValue,
    raw: &(impl RawLookup + ?Sized),
    source_bound: bool,
) -> Result<(), PublishError> {
    exact(unit, Kind::Object, if source_bound { 3 } else { 2 })?;
    let bar = field(s, unit, "bar")?;
    exact(&bar, Kind::Object, 5)?;
    let coord = decimal(&text(s, &bar, "source_coord")?)?;
    let expected = raw.get(&coord).ok_or_else(|| err("CC007单元缺原始来源"))?;
    if digest(&bar)? != expected.bar {
        return Err(err("CC007单元bar不是同cut五字段原件"));
    }
    let relations = field(s, unit, "relations")?;
    exact(&relations, Kind::Array, 24)?;
    if source_bound {
        let source = field(s, unit, "source_ref")?;
        exact(&source, Kind::Object, 8)?;
        if digest(&source)? != expected.wire {
            return Err(err("CC007单元source_ref不是同cut完整八字段原件"));
        }
    }
    Ok(())
}

pub(super) fn audit_development_batch(
    s: &mut (impl CodecRead + ?Sized),
    payload: &RefValue,
    raw: &(impl RawLookup + ?Sized),
    rule: &str,
) -> Result<(), PublishError> {
    let source_bound = crate::v3::layout::Layout::of_rule(rule)? == crate::v3::layout::Layout::Bounded2;
    if !source_bound {
        return Ok(());
    }
    exact(payload, Kind::Object, 2)?;
    let bar = field(s, payload, "bar")?;
    let entries = field(s, payload, "entries")?;
    for entry in elements(s, &entries)? {
        exact(&entry, Kind::Object, 3)?;
        let _fractal = text(s, &entry, "fractal_object_id")?;
        let unit = field(s, &entry, "unit")?;
        audit_development_unit(s, &unit, raw, true)?;
        if field(s, &unit, "bar")? != bar {
            return Err(err("CC007单元bar与批记录不同"));
        }
        let development = field(s, &entry, "development")?;
        exact(&development, Kind::Object, 2)?;
        let count = decimal(&text(s, &development, "count")?)?;
        let prefix = field(s, &development, "root")?;
        if count <= 0 {
            return Err(err("CC007发展前缀为空"));
        }
        exact(&prefix, Kind::Array, count as u64)?;
        let units = elements(s, &prefix)?;
        for previous in &units {
            audit_development_unit(s, previous, raw, true)?;
        }
        if units.last() != Some(&unit) {
            return Err(err("CC007发展前缀末项不是本单元"));
        }
    }
    Ok(())
}

