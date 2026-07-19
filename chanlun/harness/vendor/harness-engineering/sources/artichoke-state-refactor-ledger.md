# Artichoke State Refactor Evidence Ledger

The cross-reference timeline for [Destructure State (#442)] contains exactly 50
causally linked pull requests that merged before [the second integration attempt
(#661)] opened. Their descriptions or comments identify them as extracted from,
inspired by, related to, or required for #442.

The headings group pull requests by the date they opened or first
cross-referenced #442. A merge may have completed later. Ryan Lopopolo authored
all 50.

[Destructure State (#442)]: https://github.com/artichoke/artichoke/pull/442
[the second integration attempt (#661)]:
  https://github.com/artichoke/artichoke/pull/661

## January 24–26

- [converter syntax (#444)], [converter inference (#447)], [non-null class and
  module lookup (#448)], [exception-handler safety (#449)], [implicit integer
  conversion (#450)], and [hash converter allocation (#451)];
- [non-null VM pointers (#453)], [converter dependency cleanup (#455)],
  [semantic error handling (#456)], [safer exception raising (#457)], and
  [unused array backend removal (#458)]; and
- [nil coercion at the API edge (#460)], [`to_ary` error propagation (#461)],
  [Array initialization repair (#462)], [warning error propagation (#463)], [ENV
  rewrite (#464)], [Ruby exception cleanup (#465)], and [boot conversion cleanup
  (#466)].

[converter syntax (#444)]: https://github.com/artichoke/artichoke/pull/444
[converter inference (#447)]: https://github.com/artichoke/artichoke/pull/447
[non-null class and module lookup (#448)]:
  https://github.com/artichoke/artichoke/pull/448
[exception-handler safety (#449)]:
  https://github.com/artichoke/artichoke/pull/449
[implicit integer conversion (#450)]:
  https://github.com/artichoke/artichoke/pull/450
[hash converter allocation (#451)]:
  https://github.com/artichoke/artichoke/pull/451
[non-null VM pointers (#453)]: https://github.com/artichoke/artichoke/pull/453
[converter dependency cleanup (#455)]:
  https://github.com/artichoke/artichoke/pull/455
[semantic error handling (#456)]:
  https://github.com/artichoke/artichoke/pull/456
[safer exception raising (#457)]:
  https://github.com/artichoke/artichoke/pull/457
[unused array backend removal (#458)]:
  https://github.com/artichoke/artichoke/pull/458
[nil coercion at the API edge (#460)]:
  https://github.com/artichoke/artichoke/pull/460
[`to_ary` error propagation (#461)]:
  https://github.com/artichoke/artichoke/pull/461
[Array initialization repair (#462)]:
  https://github.com/artichoke/artichoke/pull/462
[warning error propagation (#463)]:
  https://github.com/artichoke/artichoke/pull/463
[ENV rewrite (#464)]: https://github.com/artichoke/artichoke/pull/464
[Ruby exception cleanup (#465)]: https://github.com/artichoke/artichoke/pull/465
[boot conversion cleanup (#466)]:
  https://github.com/artichoke/artichoke/pull/466

## January 27–February 23

- [parser state and APIs (#467)], [PRNG state (#469)], [protected FFI calls
  (#470)], [warnings on stderr (#471)], and [byte-preserving warnings (#472)];
- [mutable evaluation (#480)], [the Intern capability (#481)], [removal of the
  generated converter matrix (#487)], and [allocating conversion traits (#488)];
  and
- [Regexp cleanup (#491)], [virtual-filesystem rearchitecture (#497)],
  [constant-definition capabilities (#526)], and [output state (#539)].

[parser state and APIs (#467)]: https://github.com/artichoke/artichoke/pull/467
[PRNG state (#469)]: https://github.com/artichoke/artichoke/pull/469
[protected FFI calls (#470)]: https://github.com/artichoke/artichoke/pull/470
[warnings on stderr (#471)]: https://github.com/artichoke/artichoke/pull/471
[byte-preserving warnings (#472)]:
  https://github.com/artichoke/artichoke/pull/472
[mutable evaluation (#480)]: https://github.com/artichoke/artichoke/pull/480
[the Intern capability (#481)]: https://github.com/artichoke/artichoke/pull/481
[removal of the generated converter matrix (#487)]:
  https://github.com/artichoke/artichoke/pull/487
[allocating conversion traits (#488)]:
  https://github.com/artichoke/artichoke/pull/488
[Regexp cleanup (#491)]: https://github.com/artichoke/artichoke/pull/491
[virtual-filesystem rearchitecture (#497)]:
  https://github.com/artichoke/artichoke/pull/497
[constant-definition capabilities (#526)]:
  https://github.com/artichoke/artichoke/pull/526
[output state (#539)]: https://github.com/artichoke/artichoke/pull/539

## March 4–May 3

- [Regexp global preparation (#557)], [global-variable capabilities (#562)],
  [UTF-8 Regexp migration (#566)], [Onig Regexp migration (#571)], and [Regexp
  pattern lifetime cleanup (#613)];
- [MatchData rewrite (#618)], [interpreter-aware implicit conversions (#621)],
  [interpreter-aware Value methods (#622)], [Kernel rewrite (#623)], [core Value
  mutability (#630)], and [removing the interpreter from Value (#631)]; and
- [safe Range detection (#635)], [Regexp state extraction (#643)], [mutable
  Rust-backed values (#644)], [path-based source loading (#652)], [the VFS
  capability boundary (#655)], [the IO capability (#657)], [the Regexp
  capability boundary (#658)], and [the PRNG capability boundary (#659)].

[Regexp global preparation (#557)]:
  https://github.com/artichoke/artichoke/pull/557
[global-variable capabilities (#562)]:
  https://github.com/artichoke/artichoke/pull/562
[UTF-8 Regexp migration (#566)]: https://github.com/artichoke/artichoke/pull/566
[Onig Regexp migration (#571)]: https://github.com/artichoke/artichoke/pull/571
[Regexp pattern lifetime cleanup (#613)]:
  https://github.com/artichoke/artichoke/pull/613
[MatchData rewrite (#618)]: https://github.com/artichoke/artichoke/pull/618
[interpreter-aware implicit conversions (#621)]:
  https://github.com/artichoke/artichoke/pull/621
[interpreter-aware Value methods (#622)]:
  https://github.com/artichoke/artichoke/pull/622
[Kernel rewrite (#623)]: https://github.com/artichoke/artichoke/pull/623
[core Value mutability (#630)]: https://github.com/artichoke/artichoke/pull/630
[removing the interpreter from Value (#631)]:
  https://github.com/artichoke/artichoke/pull/631
[safe Range detection (#635)]: https://github.com/artichoke/artichoke/pull/635
[Regexp state extraction (#643)]:
  https://github.com/artichoke/artichoke/pull/643
[mutable Rust-backed values (#644)]:
  https://github.com/artichoke/artichoke/pull/644
[path-based source loading (#652)]:
  https://github.com/artichoke/artichoke/pull/652
[the VFS capability boundary (#655)]:
  https://github.com/artichoke/artichoke/pull/655
[the IO capability (#657)]: https://github.com/artichoke/artichoke/pull/657
[the Regexp capability boundary (#658)]:
  https://github.com/artichoke/artichoke/pull/658
[the PRNG capability boundary (#659)]:
  https://github.com/artichoke/artichoke/pull/659
