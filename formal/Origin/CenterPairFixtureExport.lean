/- #1419 / #1424：五叶判据的 Lean → Rust 导出入口。
8 例输入逐字承接固定 donor 634f62cddfaddf656a35ca9b9db20cb716b62bff 的本文件，
对应当前 rust/tests/fixtures/center_pair_1424_lean.json；本轮只静态核对，未运行 Lean 导出。
donor 注释所称 oracle_pairs.py 在当前树与 donor 均未找到，不能据此声称独立生成已复核。
标签与旧三态投影在执行本入口时由下列函数求值；实际来源/待验项见 #1419 lean-v4 报告。
有效域 DD≤ZD≤ZG≤GG；这 8 例还满足 S 的 ZD<ZG。无 16 格矩阵导出，
不证明扫描构造/≥9 窗口/正式会话验收；四态投影定理见 CenterStates。
后续获准远端验收时的入口：lake env lean Origin/CenterPairFixtureExport.lean
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
