/- #1424：五叶判据的 Lean → Rust 机器导出夹具。
输入来自独立工作例 oracle_pairs.py 的 W2/W3/W4/W5/W6/W11；标签与投影由 Lean 真求值。
只验证 DD≤ZD≤ZG≤GG 域内代数对拍；不证明扫描构造/≥9 窗口/正式会话验收。
运行：lake env lean Origin/CenterPairFixtureExport.lean
-/
import Origin.CenterStates
import Lean.Data.Json

open Lean (Json)
open NewChanlun.Origin

namespace NewChanlun.Origin.CenterPairFixtureExport

def mkCenter (dd zd zg gg : Tick) (hlo : dd ≤ zd) (hcore : zd ≤ zg) (hhi : zg ≤ gg) : CenterWithOuter :=
  { core := { zd, zg, startIndex := 0, endIndex := 0, valid := hcore }
    dd, gg, outer_lo := hlo, outer_hi := hhi }

def leafName : PairLeaf → String
  | .coreNotStrictlySeparated => "CORE_NOT_STRICTLY_SEPARATED"
  | .upNewbirth => "UP_NEWBIRTH"
  | .downNewbirth => "DOWN_NEWBIRTH"
  | .upExpansion => "UP_EXPANSION"
  | .downExpansion => "DOWN_EXPANSION"

def developmentName : CenterDevelopment → String
  | .extension => "extension"
  | .newBirth => "newBirth"
  | .expansion => "expansion"

def side (c : CenterWithOuter) : Json :=
  Json.mkObj [("DD", Json.num c.dd), ("ZD", Json.num c.core.zd),
    ("ZG", Json.num c.core.zg), ("GG", Json.num c.gg)]

def row (name : String) (a b : CenterWithOuter) : Json :=
  Json.mkObj [("name", Json.str name), ("A", side a), ("B", side b),
    ("leaf", Json.str (leafName (classifyPairLeaf a b))),
    ("development", Json.str (developmentName (classifyDevelopment a b)))]

def fixture : Json := Json.mkObj [
  ("schema", Json.str ("tb1424-lean-pair/1" : String)),
  ("source", Json.str ("Origin/CenterPairFixtureExport.lean" : String)),
  ("cases", Json.arr #[
    row "W2_UP_EXPANSION" (mkCenter 5 10 25 35 (by decide) (by decide) (by decide)) (mkCenter 30 33 41 43 (by decide) (by decide) (by decide)),
    row "W3_DISC_DOWN_NEWBIRTH" (mkCenter 10 20 35 40 (by decide) (by decide) (by decide)) (mkCenter 2 3 8 9 (by decide) (by decide) (by decide)),
    row "W3_FULL_UNIT_COUNTERFACTUAL" (mkCenter 5 20 35 40 (by decide) (by decide) (by decide)) (mkCenter 2 3 8 9 (by decide) (by decide) (by decide)),
    row "W4_DISC_UP_NEWBIRTH" (mkCenter 5 10 25 35 (by decide) (by decide) (by decide)) (mkCenter 36 37 42 43 (by decide) (by decide) (by decide)),
    row "W5_OUTER_TOUCH" (mkCenter 10 20 35 40 (by decide) (by decide) (by decide)) (mkCenter 2 3 8 10 (by decide) (by decide) (by decide)),
    row "W6_CORE_TANGENT" (mkCenter 10 20 35 50 (by decide) (by decide) (by decide)) (mkCenter 30 35 40 60 (by decide) (by decide) (by decide)),
    row "W6_REVERSE_TANGENT" (mkCenter 30 35 40 60 (by decide) (by decide) (by decide)) (mkCenter 10 20 35 50 (by decide) (by decide) (by decide)),
    row "W11_CORE_OVERLAP" (mkCenter 10 20 35 50 (by decide) (by decide) (by decide)) (mkCenter 30 33 40 60 (by decide) (by decide) (by decide))
  ])]

end NewChanlun.Origin.CenterPairFixtureExport

#eval IO.println NewChanlun.Origin.CenterPairFixtureExport.fixture.compress
