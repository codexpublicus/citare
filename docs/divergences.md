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
