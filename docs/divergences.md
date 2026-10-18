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
| [`§` pin cites on `Id.` and `Ibid.`](#id-section-pin) | #41, #48 | [freelawproject/eyecite#299](https://github.com/freelawproject/eyecite/issues/299); follow-up drafted in [`id-section-pin.md`](eyecite-issue-drafts/id-section-pin.md) |
| [Statute section numbers with letters or Unicode hyphens](#law-section) | #48 | drafted in [`law-section.md`](eyecite-issue-drafts/law-section.md) (reporters-db) |
| [Supra after parallel citations](#supra-after-parallel) | #60 | drafted in [`supra-after-parallel.md`](eyecite-issue-drafts/supra-after-parallel.md) |
| [Codes cited by subject keep the subject](#law-subject-split) | #49 | drafted in [`law-subject-split.md`](eyecite-issue-drafts/law-subject-split.md) |
| [State-code statute forms](#state-code-forms) | #69 | drafted in [`state-code-forms.md`](eyecite-issue-drafts/state-code-forms.md) (reporters-db) |
| [Parallel citation groups stay within one decision](#parallel-groups) | #66 | drafted in [`parallel-groups.md`](eyecite-issue-drafts/parallel-groups.md) |
| [Reference citations after parallel citations](#reference-after-parallel) | #67 | drafted with the supra case in [`supra-after-parallel.md`](eyecite-issue-drafts/supra-after-parallel.md) |

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

**The longer forms (#48).** The pin cite runs on through what real opinions put after the
section number, where eyecite still returns a bare `Id.` and an unknown `§`:

| Input after `Id.` | citare's pin cite |
|---|---|
| `§ 5304, subd. (a)`, `§ 3, subd. 6` | the subdivision |
| `§ 2B1.1, cmt. (n.2)`, `§ 2L1.2 cmt. n. 1(B)(iii)`, `§ 381, comment a`, `§ 2L1.2 application n. 5`, `§ 199, illus. 1`, `§ 2, cl. 1` | the comment, note, illustration or clause |
| `§ 41.02[4]`, `§ 261[1][a]`, `§ 6105(a.1)(1)`, `§ 16.012(d-1)`, `§ 1677(7)(C) (iii)` | bracketed, dotted and spaced subsections |
| `§§ 216.103 to 216.111`, `§§ 58 through 61` | the range |
| `§§ 2007.023(b), .024(a)`, `§§ 22-501, -3202`, `§§ 23-602(A)(8); 23-603(A)(4)`, `§ 457 and § 459` | the whole list; the second `§` is not an unknown citation |
| `§\n401`, `§` and a non-breaking space, `§ 76‐9‐101` (U+2010), `§ IV(B)(6)(j)` | the section |
| `id.\n§ 7407(d)(1)`, `Id. at\n\n§ 2149(c)` | the section on the next line |

A semicolon continues a list only when the next item ends at punctuation, so `Id. § 5; 28
U.S.C. § 1331` stops at `§ 5`, and a list stops before a number followed by a capitalized word,
which is the next citation's volume (`Id., §501 (b), 17 P.S. §211.501 (b)`).

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
that doesn't directly follow the id. citation (`Id. The rule in § 5 applies.`), and a
capitalized `Id.` at the end of a line with the `§` on the next one. Neither engine finds an `Id.` that directly
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

As #41 first merged it, citare's pin cite stopped early in 117 of the 1,268 pin cites: it read
the section number and its parenthesized subsections, and left out what follows a comma or a
space, such as `, subd. (a)`, `, comment (d)`, `, cmt. (n.2)`, `[4]`, `, -3202` and
` to 216.111`. It missed the pin cite entirely in 46, where its answer was eyecite's: 28 have
a line break before or after the `§` (`id.\n§ 7407(d)(1)`, `Id. §\n401`), 4 a non-breaking
space, others Unicode hyphens (`§ 76‐9‐101`), roman numerals (`§ IV(B)(6)(j)`) or OCR damage
(`§l(h)`). #48 extended the grammar. On the same opinions, 93 of the 117 and 33 of the 46 are
now right, 27 still stop early (mostly OCR damage, and lists whose items carry their own
parentheticals), 9 are still missed and 1 is wrong (a page number after `§\n\n`). No pin cite
that was right before got worse, and resolution changed in 10 matches, each to the right answer.
6 of the 9 still missed are a capitalized `Id.` with the `§` on the next line, which citare
keeps as a stop: in the CAP digests that layout starts a new paragraph, and none of the 10 CAP
matches like it is a pin cite. After a lowercase `id.` or after `at`, the next line's `§` is
the pin cite. The rows, before and after, are in
[`evidence/section-pin-forms.md`](evidence/section-pin-forms.md).

### Tests

- `crates/citare-core/tests/id_section_pin.rs`: every row above, with the span, pin cite,
  parenthetical and resolution of each, plus the `¶` and page pin cite cases that match eyecite.
  `section_pin_forms`, `more_section_pin_forms`, `section_pin_stops` and
  `capitalized_id_before_a_line_break_is_not_a_pin` cover #48's forms and where they stop.
- citation-bench `vectors/divergences/`: 34 inputs as `find` and `resolve` vectors, the 18
  from #41 and 16 of #48's forms (`section_pin_forms`) (`mise run bench:divergences citare`).
  eyecite's misses are listed in `baselines/eyecite.known-differences`.

### Upstream

eyecite issue [#299](https://github.com/freelawproject/eyecite/issues/299) reports the missing
pin cite, and [PR #343](https://github.com/freelawproject/eyecite/pull/343) captures it. The
draft follow-up in [`eyecite-issue-drafts/id-section-pin.md`](eyecite-issue-drafts/id-section-pin.md)
adds the resolution side and the corpus evidence: #343 resolves the same way citare does
after a case and in the id. chain, but leaves `Id. § 4(b)` after `83 Stat. 841` unresolved
and drops the subsection from `§ 4(b)`.

<a id="law-section"></a>
## Statute section numbers with letters or Unicode hyphens

reporters-db's `law_section` regex, which eyecite and citare both build their statute
citations from, takes digits joined by ASCII hyphens, dots and colons
(`\d+(?:[\-.:]\d+){,3}`). A section number with a letter in it stops at the part before the
letter, and one with U+2010 or U+2011 for its hyphens stops at the first hyphen. citare
overrides the variable in `crates/citare-core/build.rs`: a part may end in up to two lowercase
letters or one capital, a hyphen may introduce up to three letters, and U+2010 and U+2011
separate parts like a hyphen.

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `See Mich. Comp. Laws § 600.2912a.` | `Mich. Comp. Laws § 600`, section `600` | `Mich. Comp. Laws § 600.2912a`, section `600.2912a` |
| `See 42 U.S.C. § 2000e-5(f).` | no law citation; unknown citation `§` | `42 U.S.C. § 2000e-5`, section `2000e-5`, pin cite `(f)` |
| `See 17 C.F.R. § 240.10b-5.` | `17 C.F.R. § 240`, section `240` | `17 C.F.R. § 240.10b-5`, section `240.10b-5` |
| `See Utah Code Ann. § 76‐9‐101.` (U+2010) | `Utah Code Ann. § 76`, section `76` | `Utah Code Ann. § 76‐9‐101`, section `76‐9‐101` |
| `See N.Y. Gen. Bus. Law § 394-ccc(1).` | `N.Y. Gen. Bus. Law § 394`, section `394` | `N.Y. Gen. Bus. Law § 394-ccc`, section `394-ccc`, pin cite `(1)` |

Unchanged: sections without letters or Unicode hyphens (`§ 76-9-101`, `§ 1170.12`,
`§ 161.001(b)(1)(D)`), and reporters-db's second form, a number with parenthesized parts
(`81(a)(2)`).

### Evidence

The CourtListener samples from the `id-section-pin` study (8,409 opinions), run through citare
with and without the override. With it, citare finds 793 statute citations it didn't find
before and extends 85 it cut short, in 231 opinions. It loses none, and none of the statute
citations it already read to the end changes. Most of the 793 have a letter straight after the
first number, where the old regex can't end the section at all, so the citation isn't found:
`42 U.S.C. § 2000e-5` (250 of them in 42 U.S.C.), `15 U.S.C. § 78j(b)`, `19 U.S.C. § 1677b`,
`8 U.S.C. § 1229b`, `Minn.Stat. § 253B.03`, `N.C. Gen.Stat. § 7A-450`. The 85 extended ones
include `17 C.F.R. § 240.10b-5`, `38 C.F.R. § 4.71a`, `Treas.Reg. § 1.170A-1`,
`Conn. Gen. Stat. § 42-110b`, and Utah and Nebraska sections written with U+2010 or U+2011.
I read a random 40 of the new ones in context and all 85 extended ones: each section is the
whole section number and nothing after it.

### Tests

- `crates/citare-core/tests/law_section.rs`: each row above, and sections the override leaves
  alone.
- citation-bench `vectors/divergences/`: the five inputs above as `find` vectors
  (`law_section`). eyecite's misses are listed in `baselines/eyecite.known-differences`.

### Upstream

The regex is reporters-db's, so the fix belongs there; a draft issue is in
[`eyecite-issue-drafts/law-section.md`](eyecite-issue-drafts/law-section.md).

<a id="supra-after-parallel"></a>
## Supra after parallel citations

A decision cited in parallel is one full citation per reporter: `Smith v. Jones, 275 Conn. 38,
881 A.2d 194 (2005)` is two, and both carry the case name. eyecite resolves a supra by finding
the earlier full citations whose plaintiff or defendant contains its antecedent, and resolves
only when they are all one resource. Here they are two resources, so the supra resolves to
nothing, and so does every id. citation after it. A short cite (`Smith, 881 A.2d at 199`) is
matched by reporter and volume first, so it finds one citation and isn't affected.

citare records which full case citations are cited in parallel: adjacent full case citations
with the same full span start and case name, the pairs eyecite and citare already copy the case
name and year across. When a supra's or short cite's matches are different resources and one
parallel citation covers all of them, citare resolves to the match in the short form's reporter,
or else the first match. Matches from two different decisions with the same name still leave
it unresolved.

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). … Smith, supra, at 46. Id. at 47.` | supra and `Id.` **unresolved** | both resolve to `275 Conn. 38` |
| `Roe v. Wade, 410 U.S. 113, 93 S. Ct. 705, 35 L. Ed. 2d 147 (1973). … Roe, supra, at 120.` | **unresolved** | `410 U.S. 113` |
| `Haynes v. Middletown, 314 Conn. 303, 101 A.3d 249 (2014). … Haynes v. Middletown, supra, at 312, 101 A.3d 249. … Haynes, supra, at 313.` | both supras **unresolved** | both resolve to `314 Conn. 303` |
| `Smith v. Jones, 881 A.2d 194 (Conn. 2005). … Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). Smith, supra, at 46.` | **unresolved** | `881 A.2d 194` |
| `Smith v. Jones, 1 U.S. 1 (1990). … See also Smith v. Jones, 410 U.S. 113 (1999). Smith, supra, at 3.` | unresolved | unresolved (two decisions) |

The third row is how Connecticut opinions cite: after the first full citation, each supra
repeats the regional cite (`Haynes v. Middletown, supra, 314 Conn. at 312, 101 A.3d 249`). The
repeated `101 A.3d 249` is a full citation on its own, the same resource as the first one, so a
later supra matches three citations of two resources, which the first parallel citation covers.
Each parallel citation counts only as itself: two of them that share a resource aren't merged.
In one CourtListener opinion, citare reads `(citing Celotex, 477 U.S. at 321-25, 106 S.Ct.
2548; Anderson, 477 U.S. at 255-57, 106 S.Ct. 2505)` as if both S. Ct. cites were parallel
citations of one case named Celotex.
Merged with the earlier `Anderson v. Liberty Lobby, Inc., 477 U.S. 242, 106 S.Ct. 2505`, that
pair would make Celotex and Anderson one decision, and `477 U.S. at 255` would resolve to
Celotex.

### Evidence

138 A.3d 1012 (Brooks v. Powers, Conn. App. 2016) has 52 supras, and eyecite and citare both
left all 52 unresolved: every case it cites by supra was first cited in Conn. and A.2d or A.3d.
With this change 45 resolve. The other 7 are `Cotto v. Board of Education, supra` and `Swanson
v. Groton, supra`, whose antecedent guess is the defendant (`Education`, `Groton`), which two
other cases in the opinion share; that ambiguity is real and is left alone. In a sample of 38
recent opinions from A.3d, P.3d, N.E.3d, So. 3d, S.E.2d, S.W.3d and N.W.2d, eyecite and citare
both resolved 13 of 49 supras; citare now resolves 35.

On the CourtListener ten-percent sample (7,632 opinions, citation-bench's
`courtlistener-10pct`), the citare CLI's `resolve` resolves 2,263 of 5,140 supras, up from
1,227, and 9,880 id. citations, up from 9,825 (the ones after a newly resolved supra). Short
and reference citations are unchanged, and no citation that resolved before resolves to a
different resource. I read 23 of the 1,036 newly resolved supras, picked at random, in
context: each names the decision it resolves to (`First Nat. Bank in Albuquerque v. Rowe,
supra` to `52 N.M. 366`, `Gannett Co., Inc. v. DePasquale, supra` to `443 U.S. 368`, `In re
Murchison, supra` to `349 U.S. 133`).

### Tests

- `crates/citare-core/src/resolve.rs`: the Conn. and A.2d supra with an id. citation after it,
  a supra after U.S., S. Ct. and L. Ed. 2d cites, the Connecticut repeated cite, a decision
  cited alone and then in parallel, a short cite in the parallel reporter (which still resolves
  to its own citation, as in eyecite), and two decisions with one name, cited singly and in
  parallel, whose supra stays unresolved.
- citation-bench `vectors/divergences/`: the first four rows of the table as `resolve` vectors
  (`supra_after_parallel`). eyecite's misses are listed in
  `baselines/eyecite.known-differences`.

### Upstream

Not filed; a draft is in
[`eyecite-issue-drafts/supra-after-parallel.md`](eyecite-issue-drafts/supra-after-parallel.md).

<a id="law-subject-split"></a>
## Codes cited by subject keep the subject

reporters-db keys some codes without the subject that names them, and matches the subject
with a `subject` group: `N.Y. Law` (`N.Y. Gen. Bus. Law § 394`), `Cal. Code` (`Cal. Civ.
Code § 1714`), `Tex. Code Ann.` (`Tex. Penal Code Ann. § 22.01`), `N.Y.` (`N.Y. C.P.L.R. §
5501`) and `Md. Code Ann.` (`Md. Code Ann., Cts. & Jud. Proc. § 5-101`). eyecite's normalized
reporter is the key, so every New York, California and Texas code normalizes to the same
string. New York has a second problem: its `N.Y.` and `N.Y. Law` entries both match `N.Y.
Gen. Bus. Law § 394`, with different groups, and eyecite keeps whichever of the two
extractors its set of extractors happens to yield first, which changes from process to
process.

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `N.Y. Gen. Bus. Law § 394` | reporter `N.Y. Gen. Bus. Law`, subject `Gen. Bus.`, normalized `N.Y. Law`; or, in another process, reporter `N.Y.`, subject `Gen. Bus. Law`, normalized `N.Y.` | reporter `N.Y. Gen. Bus. Law`, subject `Gen. Bus.`, normalized `N.Y. Gen. Bus. Law` |
| `Cal. Civ. Code § 1714` | normalized `Cal. Code` | normalized `Cal. Civ. Code` |
| `Tex. Penal Code Ann. § 22.01` | normalized `Tex. Code Ann.` | normalized `Tex. Penal Code Ann.` |
| `N.Y. C.P.L.R. § 5501` | normalized `N.Y.` | normalized `N.Y. C.P.L.R.` |
| `Md. Code Ann., Cts. & Jud. Proc. § 5-101` | normalized `Md. Code Ann.` | normalized `Md. Code Ann., Cts. & Jud. Proc.` |

When two laws.json regexes match the same text, citare keeps the one with the longer
`reporter` group, the more specific entry: `N.Y. Law` over `N.Y.`, the split the `Cal. Code`
and `Tex. Code Ann.` entries already give. The normalized reporter (`corrected_reporter()`,
and `code` in `Citation::law()`) puts the subject back into the key: after its first word for
the codes named around it (`N.Y. Law`, `Cal. Code`, `Tex. Code Ann.`), after the key for the
others, with the comma the text has (`Md. Code Ann., Cts. & Jud. Proc.`). Groups are unchanged
apart from the New York split. Laws also no longer take a CAP reporter slug from a reporter
edition with the same name: the `N.Y.` statute entry had New York Reports' `ny`.

### Evidence

Ten processes of eyecite 2.7.8 on `See N.Y. Gen. Bus. Law § 394 (2020).`: five gave reporter
`N.Y. Gen. Bus. Law` (normalized `N.Y. Law`) and five reporter `N.Y.` (normalized `N.Y.`). A
link to the statute's text needs the code: `N.Y. Law § 394` names no statute, and neither
does `Tex. Code Ann. § 22.01`. Over the CourtListener ten-percent sample, citare's normalized
reporter changes for 144 statute citations: 91 Texas codes, 36 New York, 14 California and
3 Maryland. citation-bench's eyecite vectors have no code cited by subject, so parity is
unchanged.

### Tests

- `crates/citare-core/tests/law_local.rs`: `new_york_code_split`, and the codes cited by
  subject in `statute_fields`.
- citation-bench: `vectors/divergences/` `law_section/4` (`N.Y. Gen. Bus. Law § 394-ccc`), and
  `vectors/statutes/` `cal_civ_code` and `tex_penal_code_ann`, which eyecite misses only on the
  normalized reporter.

### Upstream

The split is eyecite's (the extractor order) and the normalized reporter is eyecite's
`corrected_reporter()`; a draft issue is in
[`eyecite-issue-drafts/law-subject-split.md`](eyecite-issue-drafts/law-subject-split.md).

<a id="state-code-forms"></a>
## State-code statute forms

reporters-db names each state's code by its Bluebook abbreviation, and state courts mostly
cite their codes in their own forms. eyecite returns those as a bare `§` unknown citation.
citare adds the most common forms from its own supplement to reporters-db's laws data,
[`data/laws-local.json`](../data/laws-local.json), written up with the measurement in
[`laws-local.md`](laws-local.md).

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `General Statutes § 52-557n` | unknown citation `§` | law citation: Connecticut, `Conn. Gen. Stat.`, section `52-557n` |
| `Practice Book § 17-49` | unknown citation `§` | Connecticut, `Conn. Practice Book`, section `17-49` |
| `OCGA § 16-9-93` | unknown citation `§` | Georgia, `Ga. Code Ann.`, section `16-9-93` |
| `42 Pa.C.S. § 5322` | unknown citation `§` | Pennsylvania, `Pa. Cons. Stat.`, title `42`, section `5322` |
| `Penal Law § 160.05` | unknown citation `§` | New York, `N.Y. Penal Law`, section `160.05` |
| `(Pen. Code, § 211)` | unknown citation `§` | California, `Cal. Pen. Code`, section `211` |
| `TEX. PENAL CODE ANN. § 1.03(a)` | unknown citation `§` | Texas, `Tex. Penal Code Ann.`, section `1.03`, pin cite `(a)` |
| `G. L. c. 93A, § 2` | unknown citation `§` | Massachusetts, `Mass. Gen. Laws`, chapter `93A`, section `2` |
| `22 O.S. 1951, § 1066` | case citation `22 O.S. 1951` (Ohio State Reports, page 1951) and unknown citation `§` | Oklahoma, `Okla. Stat.`, title `22`, section `1066` |

The rest of the forms are in `data/laws-local.json`: `A.R.S.`, `HRS`, `N.C.G.S.` and `G.S.`,
`N.D.C.C.`, `T.C.A.`, `40 P.S.`, `19-A M.R.S.`, `24 V.S.A.`, `Family Ct Act`, `Code Civ.
Proc.,`, `Tex. Fam.Code`, `MCL`, `NMSA 1978,`, `Ala. Code 1975,`, `A.C.A.`, `La.Rev.Stat.`,
`G.L. 1956`, `West Virginia Code` and `WIS. STAT.`. A statute's `reporter` group also no
longer ends in whitespace: eyecite's `28 U.S.C.` and a line break before the `§` gives reporter
`U.S.C.` plus the line break, and citare's gives `U.S.C.`.

### Evidence

Over the CourtListener ten-percent sample, 820 CAP opinions and the stare.pub sample, citare
finds 4,710 statute citations it didn't, in 988 opinions, and loses none; 85 case citations
that were Oklahoma statutes read as Ohio State Reports go away. I read 40 of the new
citations, drawn at random, in context: all 40 have the right jurisdiction, code and title or
chapter, and 39 the right section
([`evidence/state-code-forms.md`](evidence/state-code-forms.md)). The trimmed reporter group
changes 81 `U.S.C.` citations in the ten-percent sample.

### Tests

- `crates/citare-core/tests/law_local.rs`: every example in `data/laws-local.json`, the fields
  of each form (`statute_fields`), look-alikes that must not match (`not_a_supplement_code`),
  and the reporter group's end (`reporter_group_has_no_trailing_space`).
- citation-bench `vectors/statutes/`: 78 inputs from real opinions over 41 forms, scored with
  `mise run bench:statutes`. eyecite's misses are in `baselines/eyecite.known-differences`.

### Upstream

The forms belong in reporters-db's laws data, where they would reach eyecite too; a draft
issue is in [`eyecite-issue-drafts/state-code-forms.md`](eyecite-issue-drafts/state-code-forms.md).

<a id="parallel-groups"></a>
## Parallel citation groups stay within one decision

A decision cited in parallel is one full case citation per reporter, and eyecite and citare
give them all the case name: `Smith v. Jones, 275 Conn. 38, 881 A.2d 194` is two citations of
one decision, named `Smith v. Jones` and with one full span start. That shared start is what
marks a parallel group, for the name and year copied across in `find` and for the
[supra rule](#supra-after-parallel) in `resolve`. eyecite finds a citation's case name by
searching back from it for a `v.` or another stop word, and the search passes over any
citations on the way. A citation of a second decision that has no `v.` of its own between it
and the first decision's citations takes the first decision's name, start and year, and the
two read as one decision.

citare's search passes over an earlier citation only when nothing but pin cites and
punctuation stand between them (pages and ranges, `n. 6`, `¶ 19`, `418(II)`, brackets and
parentheses, a reporter citare doesn't know such as `2 OBR 380` or `27 L.R.A. (N.S.) 435`) and
the two are in different reporters, since a decision is printed once in each. Anything else
between them, such as `affd.`, `cert. den.`, `and`, a short case name, `Matter of`, or a year
in parentheses that closes the first citation, ends the search, and the case name is whatever
follows the earlier citation, if anything does. The search for a plaintiff after a `v.`
stops at an earlier citation the same way, as does an emphasis tag that holds an earlier
citation or a semicolon. A full case citation that directly follows another with only pin
cites and punctuation between them joins its group, even when its own search for a name ends
elsewhere. The search looks back only 28 words, so in `Texas Industries, 451 U.S. at 629, 101
S.Ct. at 2061, and Northwest Airlines, Inc. v. Transport Workers Union of America, 451 U.S. 77,
101 S.Ct. 1571, 67 L.Ed.2d 750` the first two citations stop at `101 S.Ct. at 2061` and find
`Northwest Airlines, Inc.`, while the third runs out of words first and finds `Inc.`, as eyecite
does for all three.

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `(citing <i>Sutton v. Elrod,</i> 724 So.2d 551 (Ala.Civ.App.1998); <i>Beasley,</i> 564 So.2d 959 (Ala.Civ.App.1990))` | one group; `564 So.2d 959` is `Sutton v. Elrod` | two decisions; `564 So.2d 959` is `Beasley` |
| `(<i>see Matter of Jamal B. v Danielle C.,</i> 38 AD3d 908 [2007]; <i>Matter of Marie H,</i> 25 AD3d 704 [2006])` | one group, both `Matter of Jamal B. v Danielle C.` | two; `25 AD3d 704` is `Matter of Marie H` |
| `<i>Zink v. McManus</i> (49 Hun, 583; affd., 121 N. Y. 259)` | one group | two; the affirmance has no case name |
| `<i>Flaksa v. Little River Marine Construction Co.,</i> 389 F.2d 885 (5th Cir.), cert. den., 392 U.S. 928, 88 S.Ct. 2287, 20 L.Ed.2d 1387 (1968)` | one group of four | the court of appeals' citation alone; the three certiorari citations have no case name |
| `See <i>Commonwealth v. Hart,</i> 470 Mass. 837 and 472 Mass. 448 (2015)` | one group | two |
| `Perry v. Beverage, 121 Wash. 652, 209 P. 1102 (1922), 214 P. 146 (1923)` | one group of three | `121 Wash. 652` and `209 P. 1102`; `214 P. 146`, the opinion on rehearing, alone |
| `in Daubert, 509 U.S. 579, 113 S.Ct. 2786, 125 L.Ed.2d 469, and Kumho Tire, 526 U.S. 137, 119 S.Ct. 1167, 143 L.Ed.2d 238` | one group of six, all `Daubert` | `Daubert` and `Kumho Tire`, three each |
| `McMann v. Richardson, 397 U.S. 759, 90 S.Ct. 1441, 25 L.Ed.2d 763 (1970) and Brady v. United States, 397 U.S. 742, 90 S.Ct. 1463, 25 L.Ed.2d 747 (1970)` | `397 U.S. 742` and `90 S.Ct. 1463` are `Richardson`, plaintiff `""`; `25 L.Ed.2d 747` alone is `Brady v. United States` | two groups of three, `McMann v. Richardson` and `Brady v. United States` |
| `Northwest Airlines, Inc. v. Transport Workers Union of America, 451 U.S. 77, 101 S.Ct. 1571, 67 L.Ed.2d 750`, after `Texas Industries, 451 U.S. at 629, 101 S.Ct. at 2061, and` | one group, plaintiff `Inc.` | one group, plaintiff `Northwest Airlines, Inc.` |
| `Wade v. State, 236 Ind. 350, 140 N.E.2d 98, and Hughes v. State, 212 Ind. 577, 10 N.E.2d 629` | two groups; the second has plaintiff `""` | two; the second is `Hughes v. State` |
| `<em>Bate v. Hill, 1 Car. & P., 100; Turnpike Co. v. Loomis, 32 N. Y., 127; La Beau v. The People, 34 N. Y., 223; Com. v. Jennings, 107 Mass., 488.</em>` | one group of three, all plaintiff `Bate` | three decisions, each with its own name |
| `Celotex Corp. v. Catrett, 477 U.S. 317, 322, 106 S.Ct. 2548; Anderson v. Liberty Lobby, Inc., 477 U.S. 242, 248, 106 S.Ct. 2505` | two groups | the same |
| `People v Smith, 392 Mich 30, 35; 219 NW2d 41 (1974)`, in markup | one group | the same |

The last two rows are the forms that stay as they were: the citing list from #66 already gave
two groups, and Michigan separates parallel citations with a semicolon. Two things are left
out. A later decision's own parallel citations in subsequent history (`cert. den., 392 U.S.
928, 88 S.Ct. 2287, 20 L.Ed.2d 1387`) have no case name, so they form no group; eyecite put
them in the first decision's group. And an opinion and its rehearing in one reporter (`86 Cal.
449 [25 P. 16, 26 P. 800]`) are split, though one case.

### Evidence

On the CourtListener ten-percent sample (7,632 opinions), I counted groups as runs of full
case citations, in output order, with one full span start. There were 19,518 before and are
19,666 after. I flagged a group as spanning two decisions when some pair of its citations has
subsequent history, a new case name or a closing year between them: 448 groups before, 3
after, and those 3 are one correct group (`Housel v. Pacific Elec. Ry. Co., 167 Cal. 245 [139
P. 73, Ann.Cas. 1915C 665, 51 L.R.A.N.S. 1105]`) that the flag misreads. eyecite 2.7.8's
search is the same as citare's was, so it has the same 448.

787 pairs of citations are no longer in one group:

| Between the two citations | Pairs |
|---|---:|
| subsequent history (`affd.`, `cert. den.`, `rev. den.`, `modified`) | 297 |
| a new case name (`; Matter of Daniel C.,`, `, and Troxel,`) | 158 |
| a closing year (`(1922),`, `[2007];`) | 21 |
| other text (court parentheticals, footnote markers, a sentence break, `and`) | 91 |
| only pin cites and punctuation, in one reporter (rehearings, repeated citations, two annotations) | 95 |
| only pin cites and punctuation, the parallel citations of a later decision in subsequent history | 108 |
| only pin cites and punctuation, otherwise | 17 |

The 17 are a typo (`305 U.S. 197. 229, 59 S.Ct. 206`), nameless citations after an earlier
decision (`and 20 N.J. 537, 120 A.2d 442`), a bracketed year in the middle of a group (`3 N.J.
450, [1950], 70 A.2d 736`) and a run of nameless authorities (`1 Edw. Ch. 396; 11 Wheat. 78`).
784 pairs are newly in one group. I read 25 of them, drawn at random, in context: all 25 are
parallel citations of one decision, most of them Michigan's semicolons in plain text, which
eyecite's plain-text search stops at, and citations past the 28-word window. In all, the
plaintiff changes on 1,641 citations, the defendant on 1,800, the full span on 1,975 and the
year on 352, and 5 opinions gain 6 reference or supra citations that a wrong case name or an
overlong full span had hidden; none loses one.

In `resolve`, 60 short, supra and reference citations resolve that didn't (37 supras, 20
references, 3 short cites); I read 16 of them, drawn at random, and each resolves to the case
the text names (`Keko Industries, supra` to `192 Ct.Cl. 773`, `Cimino` to `54 AD2d 843`, `First
Nationwide` to `27 F.3d 763`). 20 no longer resolve. 13 are references to a decision that is
now one group of several citations, which the rule for references doesn't cover yet (#67). Two
are a supra of `Brown v. Liberty Loan Corp. of Duval`, now two decisions with one name (`392
F.Supp. 1023 (M.D.Fla.1975), rev'd, 539 F.2d 1355`), which leaves a bare supra ambiguous by
the [supra rule](#supra-after-parallel). Two are a page typo in the opinion (`169 So. 39` and
`169 So. 40` for one decision), two are short cites after a pin cite read as a full citation
(`Id., 340 U.S. 478, 479, 71 S.Ct., 460`), and one is an id. citation. 4 resolve to a different
citation: a supra of `Stanford v. Texas` moves to the first citation of the same decision, and
three id. citations used as "the same case name" before another decision's citation (`In re
Murray (1990), 52 Ohio St.3d 155, 157; Stanley v. Illinois (1972), 405 U.S. 645, 651. Id.,
(1990), 52 Ohio St.3d 155`) now follow eyecite's rule and resolve to the citation before them.
For two of them that is another decision in the same case; for Murray it is Stanley v.
Illinois, which is wrong. Speed is unchanged (1.79 to 1.81 ms/KB on the 30 largest
opinions against 1.83 to 1.94 before).

### Tests

- `crates/citare-core/tests/parallel_groups.rs`: each row of the table, the pin cite forms
  that must stay grouped (`301(4)(b)`, `703 and n. 6`, `136-160`, `418(II)`, `74 PUR4th 464`,
  `----`, `et seq.`, `syllabus ¶ 3`, `Ann. Cas. 1918C, 529`, `s.c.`, `27 L.R.A. (N.S.) 435`, a
  bracketed pin `[1082]`), a citation past the search window, and two citations in one
  reporter.
- citation-bench `vectors/divergences/`: 11 `find` vectors and 2 `resolve` vectors
  (`parallel-groups/...`), the `resolve` ones a supra of `Kumho Tire` and of `Beasley`, which
  eyecite leaves unresolved. eyecite's misses are in `baselines/eyecite.known-differences`.

### Upstream

Not filed; a draft is in
[`eyecite-issue-drafts/parallel-groups.md`](eyecite-issue-drafts/parallel-groups.md).

<a id="reference-after-parallel"></a>
## Reference citations after parallel citations

A reference citation names a case already cited in full, by a party name and a pin cite in
plain text (`In Haynes at 312`) or by an emphasized name in markup (`In <i>Haynes,</i> the
court held`). eyecite resolves it to the earlier full citations whose metadata holds the name,
and only when they are all one resource. A decision cited in parallel is two or three
resources, so the reference resolves to nothing, as a supra did before
[#60](#supra-after-parallel), and so does an id. citation after it.

citare applies the supra rule: when every match is one decision cited in parallel (one
[parallel group](#parallel-groups)), the reference resolves to the first match. Two decisions
with one name still leave it unresolved, and two groups that share a citation aren't merged.

| Input | eyecite 2.7.8 | citare |
|---|---|---|
| `Haynes v. Middletown, 314 Conn. 303, 101 A.3d 249 (2014). … In Haynes at 312, the court held` | **unresolved** | `314 Conn. 303` |
| `Ankenbrandt v. Richards, 504 U.S. 689, 112 S.Ct. 2206, 119 L.Ed.2d 468 (1992). … Under Ankenbrandt at 703` | **unresolved** | `504 U.S. 689` |
| `<i>Haynes v. Middletown,</i> 314 Conn. 303, 101 A.3d 249 (2014). … In <i>Haynes,</i> the court held … <i>Id.</i> at 313.` | reference and `Id.` **unresolved** | both resolve to `314 Conn. 303` |
| `Haynes v. Middletown, 314 Conn. 303, 101 A.3d 249 (2014). … See also Haynes v. Middletown, 320 Conn. 1, 120 A.3d 1 (2015). In Haynes at 312` | unresolved | unresolved (two decisions) |

### Evidence

On the CourtListener ten-percent sample (7,632 opinions), the citare CLI's `resolve` resolves
6,128 of 6,363 reference citations, up from 3,060 with [parallel groups](#parallel-groups) alone
and 3,053 on `main` before either, and 9,021 id. citations, up from 8,753 (the ones after a
newly resolved reference). No citation that resolved before stops resolving or resolves to a
different resource. I read 20 of the 3,068 newly resolved references, drawn at random, in
context: each names the case it resolves to (`Gates` to `462 U.S. 213`, Illinois v. Gates;
`Tinker` to `393 U.S. 503`; `Schlunk` to `486 U.S. 694`, Volkswagenwerk Aktiengesellschaft v.
Schlunk; `Mayl at ¶ 56` to `106 Ohio St.3d 207`). References are rare in the CAP plain text
stare serves: 138 A.3d 1012 (Brooks v. Powers) has none, and the 38-opinion sample from #60
has one, `Puksar at 292`, which stays unresolved because two decisions are named Commonwealth
v. Puksar (597 Pa. 240 and 559 Pa. 358).

### Tests

- `crates/citare-core/src/resolve.rs`: a reference after Conn. and A.3d cites, after U.S.,
  S.Ct. and L.Ed.2d cites, an emphasized reference in markup, and two decisions with one name,
  cited singly and in parallel, whose reference stays unresolved.
- citation-bench `vectors/divergences/`: the first three rows of the table as `resolve` vectors
  (`reference-after-parallel`). eyecite's misses are in `baselines/eyecite.known-differences`.

### Upstream

Not filed; it is the same fix as the supra case, and the draft in
[`eyecite-issue-drafts/supra-after-parallel.md`](eyecite-issue-drafts/supra-after-parallel.md)
covers both.
