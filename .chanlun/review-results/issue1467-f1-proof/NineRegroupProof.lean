import NineRegroupSpec

namespace NineRegroup

open NewChanlun.Origin
open CenterAttempt (ObservedUnit)
open UpgradeResearch (Interval)

theorem gaps : GAP_TARGET := by
  intro α xs pre a gap₁ b gap₂ c post hn hc ha hb hd
  have he := congrArg List.length hc
  simp only [List.length_append] at he
  rw [hn] at he
  exact ⟨List.eq_nil_of_length_eq_zero (by omega), List.eq_nil_of_length_eq_zero (by omega),
    List.eq_nil_of_length_eq_zero (by omega), List.eq_nil_of_length_eq_zero (by omega)⟩

theorem slices : SLICE_TARGET := by
  intro α xs a b c hn h
  rcases h with ⟨pre,gap₁,gap₂,post,hcover,ha,hb,hc⟩
  rcases gaps α xs pre a gap₁ b gap₂ c post hn hcover ha hb hc with ⟨hp,hg₁,hg₂,hq⟩
  subst pre; subst gap₁; subst gap₂; subst post
  simp only [List.nil_append,List.append_nil] at hcover
  have he := congrArg List.length hcover
  simp only [List.length_append] at he
  rw [hn] at he
  have ha3 : a.length = 3 := by omega
  have hb3 : b.length = 3 := by omega
  have hc3 : c.length = 3 := by omega
  have first : xs.take 3 = a := by
    rw [← hcover]
    simpa only [List.append_assoc] using (List.take_left' (l₁:=a) (l₂:=b++c) ha3)
  have drop : xs.drop 3 = b ++ c := by
    rw [← hcover]
    simpa only [List.append_assoc] using (List.drop_left' (l₁:=a) (l₂:=b++c) ha3)
  have second : (xs.drop 3).take 3 = b := by
    rw [drop]
    exact List.take_left' hb3
  have third : xs.drop 6 = c := by
    rw [← hcover]
    have hab : (a++b).length = 6 := by simp [ha3,hb3]
    simpa only [List.append_assoc] using (List.drop_left' (l₁:=a++b) (l₂:=c) hab)
  exact ⟨first.symm,second.symm,third.symm,hcover,ha3,hb3,hc3⟩

theorem nonempty : NONEMPTY_TARGET := by
  refine ⟨?_,by decide,by decide,by decide⟩
  exact ⟨[],[],[],[],rfl,by decide,by decide,by decide⟩

theorem no_regroup : NO_REGROUP_TARGET := by
  rintro ⟨a,b,c,hsource,sa,sb,sc,ca,cb,cc,hcore⟩
  rcases slices ObservedUnit input a b c (by rfl) hsource with ⟨ha,hb,hc,_⟩
  rw [ha] at ca
  rw [hb] at cb
  rw [hc] at cc
  change bounds first = some ⟨segLow sa,segHigh sa⟩ at ca
  change bounds second = some ⟨segLow sb,segHigh sb⟩ at cb
  change bounds third = some ⟨segLow sc,segHigh sc⟩ at cc
  have ea : (⟨10040,10060⟩ : Interval) = ⟨segLow sa,segHigh sa⟩ :=
    Option.some.inj (nonempty.2.1.symm.trans ca)
  have eb : (⟨10030,10055⟩ : Interval) = ⟨segLow sb,segHigh sb⟩ :=
    Option.some.inj (nonempty.2.2.1.symm.trans cb)
  have ec : (⟨10030,10040⟩ : Interval) = ⟨segLow sc,segHigh sc⟩ :=
    Option.some.inj (nonempty.2.2.2.symm.trans cc)
  exact LiftBoundary.origin_rejection sa sb sc
    (congrArg Interval.lo ea).symm (congrArg Interval.hi ea).symm
    (congrArg Interval.lo eb).symm (congrArg Interval.hi eb).symm
    (congrArg Interval.lo ec).symm (congrArg Interval.hi ec).symm hcore

theorem exact_root : GAP_TARGET ∧ SLICE_TARGET ∧ NONEMPTY_TARGET ∧
    LiftBoundary.WITNESS_TARGET ∧ NO_REGROUP_TARGET :=
  ⟨gaps,slices,nonempty,LiftBoundary.witness,no_regroup⟩

end NineRegroup

#print axioms NineRegroup.exact_root
