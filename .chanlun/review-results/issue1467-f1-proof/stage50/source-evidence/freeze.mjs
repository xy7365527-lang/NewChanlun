import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const E=path.dirname(fileURLToPath(import.meta.url));
const R=path.resolve(E,'../../../../..');
const P='.chanlun/review-results/issue1467-f1-proof';
const sum=p=>{const b=fs.readFileSync(path.join(R,p));return {path:p,bytes:b.length,sha256:crypto.createHash('sha256').update(b).digest('hex')};};
const inputs=[
  `${P}/GoalReframe-v4.md`,
  `${P}/Stage32ExitLiftReadiness.md`,`${P}/Stage34BaseObjectAudit.md`,
  `${P}/Stage39ScopeAudit.md`,`${P}/Stage40CompletionGate.md`,
  `${P}/Stage49FullRangeCoreIdentityAndRestart.md`,
  `${P}/stage49/ScopeClarifications-v1.md`,`${P}/stage49/final-scope-review.md`,
  `${P}/stage49/base-core-identity.md`,`${P}/stage49/f2-input-contract.md`,
  `${P}/stage49/independent-f2-contract-review.md`,`${P}/stage49/independent-rc49-review.md`,
  `${P}/stage49/full-base-candidate.md`,`${P}/stage49/candidate-evidence/rc49-v1-definition.md`,
  `${P}/stage49/candidate-evidence/rc49-v1.mjs`,
  `${P}/stage49/candidate-evidence/results-v1/same_up_counterexample.json`,
  '.agents/research-tools/xsoc1-9e0da0c3/plugins/rigorous-open-math-research/skills/rigorous-open-math-research/SKILL.md',
  '.agents/research-tools/xsoc1-9e0da0c3/plugins/math-research-workflow/skills/math-research-workflow/SKILL.md',
  ...JSON.parse(fs.readFileSync(path.join(E,'source-readback.json'),'utf8')).map(x=>x.path)
];
const artifacts=[`${P}/stage50/expansion-handoff-contract.md`,
  ...['check-source-and-rc49.mjs','freeze.mjs','source-readback.json','rc49-handoff-facts.json','check-results.json'].map(x=>`${P}/stage50/source-evidence/${x}`)];
const manifest={version:'stage50-expansion-handoff-v1',date:'2026-10-05',
  researchRoot:R,expectedHead:'d9ad8c54e969e1258cde1134ba3ec6d6db800e44',
  scope:'Author source audit and ordinary conditional proofs. Independent review pending. Manifest excludes itself.',
  inputs:[...new Set(inputs)].map(sum),artifacts:artifacts.map(sum)};
fs.writeFileSync(path.join(E,'frozen-manifest.json'),`${JSON.stringify(manifest,null,2)}\n`);
console.log(JSON.stringify({inputCount:manifest.inputs.length,artifactCount:manifest.artifacts.length,document:manifest.artifacts[0]}));
