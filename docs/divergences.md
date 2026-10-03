# Deliberate differences from eyecite

citare follows [eyecite](https://github.com/freelawproject/eyecite) 2.7.8: it passes eyecite's
own tests through [citation-bench](https://github.com/codexpublicus/citation-bench), and a
difference from eyecite is a bug unless it's listed here. Each entry below is a place where
citare gives a different answer on purpose, because the evidence from real opinions says
eyecite's answer is wrong. An entry has:

- the input, and what eyecite and citare each return for it;
- the evidence: how often the case comes up in real opinions, and which engine gets it right;
- the tests in this repository and the citation-bench vectors that hold citare to it
  (`vectors/divergences/`, scored on their own so eyecite's score on its own tests stays
  100%);
- the eyecite issue that reports it upstream, or a draft of one in
  [`eyecite-issue-drafts/`](eyecite-issue-drafts/).

A new divergence goes in this file before it's merged, with all four.

| Entry | Since | eyecite issue |
|---|---|---|
| [`§` pin cites on `Id.` and `Ibid.`](#id-section-pin) | #41 | [freelawproject/eyecite#299](https://github.com/freelawproject/eyecite/issues/299); follow-up drafted in [`id-section-pin.md`](eyecite-issue-drafts/id-section-pin.md) |

<a id="id-section-pin"></a>
## `§` pin cites on `Id.` and `Ibid.`

`Id. § 5` cites section 5 of the authority cited before it. citare keeps the section as the
id. citation's pin cite. eyecite's tokenizer turns the `§` into a section token that its pin
cite scan stops at, so eyecite returns a bare `Id.` and a separate unknown citation for the `§`.
Three differences in resolution follow from that.

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `See 42 U.S.C. § 1983. Id. § 1988.` | `Id.`, no pin cite, resolves to the statute; unknown citation `§` | `Id. § 1988`, pin cite `§ 1988`, resolves to the statute |
| `Foo v. Bar, 1 U.S. 1 (1999). Id. § 5.` | `Id.`, no pin cite, resolves to the case; unknown citation `§` | `Id. § 5`, pin cite `§ 5`, **unresolved** |
| `See 83 Stat. 841. Id. § 4(b).` | `Id.`, no pin cite, resolves to the statute; unknown citation `§` | `Id. § 4(b)`, pin cite `§ 4(b)`, resolves to the statute |
| `See 42 U.S.C. § 1983. Id. § 1988. Id. at § 1983(b).` | the second `Id.` is **unresolved**: the unknown `§` before it resets eyecite's id. chain | both id. citations resolve to the statute |

The same holds for `Id. §1988`, `Id. §§ 1983-1985`, `Id. at § 1983(a)`, `Id., § 1988`,
`Ibid. § 1983(c)`, `Id. § 1983 et seq.`, `Id. § 1983(a), (b)` and `Id. § 1983 (explaining)`
(the parenthetical stays a parenthetical).

**After a case.** A pin cite that doesn't start with a page number can't be checked against a
case's pages, and eyecite's `_has_invalid_pin_cite` treats it as invalid: `Id. ¶ 4` after
`1 U.S. 1` is unresolved in both engines. citare applies the same rule to a section pin cite.
eyecite resolves `Id. § 5` to the case only because it never sees the pin cite.

**After a paged law citation.** Statutes at Large and session laws have a page, so the same
rule would leave `Id. § 4(b)` after `83 Stat. 841` unresolved. A section is a valid pin cite
into an act, so citare checks section pin cites against pages only when the antecedent is a
case. A `¶` pin cite after `83 Stat. 841` is still unresolved, as in eyecite.

**The id. chain.** eyecite resolves each id. citation to whatever the citation before it
resolved to, and its unknown `§` citation resolves to nothing. Every `Id. § N` therefore cuts
the chain, and the next `Id.` is unresolved. In citare the section is part of the id.
citation, so the chain continues.

Unchanged, and the same as eyecite: `Id. at 5`, `Id. ¶ 4`, `Id. at ¶ 4`, a bare `Id.`, a `§`
that doesn't directly follow the id. citation (`Id. The rule in § 5 applies.`), and `Id.`
at the end of a line with the `§` on the next one. Neither engine finds an `Id.` that directly
follows an opening parenthesis or bracket (`(Id. § 2441)`); that is a separate gap.

### Evidence

I searched two samples of real opinions for `Id.` or `Ibid.` followed by `§` or `§§`, allowing
a comma or `at` between them, and ran both engines over each whole opinion: eyecite 2.7.8 with
the data versions citation-bench pins, and citare at this branch.

- **CAP**: one volume (the middle one by number) from each of the 401 reporters in the Harvard
  Caselaw Access Project, 70,896 cases. 1,015 matches in 448 opinions.
- **CourtListener**: the two samples eyecite's own benchmark runs on (citation-bench's
  `courtlistener-1pct` and `courtlistener-10pct`, 8,411 opinions, deduplicated by id),
  prepared as eyecite's benchmark prepares them. 505 matches in 169 opinions.

In 230 of the CAP matches neither engine extracts an id. citation at all, 187 of them because
the `Id.` follows `(`. Both engines are identical there, so they're left out below. Every other
match was labeled by reading the opinion: whether the `§` is the id. citation's pin cite, what
the id. citation actually refers to, and whether each engine resolved it to that. I checked
100 of them by hand: every match where eyecite resolves to a case and citare doesn't, every
match after a paged law citation, every match that isn't a pin cite or whose pin cite citare
misses in CAP, and random samples of the rest. The labels agree with mine on whether the `§`
is a pin cite in all 100, and on resolution in 91 for eyecite and 93 for citare; mine are the
ones counted. The table, with a snippet of each opinion, is in
[`evidence/id-section-pin.md`](evidence/id-section-pin.md).

| | CAP | CourtListener |
|---|---|---|
| Matches | 1,015 in 448 opinions | 505 in 169 opinions |
| Neither engine extracts an id. citation | 230 | 9 |
| Labeled | 785 | 496 |
| The `§` is the id. citation's pin cite | 772 | 496 |
| Span and pin cite right, eyecite | 13 (1.7%) | 0 |
| Span and pin cite right, citare | 684 (87.1%) | 429 (86.5%) |
| The same, counting a pin cite that stops early as right | 775 (98.7%) | 455 (91.7%) |
| Resolution right, eyecite | 684 (87.1%) | 407 (82.1%) |
| Resolution right, citare | 766 (97.6%) | 475 (95.8%) |
| Resolved differently, only citare right | 92 | 76 |
| Resolved differently, only eyecite right | 10 | 8 |

eyecite's span is right only where the `§` isn't the id. citation's pin cite. A resolution
is right when the engine links the id. citation to the citation of what it actually refers to
(for a statute, the same act or code), or leaves it unresolved when that authority was never
extracted as a citation. The labels judge "never extracted" from what the two engines resolve
to, so an unresolved answer next to an antecedent that was extracted further back can count as
right; the hand check found 4 of those in its 15 both-unresolved CAP rows, and they count the
same for both engines.

What the `§` points to in the CAP matches: a statute or code section in 494, a regulation in
117, a treatise, restatement or encyclopedia in 129, a document such as a plan, manual or
contract in 17, a constitution in 5 and a court rule in 4. 16 point to a case, and only 3 of
those are pin cites into a case: they cite White & Willson's *Condensed Reports*, a Texas
reporter numbered by section, which neither engine extracts. The other 13 are digests that end
a paragraph or sentence with `Id.` and start the next one with `§`. A section pin cite after a
case almost never points into the case, which is what the unresolved answer says.

Where the engines resolve differently, citare is right in 168 and eyecite in 18:

- **eyecite resolves to a case, citare leaves it unresolved** (17 in CAP, 12 in
  CourtListener). In 26 the section belongs to a statute, a treatise, a manual or an agreement
  cited by name only, and eyecite resolves to whatever case came before:
  `47 Am.Jur.2d Judgments § 645, citing ... Aetna Casualty & Surety Co. v. Jones, 220 Conn. 285
  ... Id. at § 648` resolves to *Aetna*; `Moore's Federal Practice ¶ 41.06 ... Id. at
  § 41.02[4]` resolves to *Citizens Savings*; five TCPA cites in two Texas opinions,
  `Id. § 27.006(a)`, resolve to the last Texas case. In the other 3, all in Silvernail's digest
  reports, the `Id.` ends a sentence and `§ 23, unlike some other sections of the act, does not
  impose ...` starts the next, so the bare `Id.` is right and citare takes a section that
  isn't its pin cite. These are the only 3 of the 13 "not a pin cite" matches citare gets
  wrong; in the other 10 the `§` starts a new line, and citare leaves it alone.
- **eyecite leaves it unresolved, citare resolves to a statute or regulation** (84 in CAP, 74
  in CourtListener). These are id. chains: `20 C.F.R. § 404.1520 ... Id. § 404.1520(e) and (f)
  ... Id. § 404.1520(g)`, where eyecite's unknown `§` cuts the chain after the first link.
  citare is right in 141, eyecite in 15, neither in 2. The chain also carries an earlier
  mistake forward: `See generally 25 C.F.R. pt. 81` isn't extracted, so the first
  `id. § 81.6` resolves to the statute cited before it, 25 U.S.C. § 476, in both engines, and
  citare resolves the next six to it too where eyecite leaves them unresolved.
- **After a paged law citation** (3 in CAP, 11 in CourtListener). As first opened, #41 left
  every section pin cite after a citation with a page unresolved, as it does after a case. On
  the first id. citation after the paged law, eyecite's answer was right in 4
  (`Pub.L.No.91-184, § 3(2), 83 Stat. 841 ... id. § 4(b)`, `1980 Haw. Sess. Laws 366 ... Id.
  § 1, at 367`, `52 Stat. 1250`, `1 Stat. 112`) and the unresolved one in 2, where the paged
  citation is the Federal Register rule or the session law that amended the code section the
  pin cite names (`82 Fed. Reg. 5844 ... Id. § 404.1520c(c)(1)`). citare now resolves a
  section pin cite into a paged law, which makes that first answer eyecite's. The id. chain
  then carries the two wrong ones on through 4 more citations, which #41 as first opened left
  unresolved; that is the general cost of the chain, counted in the previous item.

citare's pin cite stops early in 117 of the 1,268 pin cites: it reads the section number and
its parenthesized subsections, and leaves out what follows a comma or a space, such as
`, subd. (a)`, `, comment (d)`, `, cmt. (n.2)`, `[4]`, `, -3202` and ` to 216.111`. It misses
the pin cite entirely in 46, where its answer is eyecite's: 27 have a line break between `Id.`
and the section number (`id.\n§ 7407(d)(1)`, `Id. §\n401`), 3 a non-breaking space after the
`§`, others Unicode hyphens (`§ 76‐9‐101`), roman numerals (`§ IV(B)(6)(j)`) or OCR damage
(`§l(h)`). Line breaks are deliberate for now: in the CAP digests a `§` on the next line starts
a new paragraph. All of these are follow-ups to the pin cite grammar, not reasons to drop it.

### Tests

- `crates/citare-core/tests/id_section_pin.rs`: every row above, with the span, pin cite,
  parenthetical and resolution of each, plus the `¶` and page pin cite cases that match eyecite.
- citation-bench `vectors/divergences/`: the same 18 inputs as `find` and `resolve` vectors
  (`mise run bench:divergences citare`). eyecite's misses are listed in
  `baselines/eyecite.known-differences`.

### Upstream

eyecite issue [#299](https://github.com/freelawproject/eyecite/issues/299) reports the missing
pin cite, and [PR #343](https://github.com/freelawproject/eyecite/pull/343) captures it. The
draft follow-up in [`eyecite-issue-drafts/id-section-pin.md`](eyecite-issue-drafts/id-section-pin.md)
adds the resolution side and the corpus evidence: #343 resolves the same way citare does
after a case and in the id. chain, but leaves `Id. § 4(b)` after `83 Stat. 841` unresolved
and drops the subsection from `§ 4(b)`.
