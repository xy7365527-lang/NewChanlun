/* #1394：Q3完整消费者；只供既有tb01c页面调用，没有产品路由或默认资源。 */
(function (root, factory) {
  const common = typeof module === "object" && module.exports;
  const build = factory(
    common ? require("./q3-values.js") : root.Q3Values,
    common ? require("./q3-typed.js") : root.Q3Typed,
    common ? require("./s-content-job-v3.js") : root.SContentJobV3,
    common ? require("./s-content-v3.js") : root.SContentV3,
    common ? require("./q3-byte-plan.js") : root.Q3BytePlan,
  );
  if (common) module.exports = build;
  else root.Q3Consumer = build;
})(globalThis, function (V, typed, jobs, content, bytePlan) {
  "use strict";
  const {
    of,
    field,
    need,
    keys,
    T,
    S,
    I,
    N,
    equal,
    sha,
    identity,
    number,
    cmp,
  } = V;
  const f = field,
    t = async (v, k) => T(await f(v, k)),
    s = async (v, k) => S(await f(v, k)),
    i = async (v, k, p = false, fr = false) => I(await f(v, k), p, fr),
    n = async (v, k) => N(await f(v, k));
  const FAMILIES = [
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
  ];
  const FIXED = [
    "session_id",
    "session_generation",
    "structure_cut",
    "cut_generation",
    "catalog_revision",
    "index_frontier",
    "profile_id",
    "profile_hash",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "catalog_evidence",
    "scope",
    "rule_revision",
    "history_mode",
    "as_of_generation",
  ];
  const HEADER = [
    "schema_revision",
    "session_id",
    "session_generation",
    "source_namespace",
    "source_epoch",
    "message_id",
    "producer_id",
    "producer_epoch",
    "payload_hash",
    "causal_refs",
    "payload",
  ];
  const PAGE = [
    "fixed_cut",
    "catalog",
    "rows",
    "counts",
    "scope",
    "omissions",
    "order_version",
    "offset",
    "done",
    "next_token",
    "cut_projection_digest",
    "validated_capture_digest",
    "ok",
  ];
  const TOKEN = [
    "token_schema",
    "session_id",
    "session_generation",
    "cut_generation",
    "scope",
    "history_mode",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "profile_hash",
    "rule_revision",
    "order_version",
    "page_size",
    "offset",
    "cut_projection_digest",
    "issued_capture_digest",
    "token_checksum",
  ];
  const CURSOR = [
    "session_id",
    "session_generation",
    "catalog_revision",
    "scope",
    "after_generation",
    "base_cut",
    "index_frontier",
    "last_seq",
    "order_version",
  ];
  const D = [
    "generation",
    "session_id",
    "catalog_revision",
    "base_cut",
    "next_cut",
    "seq_range",
    "index_frontier",
    "input_frontier",
    "catalog_run_status",
    "catalog_evidence",
  ];
  const CHANGES = [
    "upserts",
    "withdrawals",
    "replaces",
    "witnesses",
    "relations",
    "observations",
    "raw_history_added",
  ];
  const SCOPE = { structure: "CompleteCut", economic: "not_started" };
  const RAWREF = [
    "identity_key",
    "payload_hash",
    "receipt_id",
    "event_id",
    "input_revision",
    "revision",
    "seq",
    "source_coord",
  ];
  const RAW = [
    "identity_key",
    "revision",
    "input_revision",
    "payload_hash",
    "receipt_id",
    "seq",
    "source_namespace",
    "source_epoch",
    "instrument",
    "event_id",
    "received_at",
    "raw_text",
    "ts",
    "volume",
    "source_coord",
    "supersedes_revision",
    "schema_revision",
    "open",
    "high",
    "low",
    "close",
  ];
  const digest = (x) =>
    need(typeof x === "string" && /^[0-9a-f]{64}$/.test(x), "SHA-256格式错误");
  const objectId = (x) =>
    need(/^obj3-[0-9a-f]{64}$/.test(x), "新域对象身份格式错误");
  const batch = (x) =>
    need(/^batch3-[0-9a-f]{64}$/.test(x), "新域batch身份格式错误");
  function smallIdentity(v) {
    return {
      session_id: v.session_id,
      session_generation: v.session_generation,
    };
  }
  function sameIdentity(a, b) {
    return (
      a.session_id === b.session_id &&
      a.session_generation === b.session_generation
    );
  }
  async function select(v, names) {
    const p = {};
    for (const k of names) p[k] = await f(v, k);
    return of(p);
  }
  async function scalars(v, names) {
    const p = {};
    for (const k of names) p[k] = await (await f(v, k)).scalar();
    return p;
  }
  async function texts(v) {
    for await (const x of of(v).items()) await T(x);
  }
  async function coords(v) {
    let prior = -1n;
    for await (const x of of(v).items()) {
      const c = await I(x);
      need(c > prior, "来源坐标重复/未严格递增");
      prior = c;
    }
  }
  async function rawReference(raw) {
    return select(raw, RAWREF);
  }
  async function rawOriginal(raw) {
    await rawValidate(raw); // 只逆投影已证明恰为生产wire的完整原件，不能吞掉额外price/字段。
    const out = {};
    for await (const e of of(raw).entries()) {
      if (["revision", "seq", "supersedes_revision"].includes(e.key))
        out[e.key] =
          (await e.value.scalar()) === null
            ? null
            : number((await I(e.value, e.key === "revision")).toString());
      else out[e.key] = e.value;
    }
    out.price = await f(raw, "close");
    return of(out);
  }
  async function rawValidate(raw) {
    await keys(raw, RAW, "OHLC原始历史");
    need((await s(raw, "schema_revision")) === "s-ohlc/1", "OHLC schema错误");
    const low = await n(raw, "low"),
      high = await n(raw, "high");
    for (const k of ["open", "close"]) {
      const p = await n(raw, k);
      need(low <= p && p <= high, "原始OHLC几何不符");
    }
    await n(raw, "ts");
    need((await n(raw, "volume")) >= 0n, "volume为负");
    for (const k of [
      "source_namespace",
      "source_epoch",
      "instrument",
      "event_id",
      "received_at",
    ])
      await t(raw, k);
    await completeValue(await f(raw, "raw_text"));
  }
  async function auditRaw(snapshot, charge = () => {}) {
    const byId = new Map(),
      effective = new Map(),
      owners = new Map(),
      positions = new Map(),
      revisions = new Map();
    let seq = 0n;
    for await (const raw of (await f(snapshot, "raw_history")).items()) {
      charge();
      await rawValidate(raw);
      const identityKey = await t(raw, "identity_key"),
        rev = await i(raw, "revision", true),
        coord = await s(raw, "source_coord");
      await I(coord);
      need(
        (await s(raw, "input_revision")) === rev.toString() &&
          (await i(raw, "seq")) === seq++,
        "raw接纳序/input修订错误",
      );
      const expected =
        (await t(raw, "source_namespace")) +
        "|" +
        (await t(raw, "source_epoch")) +
        "|" +
        (await t(raw, "instrument")) +
        "|" +
        (await t(raw, "event_id"));
      need(identityKey === expected, "raw namespace/业务身份不符");
      need(
        !owners.has(coord) || owners.get(coord) === identityKey,
        "同源坐标多owner",
      );
      need(
        !positions.has(identityKey) || positions.get(identityKey) === coord,
        "同业务身份多坐标",
      );
      owners.set(coord, identityKey);
      positions.set(identityKey, coord);
      const old = revisions.get(identityKey),
        sup = await (await f(raw, "supersedes_revision")).scalar();
      need(
        (old === undefined ? sup === null : sup === old.toString()) &&
          (old === undefined || rev > old),
        "raw修订链断裂",
      );
      revisions.set(identityKey, rev);
      const payload = await select(raw, [
        "event_id",
        "received_at",
        "raw_text",
        "open",
        "high",
        "low",
        "close",
        "volume",
      ]);
      const p = {};
      for await (const e of payload.entries()) p[e.key] = e.value;
      p.revision = await f(raw, "input_revision");
      p.seq = await f(raw, "source_coord");
      p.timestamp = await f(raw, "ts");
      const hash = await sha(p);
      need((await s(raw, "payload_hash")) === hash, "原始payload_hash不符");
      need(
        (await s(raw, "receipt_id")) ===
          "rcpt-" + V.rawSha(identityKey + "|" + hash).slice(0, 16),
        "原始receipt不符",
      );
      const key = JSON.stringify([identityKey, rev.toString()]);
      need(!byId.has(key), "raw历史身份重复");
      byId.set(key, raw);
      effective.set(coord, raw);
    }
    return { byId, effective };
  }
  async function shapeRecord(record) {
    await keys(
      record,
      [
        "object_id",
        "object_revision",
        "kind",
        "batch_id",
        "branch",
        "dir_ab",
        "dir_bc",
        "window_start",
        "window_mid",
        "window_end",
        "comparisons",
        "input_refs",
        "source_coords",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
        "withdrawn_generation",
        "withdrawal_reason",
        "superseded_by",
        "lifecycle",
      ],
      "CC006完整记录",
    );
    await typed.validateTB02ShapeRefs(record);
    need((await s(record, "object_revision")) === "1", "CC006修订错误");
    const first = await i(record, "first_known_generation", true);
    need(
      (await s(record, "first_known_cut")) === "cut-" + first &&
        (await i(record, "published_generation", true)) === first,
      "CC006首次获知不闭合",
    );
    const withdrawn = await (await f(record, "withdrawn_generation")).scalar(),
      reason = await (await f(record, "withdrawal_reason")).scalar(),
      sup = await (await f(record, "superseded_by")).scalar();
    if (withdrawn === null)
      need(
        (await s(record, "lifecycle")) === "active" &&
          reason === null &&
          sup === null,
        "活动CC006携撤回字段",
      );
    else
      need(
        BigInt(withdrawn) > first &&
          (await s(record, "lifecycle")) === "withdrawn" &&
          ["fact_removed", "superseded_by_revision"].includes(reason) &&
          (reason === "fact_removed") === (sup === null),
        "CC006撤回不闭合",
      );
    need(
      ["TOP", "BOTTOM", "RISING", "FALLING"].includes(
        await s(record, "branch"),
      ) &&
        ["UP", "DOWN"].includes(await s(record, "dir_ab")) &&
        ["UP", "DOWN"].includes(await s(record, "dir_bc")),
      "CC006分类枚举不符",
    );
    const comparisons = await f(record, "comparisons");
    need((await comparisons.length()) === 4n, "CC006比较四条件缺失");
    let index = 0;
    for await (const c of comparisons.items()) {
      await keys(
        c,
        ["axis", "cur", "held", "pair", "prev", "strict_up"],
        "CC006比较",
      );
      const axis = ["high", "low"][index % 2],
        pair = ["ab", "bc"][Math.floor(index / 2)];
      need(
        (await s(c, "axis")) === axis && (await s(c, "pair")) === pair,
        "CC006比较轴/顺序不符",
      );
      await n(c, "cur");
      await n(c, "prev");
      need(
        (await f(c, "held")).kind === "bool" &&
          (await f(c, "strict_up")).kind === "bool",
        "CC006比较bool不符",
      );
      index++;
    }
  }
  // #1491：`layout` 由调用方从本 cut 的 rule_revision 布局组件取得（ADR 0037 §六，按版本分派）。
  // 未给出 ⟹ 调用方读的是 v1/v2 快照（ADR 0037 之前的写者，按定义 layout/1）。
  async function bindSources(snapshot, charge = () => {}, layout = typed.LAYOUTS.LAYOUT1) {
    const raw = await auditRaw(snapshot, charge),
      groups = new Map(),
      shapes = new Map(),
      records = [];
    for (const family of ["objects", "withdrawn_objects"])
      for await (const record of (await f(snapshot, family)).items()) {
        charge();
        records.push(record);
        const kind = await t(record, "kind");
        if (typed.TB02_KINDS.includes(kind)) await typed.validateTyped(record, layout);
        else await shapeRecord(record);
        const refs = await f(record, "input_refs"),
          active = (await s(record, "lifecycle")) === "active";
        async function checkRef(ref) {
          await keys(ref, RAWREF, "来源八键");
          const r = raw.byId.get(
            JSON.stringify([
              await t(ref, "identity_key"),
              await s(ref, "revision"),
            ]),
          );
          need(
            r && (await equal(ref, await rawReference(r))),
            "对象八键来源未绑定独立完整原始历史",
          );
          if (active)
            need(
              await equal(
                ref,
                await rawReference(
                  raw.effective.get(await s(ref, "source_coord")) || {},
                ),
              ),
              "活动对象使用旧修订",
            );
        }
        if (typed.TB02_KINDS.includes(kind)) {
          for await (const ref of refs.items()) await checkRef(ref);
        } else
          for await (const group of refs.items())
            for (const name of ["raw_refs", "dependency_refs"])
              for await (const ref of (await f(group, name)).items())
                await checkRef(ref);
        if (kind === "CC-005.inclusion_group") {
          const anchor = await s(await f(record, "payload"), "group_anchor");
          let versions = groups.get(anchor);
          if (!versions) groups.set(anchor, (versions = []));
          versions.push(record);
        }
        if (kind === "CC-006.local_shape")
          shapes.set(await t(record, "object_id"), record);
      }
    const partition = new Set(),
      currentGroups = [];
    for (const versions of groups.values())
      for (const record of versions) {
        if ((await s(record, "lifecycle")) === "active")
          currentGroups.push(await f(record, "payload"));
      }
    const orderedGroups = [];
    for (const payload of currentGroups)
      orderedGroups.push({ index: await i(payload, "group_index"), payload });
    orderedGroups.sort((a, b) =>
      a.index < b.index ? -1 : a.index > b.index ? 1 : 0,
    );
    let expectedIndex = 0n;
    for (const group of orderedGroups) {
      need(group.index === expectedIndex++, "同cut组合并序号不连续");
      for await (const c of (await f(group.payload, "members")).items()) {
        const coord = await S(c);
        need(!partition.has(coord), "同cut组成员重叠");
        partition.add(coord);
      }
    }
    need(
      partition.size === raw.effective.size &&
        [...partition].every((c) => raw.effective.has(c)),
      "同cut组未完整覆盖生效原始历史",
    );
    const bound = new Map();
    for (const [id, shape] of shapes) {
      const cut =
        (await s(shape, "lifecycle")) === "active"
          ? await i(snapshot, "generation")
          : await i(shape, "first_known_generation");
      const anchors = [
          await s(shape, "window_start"),
          await s(shape, "window_mid"),
          await s(shape, "window_end"),
        ],
        selected = [];
      let index = 0;
      for await (const ref of (await f(shape, "input_refs")).items()) {
        let chosen = null;
        for (const g of groups.get(anchors[index]) || []) {
          const first = await i(g, "first_known_generation"),
            withdraw = await (await f(g, "withdrawn_generation")).scalar();
          if (first <= cut && (withdraw === null || BigInt(withdraw) > cut)) {
            need(chosen === null, "同cut组版本不唯一");
            chosen = g;
          }
        }
        need(chosen, "对应cut缺唯一完整组");
        const payload = await f(chosen, "payload");
        need(
          await equal(
            await f(ref, "merged_index"),
            await f(payload, "group_index"),
          ),
          "形态/组序号不同",
        );
        const members = [];
        for await (const r of (await f(ref, "raw_refs")).items())
          members.push(await s(r, "source_coord"));
        need(
          await equal(members, await f(payload, "members")),
          "形态实际成员与组不同",
        );
        const deps = new Set();
        for (const role of ["direction_evidence", "confirmation_evidence"]) {
          const evidence = await f(payload, role);
          if (evidence.kind !== "null")
            for await (const c of (await f(evidence, "source_coords")).items())
              deps.add(await S(c));
        }
        for (const c of members) deps.delete(c);
        const depList = [];
        for await (const r of (await f(ref, "dependency_refs")).items())
          depList.push(await s(r, "source_coord"));
        need(
          await equal(
            depList,
            [...deps].sort((a, b) => (BigInt(a) < BigInt(b) ? -1 : 1)),
          ),
          "形态依赖与组事实不同",
        );
        const groupRefs = new Map();
        for await (const r of (await f(chosen, "input_refs")).items())
          groupRefs.set(await s(r, "source_coord"), r);
        for (const role of ["raw_refs", "dependency_refs"])
          for await (const r of (await f(ref, role)).items())
            need(
              await equal(r, groupRefs.get(await s(r, "source_coord")) || null),
              "形态与组修订不同",
            );
        selected.push(payload);
        index++;
      }
      need(selected.length === 3, "形态组缺失");
      bound.set(id, selected);
    }
    // #1491 bounded-layout/1：CC-007 三跳只读绑定（分型对象 → 批记录 entry → 单元序列）。
    await typed.bindDevelopmentBatches(records, layout);
    return { raw, records, shapes, bound };
  }
  async function shapeIdentity(
    record,
    groupFacts,
    profile,
    rule,
    numeric = true,
  ) {
    const p = {};
    for (const k of [
      "branch",
      "dir_ab",
      "dir_bc",
      "window_start",
      "window_mid",
      "window_end",
      "comparisons",
    ])
      p[k] = await f(record, k);
    p.merged_highs = [];
    p.merged_lows = [];
    for (const g of groupFacts) {
      p.merged_highs.push(await f(g, "high"));
      p.merged_lows.push(await f(g, "low"));
    }
    p.profile_id = profile;
    p.rule_revision = rule;
    p.input_refs = [];
    for await (const g of (await f(record, "input_refs")).items()) {
      const out = {
        merged_index: number((await i(g, "merged_index")).toString()),
        raw_refs: [],
        dependency_refs: [],
      };
      for (const k of ["raw_refs", "dependency_refs"])
        for await (const r of (await f(g, k)).items()) {
          const refs = {};
          for await (const e of r.entries())
            refs[e.key] = ["seq", "revision"].includes(e.key)
              ? number((await I(e.value, e.key === "revision")).toString())
              : e.value;
          out[k].push(of(refs));
        }
      p.input_refs.push(of(out));
    }
    return of(p);
  }
  async function auditWitnesses(snapshot, binding, charge = () => {}) {
    const seen = new Set();
    for await (const w of (await f(snapshot, "witnesses")).items()) {
      charge();
      await keys(
        w,
        [
          "witness_id",
          "object_id",
          "slot",
          "merged_source_index",
          "merged_high",
          "merged_low",
          "merged_open",
          "merged_close",
          "raw_bars",
        ],
        "witness",
      );
      const id = await t(w, "object_id"),
        slot = await i(w, "slot");
      need(slot < 3n, "witness槽越界");
      const key = JSON.stringify([id, slot.toString()]);
      need(!seen.has(key), "witness槽重复");
      seen.add(key);
      const facts = binding.bound.get(id),
        shape = binding.shapes.get(id);
      need(facts && shape, "witness缺同cut形态");
      const group = facts[Number(slot)],
        mi = await i(group, "group_index");
      need(
        await equal(
          await f(w, "merged_source_index"),
          await f(group, "group_anchor"),
        ),
        "witness锚与组不同",
      );
      for (const axis of ["open", "high", "low", "close"])
        need(
          await equal(await f(w, "merged_" + axis), await f(group, axis)),
          "witness merged_" + axis + "不是同cut组事实",
        );
      const members = await f(group, "members"),
        bars = await f(w, "raw_bars");
      need(
        (await bars.length()) === (await members.length()),
        "witness raw成员数不同",
      );
      const originalBars = [];
      let index = 0n;
      for await (const bar of bars.items()) {
        const coord = await S(await members.at(index++)),
          sourceRef = await (
            await f(await (await f(shape, "input_refs")).at(slot), "raw_refs")
          ).at(index - 1n),
          original = binding.raw.byId.get(
            JSON.stringify([
              await t(sourceRef, "identity_key"),
              await s(sourceRef, "revision"),
            ]),
          );
        need(original, "witness raw缺独立历史原件");
        charge(); // 独立raw_history原件的完整复核，不计普通raw_bars数组元素。
        const expected = await rawOriginal(original),
          actual = await rawOriginal(bar);
        need(await equal(actual, expected), "witness raw原件/次序不同");
        originalBars.push(actual);
      }
      const witnessId =
        "wit-" +
        (
          await sha({
            mi: number(mi.toString()),
            object_id: id,
            raw_bars: originalBars,
            slot: number(slot.toString()),
          })
        ).slice(0, 16);
      need((await t(w, "witness_id")) === witnessId, "witness身份不符");
    }
    need(seen.size === binding.shapes.size * 3, "形态缺完整三槽见证");
  }
  // #1404/#1414：逐cut轴集合随规则修订扩展（同 main validateTB02Axes）；规则→轴表见 RULE_AXES。
  const LEGACY_TB02_AXES = [
      "CC-001",
      "CC-002",
      "CC-003",
      "CC-004",
      "CC-005",
      "CC-006",
      "CC-007",
      "CC-054",
      "CC-056",
    ],
    BI_AXES = [...LEGACY_TB02_AXES, "CC-008", "CC-009", "CC-010", "CC-055"],
    FIRST_SEGMENT_AXES = [...BI_AXES, "CC-011", "CC-012", "CC-013"],
    TB02_AXES = [...FIRST_SEGMENT_AXES, "CC-014"];
  const RULE_AXES = new Map([
    ["s2-axis-quantifiers", LEGACY_TB02_AXES],
    ["s2-axis-quantifiers+new-bi/1", BI_AXES],
    ["s2-axis-quantifiers+new-bi/2", BI_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-first/1", FIRST_SEGMENT_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-second/1", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/2+segment-second/1", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/2+segment-second/2", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1", TB02_AXES], ["s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2", TB02_AXES],
  ]);
  async function axesValidate(axes, active = null) {
    axes = of(axes);
    const second = await axes.has("CC-014");
    await keys(
      axes,
      second
        ? TB02_AXES
        : (await axes.has("CC-011"))
          ? FIRST_SEGMENT_AXES
          : (await axes.has("CC-008"))
            ? BI_AXES
            : LEGACY_TB02_AXES,
      "逐cut目录轴",
    );
    let ids = null;
    const byId = new Map();
    if (active) {
      ids = new Set();
      for await (const r of of(active).items()) {
        const id = await t(r, "object_id");
        ids.add(id);
        byId.set(id, r);
      }
    }
    for await (const { key: cid, value: axis } of axes.entries()) {
      await keys(
        axis,
        ["impl_status", "proof_status", "run_status", "evidence"],
        "目录轴",
      );
      need(
        ["implemented", "not_implemented"].includes(
          await s(axis, "impl_status"),
        ) &&
          (await s(axis, "proof_status")) === "not_proved" &&
          ["run", "waiting", "not_run"].includes(await s(axis, "run_status")),
        "目录轴状态错误",
      );
      const e = await f(axis, "evidence");
      const scoped =
        e.kind === "object" &&
        (await e.has("scope")) &&
        (await equal(await f(e, "scope"), "TB-02-D"));
      await keys(
        e,
        [
          ...(cid === "CC-056"
            ? [
                "scope",
                "object_ids",
                "waiting_reasons",
                "raw_revisions",
                "withdrawals",
                "replaces",
              ]
            : ["scope", "object_ids", "waiting_reasons"]),
          ...(scoped
            ? ["delivered_subobligations", "deferred_subobligations"]
            : []),
        ],
        "目录轴证据",
      );
      if (scoped) {
        need(
          (await equal(await f(e, "delivered_subobligations"), [
            "finite_complete_prefix_strict_second",
            "independent_stroke_and_segment_break_facts",
          ])) &&
            (await equal(await f(e, "deferred_subobligations"), [
              "general_equal_identity",
              "general_scan_continuation",
              "global_G006_bridge",
            ])),
          "第二种子义务范围不符",
        );
        need(
          !["CC-012", "CC-013", "CC-014"].includes(cid) ||
            (await s(axis, "impl_status")) === "not_implemented",
          "有限交付冒称全轴完成",
        );
      }
      need(
        (await s(e, "scope")) ===
          (second && ["CC-011", "CC-012", "CC-013", "CC-014"].includes(cid)
            ? "TB-02-D"
            : ["CC-011", "CC-012", "CC-013"].includes(cid)
              ? "TB-02-C"
              : "TB-02-A"),
        "轴范围错误",
      );
      await texts(await f(e, "waiting_reasons"));
      const seen = new Set();
      for await (const idV of (await f(e, "object_ids")).items()) {
        const id = await T(idV);
        need(!seen.has(id) && (!ids || ids.has(id)), "目录引用缺失/重复");
        seen.add(id);
      }
      if (cid === "CC-056") {
        let any = false;
        for (const name of ["raw_revisions", "withdrawals", "replaces"]) {
          const a = await f(e, name);
          need(a.kind === "array", "修订轴集合不完整");
          if ((await a.length()) > 0n) any = true;
          for await (const x of a.items()) await completeValue(x);
        }
        need(
          (await s(axis, "run_status")) === (any ? "run" : "not_run"),
          "修订轴状态不同",
        );
      }
    }
    if (second) {
      const objectIds = async (cid) =>
        f(await f(await f(axes, cid), "evidence"), "object_ids");
      need(
        await equal(await objectIds("CC-014"), await objectIds("CC-013")),
        "CC014未绑定CC013对象",
      );
      if (active)
        for await (const idV of (await objectIds("CC-014")).items()) {
          const r = byId.get(await T(idV));
          need(
            r && (await s(r, "kind")) === "CC-013.segment",
            "CC014引用非段对象",
          );
        }
    }
  }
  async function completeValue(v) {
    v = of(v);
    if (v.reader) {
      await v.reader.complete(v);
      return;
    }
    if (v.kind === "array")
      for await (const x of v.items()) await completeValue(x);
    else if (v.kind === "object")
      for await (const e of v.entries()) await completeValue(e.value);
    else if (v.reader) await v.reader.load(v);
    else await v.scalar();
  }
  async function integerWalk(v) {
    v = of(v);
    const names = new Set([
      "revision",
      "seq",
      "object_revision",
      "first_known_generation",
      "published_generation",
      "withdrawn_generation",
      "supersedes_revision",
      "slot",
      "merged_source_index",
      "merged_index",
      "window_start",
      "window_mid",
      "window_end",
    ]);
    if (v.kind === "array") {
      for await (const x of v.items()) await integerWalk(x);
    } else if (v.kind === "object") {
      // #1392/#1404：新笔/段 payload.slot 是逻辑身份；见证 slot 仍按整数核对。
      const logicalSlot =
        (await v.has("schema_revision")) &&
        ["s-new-bi/1", "s-new-bi/2", "s-segment/1", "s-segment/2", "s-segment/3", "s-segment/4", "s-segment/5"].includes(
          await s(v, "schema_revision"),
        );
      for await (const e of v.entries()) {
        need(
          e.value.kind !== "json_text" ||
            ["payload_json", "input_refs_json", "source_coords_json"].includes(
              e.key,
            ),
          "未声明的json_text位置",
        );
        if (
          names.has(e.key) &&
          e.value.kind !== "null" &&
          !(e.key === "slot" && logicalSlot)
        )
          await I(e.value);
        await integerWalk(e.value);
      }
    } else if (v.kind === "json_text")
      await integerWalk(await v.reader.inner(v));
    else if (v.reader) await v.reader.load(v);
    else await v.scalar();
  }
  async function rowKey(family, r) {
    const names =
      family === "objects" || family === "withdrawn_objects"
        ? ["object_id"]
        : family === "relations"
          ? ["subject", "relation_type", "object"]
          : family === "raw_history"
            ? ["identity_key", "revision"]
            : [family === "witnesses" ? "witness_id" : "observation_id"];
    const out = [];
    for (const k of names) out.push(await t(r, k));
    return out;
  }
  async function tuple(family, r) {
    if (family === "objects")
      return (await r.has("fact_key"))
        ? [
            1n,
            await t(r, "kind"),
            await t(r, "fact_key"),
            await t(r, "object_id"),
            await i(r, "object_revision"),
          ]
        : [
            0n,
            await i(r, "window_start"),
            await i(r, "window_mid"),
            await i(r, "window_end"),
            await t(r, "object_id"),
            await i(r, "object_revision"),
          ];
    if (family === "withdrawn_objects")
      return [
        await i(r, "withdrawn_generation"),
        await t(r, "object_id"),
        await i(r, "object_revision"),
      ];
    if (family === "witnesses")
      return [
        await t(r, "object_id"),
        await i(r, "slot"),
        await t(r, "witness_id"),
      ];
    if (family === "relations")
      return [
        await t(r, "subject"),
        await t(r, "relation_type"),
        await t(r, "object"),
      ];
    if (family === "observations")
      return [
        await t(r, "kind"),
        (await f(r, "window_start")).kind === "null"
          ? -1n
          : await i(r, "window_start"),
        await t(r, "observation_id"),
      ];
    return [
      await i(r, "seq"),
      await t(r, "identity_key"),
      await i(r, "revision"),
    ];
  }
  function compareTuples(a, b) {
    for (let j = 0; j < a.length; j++) {
      const c =
        typeof a[j] === "bigint"
          ? a[j] < b[j]
            ? -1
            : a[j] > b[j]
              ? 1
              : 0
          : cmp(a[j], b[j]);
      if (c) return c;
    }
    return 0;
  }
  function chargeActualIo(job, requests, wireBytes) {
    let firstError = null;
    try {
      if (requests > 0n) job.chargeRequests(requests);
    } catch (error) {
      firstError = error;
    }
    try {
      if (wireBytes > 0n) job.chargeWireBytes(wireBytes);
    } catch (error) {
      firstError ||= error;
    }
    if (firstError) throw firstError;
  }
  function operationLedger(core, operation) {
    const actual = { wire_bytes: 0n, records: 0n, requests: 0n };
    let firstStop = null;
    const ledger = {};
    const remember = (e) => {
      firstStop ||= { code: e.code || "query_validation", reason: String(e) };
    };
    for (const method of [
      "beginFirstRead",
      "firstReadDeadline",
      "checkDeadline",
      "preIoCheck",
      "reserveStaged",
      "reserveStore",
      "reconcile",
      "release",
      "reclaimStaged",
      "finishCheck",
      "cancel",
    ]) {
      ledger[method] = (...args) => {
        try {
          return core[method](...args);
        } catch (e) {
          remember(e);
          throw e;
        }
      };
    }
    ledger.acceptNode = async (node) => {
      try {
        return await core.acceptNode(node);
      } catch (e) {
        remember(e);
        throw e;
      }
    };
    for (const [method, key] of [
      ["chargeWireBytes", "wire_bytes"],
      ["chargeRecords", "records"],
      ["chargeRequests", "requests"],
    ])
      ledger[method] = (amount) => {
        need(
          typeof amount === "bigint" && amount >= 0n,
          "实际量必须为规范非负BigInt",
        );
        // actual已经发生；pure Job闭账也不能把网络尾量或已经访问的记录从本operation账里删掉。
        actual[key] += amount;
        try {
          return core[method](amount);
        } catch (e) {
          remember(e);
          throw e;
        }
      };
    ledger.snapshot = () => {
      let snapshot;
      try {
        snapshot = core.snapshot();
      } catch (e) {
        remember(e);
        return Object.freeze({
          ...Object.fromEntries(
            Object.entries(actual).map(([k, v]) => [k, v.toString()]),
          ),
          stopped: e.code || firstStop.code,
          first_failure: firstStop,
          core_snapshot_error: String(e),
          core_metrics_verified: false,
          accounting_operation: String(operation),
          actual_total_fields: ["wire_bytes", "records", "requests"],
          pure_metrics:
            "unknown: monotonic clock failed; no physical amount substituted",
        });
      }
      return Object.freeze({
        ...snapshot,
        accounting_operation: String(operation),
        actual_total_fields: ["wire_bytes", "records", "requests"],
        pure_snapshot_fields: [
          "node_bytes",
          "nodes",
          "staged",
          "store",
          "open_reservations",
          "elapsed_ms",
          "stopped",
        ],
        actual_tail: Object.freeze(
          Object.fromEntries(
            Object.entries(actual).map(([key, value]) => [
              key,
              (value - BigInt(snapshot[key])).toString(),
            ]),
          ),
        ),
        ...Object.fromEntries(
          Object.entries(actual).map(([k, v]) => [k, v.toString()]),
        ),
      });
    };
    return Object.freeze(ledger);
  }
  return function build(api) {
    // snapshot=null：只核目录自身结构并返回静态列（main validateTB02Catalog 的面）；轴绑定与静态SHA要完整同cut态。
    async function catalogValidate(catalog, snapshot = null) {
      const expected = api.TB02_CATALOG;
      need(
        (await s(catalog, "catalog_revision")) === expected.catalog_revision,
        "未绑定冻结目录版本",
      );
      const items = await f(catalog, "items");
      need(
        (await items.length()) === BigInt(expected.ids.length),
        "目录完整数量不符",
      );
      const staticFields = {},
        seen = new Set(),
        counts = { implemented: 0, not_implemented: 0, run: 0, not_run: 0 };
      const generation =
          snapshot === null ? null : await s(snapshot, "generation"),
        axes =
          snapshot === null || generation === "0"
            ? null
            : await f(await f(snapshot, "catalog_evidence"), "axes");
      if (axes) await axesValidate(axes, await f(snapshot, "objects"));
      for await (const item of items.items()) {
        await keys(
          item,
          [
            "id",
            "kind",
            "title",
            "domain",
            "branches",
            "implementation_status",
            "proof_status",
            "run_status",
            "evidence",
          ],
          "目录项",
        );
        const id = await t(item, "id");
        need(
          expected.ids.includes(id) && !seen.has(id),
          "目录id缺失/重复/额外",
        );
        seen.add(id);
        staticFields[id] = await select(item, [
          "kind",
          "title",
          "domain",
          "branches",
        ]);
        const impl = await s(item, "implementation_status"),
          run = await s(item, "run_status"),
          proof = await s(item, "proof_status");
        need(
          ["implemented", "not_implemented"].includes(impl) &&
            proof === "not_proved" &&
            ["run", "waiting", "not_run"].includes(run),
          "目录状态不符",
        );
        counts[impl]++;
        if (Object.hasOwn(counts, run)) counts[run]++;
        const axis = axes && (await axes.has(id)) ? await f(axes, id) : null;
        if (snapshot === null) continue;
        if (axis) {
          need(
            impl === (await s(axis, "impl_status")) &&
              proof === (await s(axis, "proof_status")) &&
              run === (await s(axis, "run_status")) &&
              (await equal(
                await f(item, "evidence"),
                await f(axis, "evidence"),
              )),
            "目录未绑定同cut完整轴",
          );
        } else
          need(
            impl === "not_implemented" &&
              proof === "not_proved" &&
              run === "not_run" &&
              (await equal(await f(item, "evidence"), {})),
            "目录回填未来事实",
          );
      }
      if (snapshot !== null)
        need(
          (await sha(staticFields)) === expected.public_static_sha256,
          "冻结目录全部静态列SHA不符",
        );
      const actual = await f(catalog, "counts");
      for (const k of Object.keys(counts))
        need((await s(actual, k)) === String(counts[k]), "目录状态计数不符");
      return staticFields;
    }
    async function validateState(
      state,
      protocol = "s-session/3",
      check = () => {},
      charge = () => {},
    ) {
      state = of(state);
      const snapshot = await f(state, "snapshot"),
        catalog = await f(state, "catalog"),
        cut = await f(state, "cut");
      const pubs = [
        "session_id",
        "generation",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
        "scope",
        "input_frontier",
        "seq_range",
        "catalog_run_status",
        "catalog_evidence",
      ];
      for (const k of pubs)
        need(
          await equal(await f(catalog, k), await f(snapshot, k)),
          "Catalog/Snapshot同cut字段不同：" + k,
        );
      for (const k of [
        "session_id",
        "generation",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
      ])
        need(
          await equal(await f(cut, k), await f(snapshot, k)),
          "State.cut字段不同",
        );
      const generation = await s(snapshot, "generation");
      await I(generation);
      need(
        (await s(snapshot, "structure_cut")) === "cut-" + generation,
        "cut代际错误",
      );
      await i(snapshot, "input_frontier", false, true);
      const seq = await f(snapshot, "seq_range");
      await keys(seq, ["from", "to"], "seq_range");
      await i(seq, "from");
      await i(seq, "to", false, true);
      need(
        (await equal(
          await f(seq, "to"),
          await f(snapshot, "input_frontier"),
        )) && (await equal(await f(snapshot, "scope"), SCOPE)),
        "输入前沿/scope错误",
      );
      const mode = await s(snapshot, "history_mode"),
        asof = await (await f(snapshot, "as_of_generation")).scalar();
      need(
        mode === "AsKnown"
          ? asof === generation
          : mode === "RecomputedWithRevision" && asof === null,
        "历史模式/cut错误",
      );
      const profile = await s(snapshot, "profile_id"),
        ph = await s(snapshot, "profile_hash"),
        rule = await s(snapshot, "rule_revision");
      // 空会话也必须先声明已知规则，不能等首次发布才选择合同。
      need(RULE_AXES.has(typed.splitRule(rule).doctrine), "cut规则修订未声明");
      if (generation === "0") {
        need(profile && /^[0-9a-f]{64}$/.test(ph), "cut0无冻结profile");
      } else {
        const e = await f(snapshot, "catalog_evidence");
        for (const k of [
          "classified_objects",
          "windows_total",
          "merged_bars",
          "effective_source_positions",
          "withdrawals",
          "replaces",
          "insufficient_knowledge",
          "domain_not_satisfied",
        ])
          await i(e, k);
        for (const k of [
          "profile_id",
          "profile_hash",
          "rule_revision",
          "structure_cut",
          "scope",
        ])
          need(
            await equal(await f(e, k), await f(snapshot, k)),
            "目录证据同cut来源绑定错误",
          );
        need(
          await equal(
            await f(e, "batch_id"),
            await f(snapshot, "index_frontier"),
          ),
          "目录证据batch不同",
        );
      }
      for (const family of FAMILIES) {
        const records = await f(snapshot, family);
        need(records.kind === "array", "六族不是数组");
        for await (const record of records.items()) {
          charge();
          need(record.kind === "object", "记录非对象");
          if (family === "objects" || family === "withdrawn_objects")
            need(
              (await s(record, "lifecycle")) ===
                (family === "objects" ? "active" : "withdrawn"),
              "六族/生命周期错位",
            );
          if (family === "relations") {
            await keys(
              record,
              ["subject", "relation_type", "object"],
              "完整关系",
            );
            for (const name of ["subject", "relation_type", "object"])
              await t(record, name);
          }
          if (family === "observations") {
            await keys(
              record,
              [
                "observation_id",
                "batch_id",
                "kind",
                "reason",
                "window_start",
                "window_mid",
                "window_end",
                "detail",
              ],
              "完整观察",
            );
            for (const name of ["observation_id", "batch_id", "kind", "reason"])
              await t(record, name);
          }

          await integerWalk(record);
          check();
        }
      }
      need(
        profile === "ohlc_integer_tb02a_v1",
        "Q3完整消费者只接受显式OHLC已批准profile",
      );
      // #1404/#1414：按 cut 规则修订选轴表（同 main validateTB02Candidate，先于逐轴校验）。
      if (generation !== "0")
        await keys(
          await f(await f(snapshot, "catalog_evidence"), "axes"),
          RULE_AXES.get(typed.splitRule(rule).doctrine),
          "cut规则目录轴",
        );
      await catalogValidate(catalog, snapshot);
      const binding = await bindSources(
        snapshot,
        charge,
        typed.splitRule(rule).layout,
      );
      await bindSegmentSequences(snapshot);
      for (const record of binding.records) {
        charge();
        let want;
        need(
          (await i(record, "first_known_generation", true)) <=
            BigInt(generation),
          "对象首次获知来自未来cut",
        );
        const withdrawn = await (
          await f(record, "withdrawn_generation")
        ).scalar();
        if (withdrawn !== null)
          need(BigInt(withdrawn) <= BigInt(generation), "撤回来自未来cut");
        if (protocol === "s-session/3") {
          objectId(await t(record, "object_id"));
          batch(await t(record, "batch_id"));
        }
        const kind = await t(record, "kind");
        if (typed.TB02_KINDS.includes(kind)) {
          const [biSchema, segmentSchema, domain] = typed.ruleContract(rule) || [null, null, null];
          const payload = await f(record, "payload");
          if (typed.SEGMENT_KINDS.includes(kind))
            need(await equal(await f(payload, "schema_revision"), segmentSchema), "段合同与cut规则版本不符");
          if (typed.BI_KINDS.includes(kind))
            need(await equal(await f(payload, "schema_revision"), biSchema), "新笔合同与cut规则版本不符");
          if (kind === "CC-054.knowledge_state" && domain !== null)
            need(await equal(await f(payload, "domain"), domain), "TB-02-A 知识域与cut规则版本不符");
          const p = {};
          for (const k of ["kind", "fact_key", "payload", "input_refs"])
            p[k] = await f(record, k);
          p.profile_id = profile;
          p.rule_revision = rule;
          want =
            protocol === "s-session/3"
              ? await identity(0x20, p, "obj3-")
              : "obj-" + (await sha(p));
        } else {
          const p = await shapeIdentity(
            record,
            binding.bound.get(await t(record, "object_id")),
            profile,
            rule,
          );
          want =
            protocol === "s-session/3"
              ? await identity(0x20, p, "obj3-")
              : "obj-" + (await sha(p));
        }
        need((await t(record, "object_id")) === want, "对象内容身份不符");
        check();
      }
      await auditWitnesses(snapshot, binding, charge);
      check();
      return binding;
    }
    // #1414/#1420：同 cut 活动集内 s-segment/2、/3 段与其 FIRST/SECOND 序列对象互绑；判据逐字同 main
    // validateTB02Candidate（payload 已由 typed 完整校验，此处在同一原件上做跨对象绑定）。
    async function bindSegmentSequences(snapshot) {
      const equalPlain = (a, b) => api.canonical(a) === api.canonical(b);
      const activeValues = new Map();
      for await (const record of (await f(snapshot, "objects")).items())
        activeValues.set(await t(record, "object_id"), record);
      for (const value of activeValues.values()) {
        if ((await s(value, "kind")) !== "CC-013.segment") continue;
        const r = await typed.plain(value);
        if (!["s-segment/2", "s-segment/3", "s-segment/4", "s-segment/5"].includes(r.payload.schema_revision)) continue;
        const d = r.payload.data;
        need(d.second_sequence_id !== null || d.fact_vector.scan_complete, "未完成扫描缺SECOND资源证据");
        for (const [prefix, role] of [["sequence", "FIRST_SEQUENCE"], ["second_sequence", "SECOND_SEQUENCE"]]) {
          const oid = d[prefix + "_object_id"]; if (oid === null) continue;
          const qValue = activeValues.get(oid), q = qValue ? await typed.plain(qValue) : undefined;
          need(q && q.kind === "CC-012.feature_sequence" && q.payload.schema_revision === r.payload.schema_revision, "段引用缺同cut序列对象");
          const v = q.payload.data;
          need(v.entity_id === d[prefix + "_id"] && v.segment_id === d.entity_id && v.role === role, "段序列引用身份/角色不符");
          need(q.source_coords.every(c => r.source_coords.includes(c)), "段缺序列实际来源");
          if (role === "FIRST_SEQUENCE" && d.strict !== null) need(v.facts.checks.some(c => equalPlain(c.elements, d.strict.elements)), "strict未绑定FIRST检查");
          if (role === "SECOND_SEQUENCE") need(d.fact_vector.scan_complete !== (v.facts.resource_truncated && v.facts.matched === null), "段事实可知性未绑定SECOND扫描证据");
          if (role === "SECOND_SEQUENCE" && d.result === "CASE_TWO") need(v.second_sequence_status === "CONFIRMED" && equalPlain(d.fact_vector.termination_witness.second_match, v.facts.matched), "第二种终结证书未绑定同次SECOND命中");
          if (role === "SECOND_SEQUENCE" && ["s-segment/3", "s-segment/4", "s-segment/5"].includes(r.payload.schema_revision) && ["KILLED_CASE_TWO", "PENDING_CASE_TWO"].includes(d.result)) need(v.second_sequence_status === (d.result === "KILLED_CASE_TWO" ? "KILLED" : "PENDING"), "段结果未绑定SECOND终态");
          if (role === "FIRST_SEQUENCE" && ["s-segment/4", "s-segment/5"].includes(r.payload.schema_revision)) typed.turnLinks(d, v.facts);
        }
        if (["s-segment/4", "s-segment/5"].includes(r.payload.schema_revision) && d.parse_invalid !== null) {
          // #1493 §7.1 第 6 条：段端门失败 fail-closed——同 cut 段版本与逐轴证据必须带阻断标记。
          const versions = [];
          for (const x of activeValues.values()) {
            if ((await s(x, "kind")) !== "CC-056.segment_version") continue;
            const pv = await typed.plain(x);
            if (pv.payload.schema_revision === r.payload.schema_revision) versions.push(pv);
          }
          need(versions.length > 0 && versions.every(x => x.payload.data.waiting_reasons.includes(typed.PARSE_INVALID_REASON)), "段端门失败未在段版本标阻断");
          const axes = await typed.plain(await f(await f(snapshot, "catalog_evidence"), "axes"));
          need(["CC-011", "CC-012", "CC-013", "CC-014"].every(cid => !Object.hasOwn(axes, cid) || axes[cid]?.evidence?.waiting_reasons?.includes(typed.PARSE_INVALID_REASON) === true), "段端门失败未在逐轴证据标阻断");
        }
      }
    }
    // #1394 整合（根裁定 (1)，(4) 扩到 v3；与 #1414 validate_once 同一原则）：同一 State 只完整验证一次。
    // s-session/2：LegacyClient.collect 先经页面钩子、再自己各调一次，两次入参只差 cut 的表示与
    // rule_revision 的取值来源（TB-02-D cut40 上两遍合计约 32–35 s，超过 30 s 读取期限）；
    // s-session/3：Q3.Client 采集先自己调一次、再经页面钩子调一次，是同一 state。
    // 缓存键 = 协议 + validateState 实际读取的全部输入（catalog、snapshot、cut 的五个同 cut 键）的内容
    // 编码的 SHA-256：普通值逐值编码；Reader 支持的图值取它已核的内容引用（kind、root、extent），root 是
    // Merkle 内容摘要，任一字节变化即换根。任一内容变化即换键重验。只记通过，失败不记；命中时按首验的
    // charge 次数原样重放（v3 的记录计费与预算判定不被缓存吃掉），再调一次 check() 守读取期限；判据一字不改。
    const ONCE_CUT_KEYS = [
      "session_id",
      "generation",
      "structure_cut",
      "catalog_revision",
      "index_frontier",
    ];
    const ONCE_CAPACITY = 2;
    const validatedOnce = new Map();
    // 普通对象（任一 realm 的 Object.prototype 或 null 原型）；Value 包装与数组不算。
    function plainRecord(value) {
      if (value === null || typeof value !== "object" || Array.isArray(value))
        return false;
      const proto = Object.getPrototypeOf(value);
      return proto === null || Object.getPrototypeOf(proto) === null;
    }
    const own = (value, key) => Object.prototype.hasOwnProperty.call(value, key);
    // 无 Reader 的 Value 只是普通值的包装；Reader 支持的图值返回 null（不可当普通容器展开）。
    const plainOf = (value) =>
      value instanceof V.Value ? (value.reader ? null : value.plain) : value;
    // 带类型标签的内容编码：不同类型、不同结构不会编成同一串；不可编码的值抛出，交给完整验证。
    function onceShape(value) {
      if (value instanceof V.Value) {
        if (value.reader) {
          const ref = value.ref;
          return [
            "ref",
            ref.kind,
            ref.root,
            ref.extent === null ? "" : String(ref.extent),
          ];
        }
        return onceShape(value.plain);
      }
      if (value === null) return ["null"];
      if (typeof value === "boolean") return ["bool", value ? "1" : "0"];
      if (typeof value === "string") return ["text", value];
      if (V.isNumber(value)) return ["number", value.token];
      if (Array.isArray(value)) {
        const items = ["array"];
        for (let i = 0; i < value.length; i++) {
          need(own(value, i), "稀疏数组不进缓存");
          items.push(onceShape(value[i]));
        }
        return items;
      }
      need(plainRecord(value), "不可编码值不进缓存");
      const fields = ["object"];
      for (const key of Object.keys(value).sort())
        fields.push([key, onceShape(value[key])]);
      return fields;
    }
    async function onceKey(state, protocol) {
      if (protocol !== "s-session/2" && protocol !== "s-session/3") return null;
      const root = plainOf(state);
      if (
        !plainRecord(root) ||
        !["catalog", "snapshot", "cut"].every((k) => own(root, k))
      )
        return null;
      const cut = plainOf(root.cut);
      if (!plainRecord(cut) || !ONCE_CUT_KEYS.every((k) => own(cut, k)))
        return null;
      try {
        return await api.sha256([
          "q3-validate-once/1",
          protocol,
          onceShape(root.catalog),
          onceShape(root.snapshot),
          onceShape(Object.fromEntries(ONCE_CUT_KEYS.map((k) => [k, cut[k]]))),
        ]);
      } catch {
        return null; // 无规范内容编码的入参不进缓存，交给完整验证去拒绝。
      }
    }
    async function validateStateOnce(
      state,
      protocol = "s-session/3",
      check = () => {},
      charge = () => {},
    ) {
      const key = await onceKey(state, protocol);
      if (key !== null && validatedOnce.has(key)) {
        const entry = validatedOnce.get(key);
        validatedOnce.delete(key);
        validatedOnce.set(key, entry);
        for (let n = 0; n < entry.charges; n++) charge();
        check();
        return entry.binding;
      }
      let charges = 0;
      const binding = await validateState(state, protocol, check, () => {
        charges++;
        charge();
      });
      if (key !== null) {
        validatedOnce.set(key, { binding, charges });
        while (validatedOnce.size > ONCE_CAPACITY)
          validatedOnce.delete(validatedOnce.keys().next().value);
      }
      return binding;
    }
    async function validateDelta(outer, charge = () => {}) {
      outer = of(outer);
      await keys(outer, [...D, "delta"], "Delta外层");
      const inner = await f(outer, "delta");
      await keys(inner, [...D, ...CHANGES], "Delta完整内层");
      for (const k of D)
        need(
          await equal(await f(outer, k), await f(inner, k)),
          "Delta内外字段不同：" + k,
        );
      const generation = await s(inner, "generation");
      await I(generation, true);
      need(
        (await s(inner, "next_cut")) === "cut-" + generation,
        "Delta cut不符",
      );
      batch(await s(inner, "index_frontier"));
      const range = await f(inner, "seq_range");
      await keys(range, ["from", "to"], "Delta.seq_range");
      await i(range, "from");
      await i(range, "to", false, true);
      need(
        await equal(await f(range, "to"), await f(inner, "input_frontier")),
        "Delta前沿不符",
      );
      const replacements = new Set();
      for (const name of CHANGES) {
        const arr = await f(inner, name);
        need(arr.kind === "array", "Delta变化集合缺失");
        for await (const record of arr.items()) {
          charge();
          await integerWalk(record);
        }
      }
      for await (const w of (await f(inner, "withdrawals")).items()) {
        charge();
        const hasFact = await w.has("fact_key");
        await keys(
          w,
          hasFact
            ? ["object_id", "fact_key", "reason", "superseded_by"]
            : [
                "object_id",
                "reason",
                "superseded_by",
                "window_start",
                "window_mid",
                "window_end",
              ],
          "撤回",
        );
        const reason = await t(w, "reason"),
          sup = await (await f(w, "superseded_by")).scalar();
        need(
          ["fact_removed", "superseded_by_revision"].includes(reason) &&
            (reason === "fact_removed") === (sup === null),
          "撤回原因/替代不符",
        );
        if (sup !== null)
          replacements.add(
            JSON.stringify([await t(w, "object_id"), await T(sup)]),
          );
      }
      const actual = new Set();
      for await (const r of (await f(inner, "replaces")).items()) {
        charge();
        await keys(r, ["old_object_id", "new_object_id"], "替代");
        const key = JSON.stringify([
          await t(r, "old_object_id"),
          await t(r, "new_object_id"),
        ]);
        need(!actual.has(key), "替代端点重复");
        actual.add(key);
      }
      need(
        actual.size === replacements.size &&
          [...actual].every((k) => replacements.has(k)),
        "Delta遗漏/额外replaces",
      );
      const e = await f(inner, "catalog_evidence");
      await axesValidate(await f(e, "axes"));
      return inner;
    }
    async function comparePublication(delta, snapshot) {
      const dd = await f(delta, "delta");
      for (const [dk, sk] of [
        ["session_id", "session_id"],
        ["generation", "generation"],
        ["catalog_revision", "catalog_revision"],
        ["next_cut", "structure_cut"],
        ["index_frontier", "index_frontier"],
        ["input_frontier", "input_frontier"],
        ["seq_range", "seq_range"],
        ["catalog_run_status", "catalog_run_status"],
        ["catalog_evidence", "catalog_evidence"],
      ])
        need(
          await equal(await f(dd, dk), await f(snapshot, sk)),
          "Delta/Snapshot字段不同：" + sk,
        );
      for (const k of ["profile_id", "profile_hash", "scope"])
        need(
          await equal(
            await f(await f(dd, "catalog_evidence"), k),
            await f(snapshot, k),
          ),
          "Delta来源不同",
        );
    }
    class Client {
      #committed = null;
      get committed() {
        return this.#committed;
      }
      constructor({
        fetch: fetcher,
        hooks = {},
        onCommit = () => {},
        limits,
        allocation,
        byteDeployment = null,
        clock = () => Math.floor(performance.now()),
      }) {
        need(
          typeof (fetcher || globalThis.fetch) === "function",
          "缺网络读取能力",
        );
        this.fetch = fetcher || globalThis.fetch.bind(globalThis);
        this.byteDeployment = byteDeployment;
        this.hooks = hooks;
        this.onCommit = onCommit;
        this.limits = jobs.validateConfig(limits);
        const configIdentity = {
          schema_revision: this.limits.schema_revision,
          job_limits: this.limits.job_limits,
        };
        for (const key of [
          "pageSize",
          "watchBatches",
          "pollMs",
          "timeoutMs",
          "replyBytes",
        ])
          configIdentity[key] = number(String(this.limits[key]));
        this.configDigest = sha(configIdentity);
        need(
          allocation &&
            allocation.contract === "s-query-private-allocation/1" &&
            typeof allocation.measure === "function" &&
            typeof allocation.reserveBound === "function" &&
            typeof allocation.write === "function" &&
            typeof allocation.release === "function",
          "未验证：没有实际分配测量/暂存适配器，不能宣称Browser物理限额通过",
        );
        need(
          typeof allocation.measurementSource === "string" &&
            allocation.measurementSource.length > 0,
          "测量适配器缺具名实际来源",
        );
        api.canonical(allocation.measurementSource);
        this.allocation = allocation;
        this.clock = clock;
        this.operation = 0;
        this.serial = 0n;
        this.instance =
          globalThis.crypto?.randomUUID?.() ||
          (typeof require === "function"
            ? require("node:crypto").randomUUID()
            : null);
        need(this.instance, "缺查询身份");
        this.active = null;
        this.lastJob = null;
      }
      cancel() {
        // 立即禁止旧operation安装并停止纯Job，Hash晚到不准入；
        // 已发起I/O的真实尾量仍由同operation诊断账保留，不能复活纯Job。
        this.operation++;
        this.active?.job.cancel();
      }
      async begin() {
        this.cancel();
        const operation = this.operation,
          started = this.clock(),
          abort = new AbortController();
        const elapsedCap = BigInt(this.limits.job_limits.max_elapsed_ms);
        const waitLimit =
          elapsedCap < BigInt(this.limits.timeoutMs)
            ? Number(elapsedCap)
            : this.limits.timeoutMs;
        let measureTimer, measured;
        try {
          measured = await Promise.race([
            this.allocation.measure({ operation, signal: abort.signal }),
            new Promise((_, reject) => {
              measureTimer = setTimeout(() => {
                abort.abort();
                reject(new Error("实际分配基线测量超过本operation绝对期限"));
              }, waitLimit);
            }),
          ]);
        } finally {
          clearTimeout(measureTimer);
        }
        need(operation === this.operation, "旧分配测量晚到");
        need(
          measured &&
            measured.exact === true &&
            measured.source === this.allocation.measurementSource,
          "缺exact measured基线",
        );
        const coreJob = new jobs.Job(
          this.limits.job_limits,
          {
            store_bytes: measured.store_bytes,
            staged_bytes: measured.staged_bytes,
          },
          (() => {
            let first = true;
            return () => {
              if (first) {
                first = false;
                return started;
              }
              return this.clock();
            };
          })(),
        );
        const job = operationLedger(coreJob, operation);
        job.beginFirstRead();
        const budget = { operation, job, readers: [], identity: null };
        this.active = budget;
        this.lastJob = job;
        this.check(budget);
        return budget;
      }
      check(b) {
        need(
          b.operation === this.operation && this.active === b,
          "旧操作/取消结果不得安装",
        );
        b.job.checkDeadline();
      }
      async store(node, b) {
        this.check(b);
        const requested = await this.allocation.reserveBound(node);
        this.check(b);
        const grant = b.job.reserveStaged(requested);
        let actual;
        try {
          actual = await this.allocation.write(node, {
            operation: b.operation,
          });
        } catch (e) {
          if (e.allocated_bytes !== undefined)
            b.job.reconcile(grant, e.allocated_bytes);
          else b.job.release(grant);
          throw e;
        }
        need(
          actual &&
            actual.exact === true &&
            actual.source === this.allocation.measurementSource,
          "写后实际分配没有exact measured对账",
        );
        b.job.reconcile(grant, actual.allocated_bytes);
        this.check(b);
      }
      async request(identity, payload, b, reservedSerial = null, fetcher = this.fetch) {
        this.check(b);
        identity = Object.freeze(smallIdentity(identity));
        payload = api.parse(api.canonical(payload));
        need(
          typeof identity.session_id === "string" && identity.session_id,
          "会话身份为空",
        );
        await I(identity.session_generation, true);
        b.job.beginFirstRead();
        const serial = reservedSerial ?? ++this.serial;
        const request = bytePlan.requestEnvelope(smallIdentity(identity), payload,
          this.instance, serial, await api.sha256(payload));
        this.check(b);
        const body = api.canonical(request),
          abort = new AbortController(),
          started = this.clock(),
          timer = setTimeout(() => abort.abort(), this.limits.timeoutMs);
        let reader;
        try {
          b.job.preIoCheck();
          let pendingResponse, launchError;
          try {
            pendingResponse = Promise.resolve(
              fetcher("/api/v3/" + payload.op, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body,
                signal: abort.signal,
                cache: "no-store",
              }),
            );
          } catch (error) {
            launchError = error;
          }
          if (pendingResponse) pendingResponse.catch(() => {});
          chargeActualIo(
            b.job,
            1n,
            BigInt(new TextEncoder().encode(body).length),
          );
          if (launchError) throw launchError;
          const response = await pendingResponse;
          this.check(b);
          need(
            response.body && typeof response.body.getReader === "function",
            "缺有限响应流",
          );
          reader = response.body.getReader();
          const chunks = [];
          let length = 0;
          while (true) {
            this.check(b);
            need(
              this.clock() - started < this.limits.timeoutMs,
              "单请求期限已过",
            );
            const { done, value } = await reader.read();
            if (value) {
              length += value.byteLength;
              b.job.chargeWireBytes(BigInt(value.byteLength));
            }
            this.check(b);
            need(length <= this.limits.replyBytes, "响应超过显式replyBytes");
            if (done) break;
            chunks.push(value);
          }
          const bytes = new Uint8Array(length);
          let at = 0;
          for (const chunk of chunks) {
            bytes.set(chunk, at);
            at += chunk.byteLength;
          }
          const reply = api.parse(
            new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
              bytes,
            ),
          );
          await keys(reply, HEADER, "响应11字段");
          need(reply.schema_revision === "s-session/3", "响应协议不符");
          for (const k of HEADER.filter(
            (k) => !["causal_refs", "payload"].includes(k),
          ))
            await T(reply[k]);
          await I(reply.session_generation, true);
          await I(reply.producer_epoch, true);
          digest(reply.payload_hash);
          need(
            reply.source_namespace === "s-observe/replies" &&
              reply.source_epoch === reply.producer_epoch &&
              reply.producer_id === "Q:" + reply.session_id,
            "响应来源/生产者身份错误",
          );
          const causal = {};
          for (const k of [
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
          ])
            causal[k] = request[k];
          need(await equal(reply.causal_refs, [causal]), "因果身份不同");
          need(
            reply.payload_hash === (await api.sha256(reply.payload)),
            "payload_hash错误",
          );
          need(
            reply.message_id ===
              "reply-" +
                (await api.sha256([
                  request.source_namespace,
                  request.source_epoch,
                  request.message_id,
                  reply.payload_hash,
                ])),
            "reply消息身份错误",
          );
          this.check(b);
          need(
            response.ok && reply.payload.ok === true,
            "查询未完成：" + (reply.payload.error || response.status),
          );
          return reply;
        } finally {
          clearTimeout(timer);
          abort.abort();
          if (reader) {
            await reader.cancel().catch(() => {});
            reader.releaseLock();
          }
        }
      }
      async fixed(fixed) {
        await keys(fixed, FIXED, "fixed_cut");
        await T(fixed.session_id);
        await I(fixed.session_generation, true);
        await I(fixed.cut_generation);
        await I(fixed.input_frontier, false, true);
        await T(fixed.catalog_revision);
        await T(fixed.rule_revision);
        await S(fixed.profile_id);
        await S(fixed.profile_hash);
        need(
          fixed.structure_cut === "cut-" + fixed.cut_generation &&
            (await equal(fixed.scope, SCOPE)),
          "fixed cut/scope错误",
        );
        await keys(fixed.seq_range, ["from", "to"], "seq_range");
        await I(fixed.seq_range.from);
        await I(fixed.seq_range.to, false, true);
        need(fixed.seq_range.to === fixed.input_frontier, "fixed前沿错误");
        need(
          fixed.history_mode === "AsKnown"
            ? fixed.as_of_generation === fixed.cut_generation
            : fixed.history_mode === "RecomputedWithRevision" &&
                fixed.as_of_generation === null,
          "历史cut错误",
        );
        content.validateRef(fixed.catalog_evidence);
        if (fixed.cut_generation !== "0") batch(fixed.index_frontier);
      }
      async token(token, fixed, projectionDigest, offset, b) {
        await keys(token, TOKEN, "Snapshot token");
        need(
          token.token_schema === "s-snapshot-token/2" &&
            token.order_version === "s-record-order/2" &&
            token.page_size === String(this.limits.pageSize) &&
            token.offset === String(offset),
          "token版本/顺序/范围错误",
        );
        digest(token.issued_capture_digest);
        digest(token.token_checksum);
        for (const key of [
          "session_id",
          "session_generation",
          "cut_generation",
          "scope",
          "history_mode",
          "structure_cut",
          "catalog_revision",
          "index_frontier",
          "profile_hash",
          "rule_revision",
        ])
          need(
            await equal(token[key], fixed[key]),
            "token fixed身份错误：" + key,
          );
        need(
          token.cut_projection_digest === projectionDigest,
          "token投影根错误",
        );
        const unsigned = { ...token };
        delete unsigned.token_checksum;
        need(
          (await api.sha256(unsigned)) === token.token_checksum,
          "token checksum错误",
        );
        this.check(b);
      }
      chooseBytes(input, identity, b) {
        this.check(b);
        let deployment, plan;
        try {
          deployment = this.byteDeployment?.snapshot({fetch: this.fetch, ready: b.ready, identity});
          plan = bytePlan.choose({...input, identity: smallIdentity(identity), instance: this.instance,
            serial: this.serial, pageSize: this.limits.pageSize, replyBytes: this.limits.replyBytes,
            maxRequests: this.limits.job_limits.max_total_requests, deployment});
        } catch { return null; }
        if (!plan) return null;
        let serial = this.serial;
        const fetcher = this.fetch;
        this.serial = plan.serialEnd;
        return {limit: plan.limit, request: async payload => {
          need(++serial <= plan.serialEnd, "bytes证书请求区间已用完");
          const reply = await this.request(identity, payload, b, serial, fetcher);
          need(sameIdentity(reply, identity), "片段会话化身不同");
          need(reply.producer_epoch === b.ready.producer_epoch, "bytes部署纪元已变化");
          return reply.payload;
        }};
      }
      reader(anchor, identity, b) {
        content.validateAnchor(anchor);
        const r = new V.Reader({
          anchor,
          request: async (p) => {
            const e = await this.request(identity, p, b);
            need(sameIdentity(e, identity), "片段会话化身不同");
            return e.payload;
          },
          check: () => this.check(b),
          accept: async (n) => {
            this.check(b);
            await b.job.acceptNode(n);
            this.check(b);
          },
          store: (n) => this.store(n, b),
          limit: this.limits.pageSize,
          chooseBytes: this.byteDeployment ? input => this.chooseBytes(input, identity, b) : null,
        });
        b.readers.push(r);
        return r;
      }
      async collect(
        {
          identity,
          mode,
          generation = null,
          token = null,
          expectedFixed = null,
        },
        b,
      ) {
        this.check(b);
        need(
          mode === "AsKnown" || mode === "RecomputedWithRevision",
          "历史模式无效",
        );
        if (generation !== null) await I(generation);
        if (token) {
          await this.fixed(expectedFixed);
          await this.token(
            token,
            expectedFixed,
            token.cut_projection_digest,
            0,
            b,
          );
        }
        let next = token
            ? { op: "snapshot", snapshot_token: token }
            : {
                op: "snapshot",
                ...smallIdentity(identity),
                scope: SCOPE,
                history_mode: mode,
                as_of_generation: generation,
                page_size: String(this.limits.pageSize),
                order_version: "s-record-order/2",
              },
          base = null,
          issued = token?.issued_capture_digest,
          rows = [],
          seen = new Set(),
          objectIds = new Set(),
          pages = 0;
        for (;;) {
          const envelope = await this.request(identity, next, b),
            page = envelope.payload;
          pages++;
          await keys(page, PAGE, "Snapshot page");
          await this.fixed(page.fixed_cut);
          need(
            sameIdentity(envelope, identity) &&
              sameIdentity(page.fixed_cut, identity),
            "Snapshot会话/化身不符",
          );
          need(
            page.fixed_cut.history_mode === mode &&
              (generation === null ||
                page.fixed_cut.cut_generation === generation),
            "Snapshot模式/cut不符",
          );
          if (expectedFixed)
            need(await equal(page.fixed_cut, expectedFixed), "Gap指定切面不同");
          need(
            (await equal(page.scope, SCOPE)) &&
              (await equal(page.omissions, [])) &&
              page.order_version === "s-record-order/2",
            "Snapshot scope/order/omissions错误",
          );
          need(
            /^qcut3-[0-9a-f]{64}$/.test(page.cut_projection_digest),
            "qcut3格式错误",
          );
          digest(page.validated_capture_digest);
          content.validateRef(page.catalog);
          await keys(page.counts, [...FAMILIES, "total"], "counts");
          let total = 0n;
          for (const family of FAMILIES) total += await I(page.counts[family]);
          need(total === (await I(page.counts.total)), "六族总计数不同");
          need(
            Array.isArray(page.rows) &&
              typeof page.done === "boolean" &&
              page.offset === String(rows.length),
            "page offset/done/rows错误",
          );
          const metadata = {};
          for (const key of [
            "fixed_cut",
            "catalog",
            "counts",
            "scope",
            "omissions",
            "order_version",
            "cut_projection_digest",
          ])
            metadata[key] = page[key];
          if (base) need(await equal(base, metadata), "跨页固定头变化");
          else {
            base = metadata;
            issued ||= page.validated_capture_digest;
          }
          const rest = total - BigInt(rows.length),
            length =
              rest < BigInt(this.limits.pageSize)
                ? rest
                : BigInt(this.limits.pageSize);
          need(
            rest >= 0n &&
              BigInt(page.rows.length) === length &&
              page.done === (rest === length),
            "page缺漏/截断/空转",
          );
          for (const row of page.rows) {
            await keys(row, ["family", "key", "record"], "row");
            need(
              FAMILIES.includes(row.family) && Array.isArray(row.key),
              "未知族/key",
            );
            content.validateRef(row.record);
            need(row.record.kind === "object", "record引用非对象");
            for (const key of row.key) await T(key);
            const id = api.canonical([row.family, row.key]);
            need(!seen.has(id), "公开key重复");
            seen.add(id);
            if (["objects", "withdrawn_objects"].includes(row.family)) {
              need(
                row.key.length === 1 && !objectIds.has(row.key[0]),
                "活动/撤回身份冲突",
              );
              objectIds.add(row.key[0]);
            }
            rows.push(row);
          }
          this.check(b);
          if (page.done) {
            need(page.next_token === null, "末页仍带token");
            break;
          }
          await this.token(
            page.next_token,
            base.fixed_cut,
            base.cut_projection_digest,
            rows.length,
            b,
          );
          need(
            page.next_token.issued_capture_digest === issued,
            "token签发capture变化",
          );
          next = { op: "snapshot", snapshot_token: page.next_token };
        }
        const counts = {};
        for (const family of FAMILIES) counts[family] = 0n;
        for (const row of rows) counts[row.family]++;
        for (const family of FAMILIES)
          need(counts[family].toString() === base.counts[family], "族计数不符");
        const fixedWire = base.fixed_cut,
          logicFixed = {
            ...fixedWire,
            seq_range: { ...fixedWire.seq_range },
            scope: { ...fixedWire.scope },
            catalog_evidence: content.refEdge(fixedWire.catalog_evidence),
          },
          logical = {
            fixed_cut: logicFixed,
            catalog: content.refEdge(base.catalog),
            rows: rows.map((row) => ({
              ...row,
              key: [...row.key],
              record: content.refEdge(row.record),
            })),
            counts: { ...base.counts },
            scope: { ...base.scope },
            omissions: [],
            order_version: base.order_version,
          };
        const built = await content.encode(content.fromPlain(logical));
        this.check(b);
        need(
          "qcut3-" + V.F(0x22, V.unhex(built.ref.root.slice(4))) ===
            base.cut_projection_digest,
          "完整七字段projection qcut3不符",
        );
        const anchor = {
            kind: "projection",
            fixed_cut: fixedWire,
            cut_projection_digest: base.cut_projection_digest,
            projection_ref: built.ref,
          },
          reader = this.reader(anchor, identity, b),
          root = reader.value(built.ref);
        await reader.complete(root);
        this.check(b);
        await reader.finish();
        this.check(b);
        const fixed = await root.field("fixed_cut"),
          catalog = await root.field("catalog"),
          families = {};
        for (const family of FAMILIES) families[family] = [];
        let previous = null;
        for await (const row of (await root.field("rows")).items()) {
          b.job.chargeRecords(1n);
          this.check(b);
          const family = await t(row, "family"),
            record = await row.field("record"),
            key = await rowKey(family, record);
          need(
            await equal(await row.field("key"), key),
            "row.key与真实记录派生不同",
          );
          const tupleValue = await tuple(family, record),
            order = { family, tuple: tupleValue };
          if (previous) {
            const fc =
              FAMILIES.indexOf(family) - FAMILIES.indexOf(previous.family);
            need(
              fc > 0 ||
                (fc === 0 && compareTuples(previous.tuple, tupleValue) < 0),
              "六族原顺序/元组没有严格递增",
            );
          }
          previous = order;
          families[family].push(record);
        }
        const snapshotFields = {};
        for await (const e of fixed.entries())
          if (e.key !== "cut_generation") snapshotFields[e.key] = e.value;
        snapshotFields.generation = fixedWire.cut_generation;
        for (const family of FAMILIES)
          snapshotFields[family] = families[family];
        const snapshot = of(snapshotFields),
          cut = {
            ...smallIdentity(identity),
            generation: fixedWire.cut_generation,
            structure_cut: fixedWire.structure_cut,
            catalog_revision: fixedWire.catalog_revision,
            index_frontier: fixedWire.index_frontier,
          },
          state = of({ ok: true, catalog, snapshot, cut });
        // 与页面钩子共用同一 validate-once 入口：同一 state 只完整验证一次（根裁定 (4)）。
        await validateStateOnce(
          state,
          "s-session/3",
          () => this.check(b),
          () => b.job.chargeRecords(1n),
        );
        this.check(b);
        if (this.hooks.validateState)
          await this.hooks.validateState(state, "s-session/3");
        this.check(b);
        const cursor = {
          ...smallIdentity(identity),
          catalog_revision: fixedWire.catalog_revision,
          scope: fixedWire.scope,
          after_generation: fixedWire.cut_generation,
          base_cut: fixedWire.structure_cut,
          index_frontier: fixedWire.index_frontier,
          last_seq: fixedWire.input_frontier,
          order_version: "s-record-order/2",
        };
        return {
          state,
          projection: root,
          fixed: fixedWire,
          cursor,
          pages,
          families,
          reader,
          fullyValidated: true,
          resources_sha256: await this.configDigest,
          allocation_measurement_source: this.allocation.measurementSource,
          allocation_test_only: this.allocation.testOnly === true,
        };
      }
      async commit(candidate, b, event) {
        this.check(b);
        need(candidate.fullyValidated, "未完成候选不得安装");
        for (const reader of b.readers) await reader.finish();
        this.check(b);
        const finalMeasure = await this.allocation.measure();
        this.check(b);
        need(
          finalMeasure.exact === true &&
            finalMeasure.source === this.allocation.measurementSource,
          "安装前缺实际测量",
        );
        const ledger = b.job.snapshot();
        need(
          BigInt(finalMeasure.store_bytes) === BigInt(ledger.store.exposure) &&
            BigInt(finalMeasure.staged_bytes) ===
              BigInt(ledger.staged.exposure),
          "安装前实际分配/累计Job账不符",
        );
        b.job.finishCheck();
        this.check(b);
        for (const reader of b.readers) reader.freeze();
        for (const family of FAMILIES)
          Object.freeze(candidate.families[family]);
        Object.freeze(candidate.families);
        Object.freeze(candidate.cursor.scope);
        Object.freeze(candidate.cursor);
        this.#committed = candidate;
        try {
          this.onCommit(candidate, event);
        } catch (error) {
          candidate.presentation_error = String(error);
        }
        return candidate;
      }
      async run(fn) {
        const b = await this.begin();
        try {
          return await fn(b);
        } catch (error) {
          this.lastFailure = Object.freeze({
            operation: b.operation,
            code: error.code || "query_validation",
            reason: String(error),
          });
          b.job.cancel();
          try {
            const deleted = await this.allocation.release({
              operation: b.operation,
            });
            if (
              deleted?.exact === true &&
              deleted.source === this.allocation.measurementSource
            )
              b.job.reclaimStaged(deleted.deleted_bytes);
          } catch {}
          throw error;
        }
      }
      async ready(b) {
        this.check(b);
        b.job.beginFirstRead();
        const abort = new AbortController(),
          timer = setTimeout(() => abort.abort(), this.limits.timeoutMs);
        let reader;
        try {
          b.job.preIoCheck();
          let pendingResponse, launchError;
          try {
            pendingResponse = Promise.resolve(
              this.fetch("/api/ready", {
                method: "GET",
                cache: "no-store",
                signal: abort.signal,
              }),
            );
          } catch (error) {
            launchError = error;
          }
          if (pendingResponse) pendingResponse.catch(() => {});
          b.job.chargeRequests(1n);
          if (launchError) throw launchError;
          const response = await pendingResponse;
          this.check(b);
          need(response.body?.getReader, "Ready缺有限字节流");
          reader = response.body.getReader();
          const chunks = [];
          let total = 0;
          while (true) {
            this.check(b);
            const item = await reader.read();
            if (item.value) {
              total += item.value.byteLength;
              b.job.chargeWireBytes(BigInt(item.value.byteLength));
            }
            this.check(b);
            need(total <= this.limits.replyBytes, "Ready超过replyBytes");
            if (item.done) break;
            chunks.push(item.value);
          }
          const bytes = new Uint8Array(total);
          let offset = 0;
          for (const c of chunks) {
            bytes.set(c, offset);
            offset += c.length;
          }
          const ready = api.parse(
            new TextDecoder("utf-8", { fatal: true }).decode(bytes),
          );
          await keys(
            ready,
            ["ok", "cut", "producer_epoch", "control_instance_id"],
            "Ready",
          );
          need(response.ok && ready.ok === true, "Ready未完成");
          await I(ready.producer_epoch, true);
          await S(ready.control_instance_id);
          await keys(
            ready.cut,
            [
              "session_id",
              "session_generation",
              "generation",
              "structure_cut",
              "catalog_revision",
              "index_frontier",
            ],
            "Ready cut",
          );
          await T(ready.cut.session_id);
          await I(ready.cut.session_generation, true);
          await I(ready.cut.generation);
          need(
            ready.cut.structure_cut === "cut-" + ready.cut.generation,
            "Ready cut不同",
          );
          await T(ready.cut.catalog_revision);
          if (ready.cut.generation !== "0") batch(ready.cut.index_frontier);
          this.check(b);
          b.ready = ready;
          return ready;
        } finally {
          clearTimeout(timer);
          abort.abort();
          if (reader) {
            await reader.cancel().catch(() => {});
            reader.releaseLock();
          }
        }
      }
      async loadReady(mode = "RecomputedWithRevision", generation = null) {
        return this.run(async (b) => {
          if (mode === "AsKnown") await I(generation);
          const ready = await this.ready(b);
          const candidate = await this.collect(
            { identity: smallIdentity(ready.cut), mode, generation },
            b,
          );
          candidate.ready = ready;
          return this.commit(candidate, b, { kind: "snapshot" });
        });
      }
      async load(
        discovery,
        mode = "RecomputedWithRevision",
        generation = null,
      ) {
        const discoveredCut = discovery?.cut;
        discovery = {
          schema_revision: discovery?.schema_revision,
          cut: discoveredCut
            ? Object.freeze(smallIdentity(discoveredCut))
            : null,
        };
        return this.run(async (b) => {
          if (mode === "AsKnown") await I(generation);
          need(
            discovery?.schema_revision === "s-session/3" && discovery?.cut,
            "需要显式Q3 Ready身份",
          );
          const identity = smallIdentity(discovery.cut);
          await T(identity.session_id);
          await I(identity.session_generation, true);
          return this.commit(
            await this.collect({ identity, mode, generation }, b),
            b,
            { kind: "snapshot" },
          );
        });
      }
      async applyBatch(candidate, delta, b) {
        const inner = await validateDelta(delta, () => b.job.chargeRecords(1n));
        if (this.hooks.validateDelta) await this.hooks.validateDelta(delta);
        this.check(b);
        const maps = {};
        for (const family of FAMILIES) {
          const map = new Map();
          for (const r of candidate.families[family]) {
            b.job.chargeRecords(1n);
            map.set(api.canonical(await rowKey(family, r)), r);
          }
          maps[family] = map;
        }
        for (const name of CHANGES) {
          const unique = new Set();
          for await (const r of (await f(inner, name)).items()) {
            b.job.chargeRecords(1n);
            this.check(b);
            const family = {
                upserts: "objects",
                withdrawals: "objects",
                witnesses: "witnesses",
                relations: "relations",
                observations: "observations",
                raw_history_added: "raw_history",
              }[name],
              key =
                name === "replaces"
                  ? api.canonical([
                      await t(r, "old_object_id"),
                      await t(r, "new_object_id"),
                    ])
                  : api.canonical(await rowKey(family, r));
            need(!unique.has(key), "Delta变化集合重复身份：" + name);
            unique.add(key);
          }
        }
        for await (const w of (await f(inner, "withdrawals")).items()) {
          const key = api.canonical([await t(w, "object_id")]),
            prior = maps.objects.get(key);
          need(prior, "撤回不存在于已提交活动对象");
          for (const name of (await w.has("fact_key"))
            ? ["fact_key"]
            : ["window_start", "window_mid", "window_end"])
            need(
              await equal(await f(w, name), await f(prior, name)),
              "撤回语义槽/窗口与原活动对象不同",
            );
          const overlay = {};
          for await (const e of prior.entries()) overlay[e.key] = e.value;
          overlay.lifecycle = "withdrawn";
          overlay.withdrawal_reason = await f(w, "reason");
          overlay.superseded_by = await f(w, "superseded_by");
          overlay.withdrawn_generation = await f(inner, "generation");
          maps.withdrawn_objects.set(key, of(overlay));
          maps.objects.delete(key);
        }
        for await (const r of (await f(inner, "upserts")).items()) {
          need(
            (await s(r, "lifecycle")) === "active",
            "Delta upsert生命周期不同",
          );
          const key = api.canonical(await rowKey("objects", r));
          maps.objects.set(key, r);
          maps.withdrawn_objects.delete(key);
        }
        for (const [name, family] of [
          ["witnesses", "witnesses"],
          ["relations", "relations"],
          ["observations", "observations"],
          ["raw_history_added", "raw_history"],
        ])
          for await (const r of (await f(inner, name)).items())
            maps[family].set(api.canonical(await rowKey(family, r)), r);
        const families = {};
        for (const family of FAMILIES) {
          const ordered = [];
          for (const r of maps[family].values())
            ordered.push({ r, tuple: await tuple(family, r) });
          ordered.sort((a, b) => compareTuples(a.tuple, b.tuple));
          families[family] = ordered.map((x) => x.r);
        }
        await axesValidate(
          await f(await f(inner, "catalog_evidence"), "axes"),
          families.objects,
        );
        return families;
      }
      async watch() {
        return this.run(async (b) => {
          need(this.committed, "尚未安装完整Snapshot");
          const before = this.committed;
          for (const node of before.reader.cache.values()) {
            this.check(b);
            await b.job.acceptNode(node);
            this.check(b);
          }
          const reply = await this.request(
              before.cursor,
              {
                op: "watch",
                cursor: before.cursor,
                max_batches: String(this.limits.watchBatches),
                client_id: this.instance,
              },
              b,
            ),
            page = reply.payload;
          await keys(
            page,
            [
              "deltas",
              "next_cursor",
              "observed_head",
              "delivery_frontier",
              "has_more",
              "gap",
              "validated_capture_digest",
              "ok",
            ],
            "Watch",
          );
          await this.fixed(page.observed_head);
          digest(page.validated_capture_digest);
          need(
            sameIdentity(reply, page.observed_head) &&
              page.observed_head.history_mode === "RecomputedWithRevision",
            "Watch观测头身份/模式不同",
          );
          await keys(
            page.delivery_frontier,
            [
              "policy_revision",
              "retain_generations",
              "first_available_generation",
              "head_generation",
            ],
            "投递前沿",
          );
          await T(page.delivery_frontier.policy_revision);
          await I(page.delivery_frontier.retain_generations, true);
          await I(page.delivery_frontier.first_available_generation, true);
          await I(page.delivery_frontier.head_generation);
          need(
            page.delivery_frontier.head_generation ===
              page.observed_head.cut_generation &&
              BigInt(page.delivery_frontier.first_available_generation) <=
                BigInt(page.delivery_frontier.head_generation) + 1n,
            "投递前沿/head不同",
          );
          need(
            Array.isArray(page.deltas) &&
              page.deltas.length <= this.limits.watchBatches &&
              typeof page.has_more === "boolean",
            "Watch批次数错误",
          );
          await keys(page.next_cursor, CURSOR, "cursor");
          if (page.gap !== null) {
            const gap = page.gap;
            await keys(
              gap,
              [
                "requested_identity",
                "reason",
                "missing_range",
                "rebuild",
                "snapshot_token",
              ],
              "Gap",
            );
            need(
              (await equal(gap.requested_identity, before.cursor)) &&
                (await equal(page.next_cursor, before.cursor)) &&
                page.deltas.length === 0 &&
                !page.has_more,
              "Gap提前推进cursor",
            );
            need(
              await equal(gap.rebuild, page.observed_head),
              "Gap rebuild/observed不同",
            );
            const changed = !sameIdentity(before.cursor, gap.rebuild);
            if (gap.reason === "session_identity_changed")
              need(changed && gap.missing_range === null, "会话Gap范围不同");
            else {
              need(
                !changed &&
                  [
                    "delivery_retention_gap",
                    "client_backlog_overflow",
                  ].includes(gap.reason),
                "Gap reason错误",
              );
              await keys(
                gap.missing_range,
                ["from_generation", "to_generation"],
                "missing_range",
              );
              const to =
                gap.reason === "delivery_retention_gap"
                  ? BigInt(page.delivery_frontier.first_available_generation) -
                    1n
                  : BigInt(gap.rebuild.cut_generation);
              need(
                gap.missing_range.from_generation ===
                  (BigInt(before.cursor.after_generation) + 1n).toString() &&
                  gap.missing_range.to_generation === to.toString() &&
                  BigInt(gap.missing_range.from_generation) <= to,
                "Gap缺口不同",
              );
            }
            return this.commit(
              await this.collect(
                {
                  identity: smallIdentity(gap.rebuild),
                  mode: gap.rebuild.history_mode,
                  generation: gap.rebuild.cut_generation,
                  token: gap.snapshot_token,
                  expectedFixed: gap.rebuild,
                },
                b,
              ),
              b,
              { kind: "gap", reason: gap.reason },
            );
          }
          need(sameIdentity(reply, before.cursor), "无Gap不准换会话");
          let candidate = before;
          for (const wire of page.deltas) {
            await keys(wire, [...D, "delta"], "Delta wire");
            content.validateRef(wire.delta);
            content.validateRef(wire.catalog_evidence);
            need(
              wire.generation ===
                (BigInt(candidate.cursor.after_generation) + 1n).toString() &&
                wire.base_cut === candidate.cursor.base_cut &&
                wire.session_id === before.cursor.session_id &&
                wire.catalog_revision === before.cursor.catalog_revision &&
                wire.seq_range.from ===
                  (BigInt(candidate.cursor.last_seq) + 1n).toString(),
              "Delta断代/会话/cut/前沿",
            );
            const anchor = {
                kind: "delta",
                ...smallIdentity(before.cursor),
                generation: wire.generation,
                base_cut: wire.base_cut,
                next_cut: wire.next_cut,
                index_frontier: wire.index_frontier,
                delta_ref: wire.delta,
              },
              reader = this.reader(anchor, before.cursor, b),
              inner = reader.value(wire.delta);
            await reader.complete(inner);
            await reader.finish();
            const outer = {};
            for (const key of D)
              outer[key] =
                key === "catalog_evidence"
                  ? reader.value(wire.catalog_evidence, [
                      { field: "catalog_evidence" },
                    ])
                  : wire[key];
            outer.delta = inner;
            const delta = of(outer),
              applied = await this.applyBatch(candidate, delta, b);
            candidate = await this.collect(
              {
                identity: before.cursor,
                mode: "RecomputedWithRevision",
                generation: wire.generation,
              },
              b,
            );
            await comparePublication(
              delta,
              await candidate.state.field("snapshot"),
            );
            if (this.hooks.comparePublication)
              await this.hooks.comparePublication(
                delta,
                await candidate.state.field("snapshot"),
              );
            for (const family of FAMILIES) {
              need(
                applied[family].length === candidate.families[family].length,
                "Delta/Snapshot六族数量不同",
              );
              for (let index = 0; index < applied[family].length; index++) {
                b.job.chargeRecords(2n);
                this.check(b);
                need(
                  await equal(
                    applied[family][index],
                    candidate.families[family][index],
                  ),
                  "Delta/Snapshot全部字段/原顺序不同：" + family,
                );
              }
            }
          }
          if (page.deltas.length === 0) {
            candidate = await this.collect(
              {
                identity: before.cursor,
                mode: before.fixed.history_mode,
                generation: before.cursor.after_generation,
              },
              b,
            );
            for (const family of FAMILIES)
              b.job.chargeRecords(
                BigInt(
                  before.families[family].length +
                    candidate.families[family].length,
                ),
              );
            need(
              await equal(candidate.projection, before.projection),
              "空Delta完整projection改变",
            );
          }
          need(
            await equal(page.next_cursor, candidate.cursor),
            "next_cursor不是末批完整身份",
          );
          const head = BigInt(page.observed_head.cut_generation),
            end = BigInt(candidate.cursor.after_generation);
          need(
            head >= end &&
              page.has_more === head > end &&
              (page.deltas.length > 0 || head === end),
            "Watch head/has_more/空批不同",
          );
          if (head === end)
            need(
              await equal(page.observed_head, {
                ...candidate.fixed,
                history_mode: "RecomputedWithRevision",
                as_of_generation: null,
              }),
              "Watch head不是同cut原完整身份",
            );
          return this.commit(candidate, b, {
            kind: "watch",
            batches: page.deltas.length,
            hasMore: page.has_more,
          });
        });
      }
    }
    return Object.freeze({
      Client,
      validateState: validateStateOnce,
      validateTB02Catalog: (catalog) => catalogValidate(catalog, null),
      validateDelta,
      comparePublication,
      rowKey,
      tuple,
      axesValidate,
      rawValidate,
      bindSources,
      auditWitnesses,
      completeValue,
      typed,
      V,
    });
  };
});
