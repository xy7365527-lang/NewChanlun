import fs from 'node:fs';import path from 'node:path';import {fileURLToPath}from'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),R=path.resolve(here,'../../../../..');
const [run,project]=process.argv.slice(2);if(!run||!project)throw Error('run directory and new Lean project directory required');
const p=JSON.parse(fs.readFileSync(path.join(run,'T60-main.source.json'))),n=JSON.parse(fs.readFileSync(path.join(run,'N60-equal-force.source.json')));
const pr=JSON.parse(fs.readFileSync(path.join(run,'T60-main.reference.stdout'))),nr=JSON.parse(fs.readFileSync(path.join(run,'N60-equal-force.reference.stdout')));
const show=s=>`⟨.${s.up?'up':'down'},${s.start},${s.end},${s.start_price},${s.end_price}⟩`;
let f=`import Origin.SegmentAutoConstruct\nimport Origin.SegmentFeatureComplete\nimport Origin.CenterComplete\nimport Origin.ForceVelocity\nnamespace Stage60\nopen NewChanlun.Origin\ninstance : Inhabited Stroke := ⟨⟨.up,0,0,0,0⟩⟩\ninstance : Inhabited FeatureElem := ⟨FeatureElem.ofStroke (default : Stroke)⟩\n`;
for(const [name,x,r]of [['positive',p,pr],['negative',n,nr]]){
 f+=`def ${name}Prices : List Int := [${x.observations.map(o=>o.price).join(',')}]\ndef ${name} : List Stroke := [${r.strokes.map(show).join(',\n')}]\n`;
 for(let s=0;s<5;s++){
  const t=5*s,up=s%2===0;
  f+=`def ${name}Input${s} := (${name}.drop ${t}).take 8\n`;
  f+=`def ${name}D${s} : SegEndData := ⟨.${up?'up':'down'},FeatureElem.ofStroke (${name}[${t+3}]!),FeatureElem.ofStroke (${name}[${t+5}]!),FeatureElem.ofStroke (${name}[${t+7}]!),[],${x.observations[1+20*s].price},${x.observations[21+20*s].price}⟩\n`;
 }
}
f+=`def b := positive.take 5\ndef c := (positive.drop 20).take 5\ndef cn := (negative.drop 20).take 5\n`;
f+=`def s1 : Segment := ⟨.down,21,41,16000,12000⟩\ndef s2 : Segment := ⟨.up,41,61,12000,15000⟩\ndef s3 : Segment := ⟨.down,61,81,15000,12500⟩\n`;
f+=`def SelectedBy (d : SegEndData) (ss : List Stroke) : Prop :=
 match extractSegEndData d.dir d.startPrice d.endPrice ss with
 | none => False
 | some a => a.dir = d.dir ∧ a.startPrice = d.startPrice ∧ a.endPrice = d.endPrice ∧
     a.e1.low = d.e1.low ∧ a.e1.high = d.e1.high ∧ a.e2.low = d.e2.low ∧ a.e2.high = d.e2.high ∧ a.e3.low = d.e3.low ∧ a.e3.high = d.e3.high

def NoContainment (d : Direction) (ss : List Stroke) : Prop :=
 let es := (extractFeatureStrokes d ss).map FeatureElem.ofStroke
 ∀ i : Fin (es.length-1),
  let a := es[i.val]!
  let b := es[i.val+1]!
  ¬ Contains a b ∧ ¬ Contains b a

def StrokeSource (ps : List Int) (s : Stroke) : Prop :=
 s.startIndex > 0 ∧ s.endIndex + 1 < ps.length ∧ s.endIndex = s.startIndex + 4 ∧ s.WellFormed ∧
 s.startPrice = ps[s.startIndex]! ∧ s.endPrice = ps[s.endIndex]! ∧
 (match s.direction with
 | .up => ps[s.startIndex-1]! > s.startPrice ∧ ps[s.startIndex+1]! > s.startPrice ∧ ps[s.endIndex-1]! < s.endPrice ∧ ps[s.endIndex+1]! < s.endPrice ∧ s.startPrice < s.endPrice
 | .down => ps[s.startIndex-1]! < s.startPrice ∧ ps[s.startIndex+1]! < s.startPrice ∧ ps[s.endIndex-1]! > s.endPrice ∧ ps[s.endIndex+1]! > s.endPrice ∧ s.endPrice < s.startPrice)

def ATOMS : Prop := positive.length = 30 ∧ negative.length = 30 ∧ positivePrices.length = 123 ∧ negativePrices.length = 123 ∧
 (∀ i : Fin 30, StrokeSource positivePrices positive[i.val]!) ∧
 (∀ i : Fin 30, StrokeSource negativePrices negative[i.val]!)
`;
const cs=[];for(const name of ['positive','negative'])for(let s=0;s<5;s++)cs.push(`(SegEndComplete ${name}D${s} ∧ ¬ HasGap ${name}D${s}.e1 ${name}D${s}.e2 ∧ SelectedBy ${name}D${s} ${name}Input${s} ∧ NoContainment .${s%2===0?'up':'down'} ${name}Input${s})`);
f+=`def FEATURES : Prop := ${cs.join(' ∧\n')}\n`;
f+=`def CORE : Prop := CenterConfirmedComplete s1 s2 s3 ∧ computeZD s1 s2 s3 = 12500 ∧ computeZG s1 s2 s3 = 15000\n`;
f+=`def FORCE : Prop := impulse b = 250 ∧ impulse c = -125 ∧ impulse cn = 250 ∧ IsImpulseDivergence b c ∧ ¬ IsImpulseDivergence b cn\nend Stage60\n`;
let proof=`import Stage60Fixture\nset_option maxRecDepth 100000\nset_option maxHeartbeats 10000000\nnamespace Stage60\nopen NewChanlun.Origin\n
instance (ps : List Int) (s : Stroke) : Decidable (StrokeSource ps s) := by
 unfold StrokeSource Stroke.WellFormed
 cases s.direction <;> dsimp <;> infer_instance
instance (d : SegEndData) (ss : List Stroke) : Decidable (SelectedBy d ss) := by
 unfold SelectedBy
 cases extractSegEndData d.dir d.startPrice d.endPrice ss <;> infer_instance
instance (d : Direction) (ss : List Stroke) : Decidable (NoContainment d ss) := by
 unfold NoContainment
 dsimp
 infer_instance

theorem atoms_checked : ATOMS := by
 unfold ATOMS
 decide +kernel

theorem features_checked : FEATURES := by
`;
const hs=[];
for(const name of ['positive','negative'])for(let s=0;s<5;s++){
 const h=`h_${name}_${s}`;hs.push(h);
 proof+=` have ${h} : SegEndComplete ${name}D${s} := by\n  unfold SegEndComplete TopAboveBottom ${name}D${s}\n  dsimp\n  unfold ${s%2===0?'IsTopFractal SegmentEndUp':'IsBottomFractal SegmentEndDown'} FractalInSeq\n  decide +kernel\n`;
}
proof+=` unfold FEATURES\n simp only [${hs.join(',')}, true_and]\n decide +kernel\n`;
proof+=`theorem core_checked : CORE := by\n unfold CORE CenterConfirmedComplete DirAlternates\n decide +kernel\ntheorem force_checked : FORCE := by\n unfold FORCE\n decide +kernel\ntheorem exact_root : ATOMS ∧ FEATURES ∧ CORE ∧ FORCE := ⟨atoms_checked, features_checked, core_checked, force_checked⟩\nend Stage60\n`;
fs.mkdirSync(path.join(project,'Origin'),{recursive:true});
for(const [name,text]of [['Stage60Fixture.lean',f],['Stage60Proof.lean',proof]])fs.writeFileSync(path.join(project,name),text);
// Compile the newly consumed force module from its current source, not from the old Stage41 review cache.
const copied=new Set();
function copyModule(m){
 if(copied.has(m))return;copied.add(m);
 const relative=m.replaceAll('.','/')+'.lean',src=path.join(R,'formal',relative);
 if(!fs.existsSync(src))throw Error(`missing local source ${src}`);
 const text=fs.readFileSync(src,'utf8'),dst=path.join(project,relative);fs.mkdirSync(path.dirname(dst),{recursive:true});fs.writeFileSync(dst,text);
 for(const match of text.matchAll(/^import\s+(Origin\.[A-Za-z0-9_.]+)/gm))copyModule(match[1]);
}
for(const m of ['Origin.SegmentAutoConstruct','Origin.SegmentFeatureComplete','Origin.CenterComplete','Origin.ForceVelocity'])copyModule(m);
