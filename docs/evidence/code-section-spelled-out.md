# Evidence: code citations with the section spelled out, a comma or a year

The hand-checked sample behind the #117 section of [`docs/laws-local.md`](../laws-local.md#section-spelled-out)
and [the `state-code-forms` entry](../divergences.md#state-code-forms): 40 statute citations
drawn at random (seed 117) from the 2,416 that citare finds with the #117 entries of
`data/laws-local.json` and didn't find before, over the CAP statute sample (12,337 opinions)
and citation-bench's stare.pub sample (1,000 opinions). Columns: the source opinion, the text
with the citation's full span in `⟦ ⟧`, and the fields citare gives. The last column is
whether the jurisdiction, code, section and pin cite are what the opinion cites.

All 40 have the right jurisdiction and code, and 39 the right section. Row 17 is CAP's OCR
of `W. Va. Code, 23-5-1c` as `23-5-lc`, and the section stops at `23-5`. In the first run, a
different draw had `Indiana Code Section 31-15-7-9.1` with section `31-15-7-9`: an Indiana
section has four parts, any of them with a decimal, one more part than `$law_section` takes.
The Indiana entry has its own section pattern since, and this table is the run after that fix.

| Row | Source | Text | Jurisdiction | Code | Title, chapter, subject | Section | Pin cite | Right |
|---|---|---|---|---|---|---|---|---|
| 1 | CAP ne2d 794, 1158-01 | … of the felonies for which the person has been con-viected. ⟦Ind.Code, § 85-50-1-2(c)⟧. At the time of Hawkins' crim… | Indiana | Ind. Code |  | 85-50-1-2 | (c) | ✓ |
| 2 | CAP w-va 172, 0656-01 | …t after his arrest is taken before a magistrate pursuant to ⟦W.Va.Code, 62-1-5 ⟧[1965], and is, inter alia, in… | West Virginia | W. Va. Code |  | 62-1-5 |  | ✓ |
| 3 | CAP w-va 172, 0375-01 | …reement between the parties is an assignment of wages under ⟦W.Va.Code, 46A-2-116(2)(b) ⟧[1974], that is subject to the… | West Virginia | W. Va. Code |  | 46A-2-116 | (2)(b) | ✓ |
| 4 | CAP w-va 172, 0575-01 | …ion thereof.” This section must be read in conjunction with ⟦W.Va.Code, 55-6-2 (1981)⟧, which provides that “[i]f th… | West Virginia | W. Va. Code |  | 55-6-2 |  | ✓ |
| 5 | CAP idaho 133, 0353-01 | …court was required to grant his motion for change of venue. ⟦Idaho Code Section 5-404 ⟧reads, in relevant part: [I]n … | Idaho | Idaho Code |  | 5-404 |  | ✓ |
| 6 | CAP w-va 163, 0154-01 | …arged. These errors by the majority result in its declaring ⟦W. Va. Code, 61-11-14⟧, unconstitutional — a needles… | West Virginia | W. Va. Code |  | 61-11-14 |  | ✓ |
| 7 | CAP cal-4th 50, 0802-01 | …e in sexually violent criminal behavior. By contrast, under ⟦Penal Code section 1367⟧, “[a] defendant is mentally i… | California | Cal. Penal Code | subject Penal | 1367 |  | ✓ |
| 8 | CAP w-va 172, 0502-01 | …nd made a jail sentence mandatory. The initial enactment of ⟦W.Va.Code, 62-12-2(b)⟧, occurred in 1979, and this p… | West Virginia | W. Va. Code |  | 62-12-2 | (b) | ✓ |
| 9 | CAP w-va 163, 0454-01 | …nts. A letter dated March 26, 1979 advised: Consistent with ⟦West Virginia Code 18A-2-7 ⟧authorizing statutory procedur… | West Virginia | W. Va. Code |  | 18A-2-7 |  | ✓ |
| 10 | CAP w-va 172, 0147-01 | …a qualified newspaper published in the publication area_” . ⟦W.Va.Code, 38-1-4 ⟧[1968], was amended in 1980. W… | West Virginia | W. Va. Code |  | 38-1-4 |  | ✓ |
| 11 | CAP cal-4th 19, 0925-01 | …chers—or, therefore, vicariously against the District under ⟦Government Code section 815.2⟧—because his injury did not “r… | California | Cal. Government Code | subject Government | 815.2 |  | ✓ |
| 12 | CAP w-va 163, 0198-01 | …bility benefits where a protest has been made to them under ⟦W. Va. Code, 23-5-1⟧. The last paragraph of W. Va.… | West Virginia | W. Va. Code |  | 23-5-1 |  | ✓ |
| 13 | CAP w-va 172, 0383-01 | …igibility requirements bar all strikers from benefits, then ⟦W.Va.Code, 21A-6-3(4) ⟧would be meaningless. When a c… | West Virginia | W. Va. Code |  | 21A-6-3 | (4) | ✓ |
| 14 | CAP cal-4th 19, 0925-01 | …7 Cal.App.3d 41, 44-45 [242 Cal.Rptr. 752]: “The effect of [⟦Government Code section 815.2⟧] is to incorporate ‘general s… | California | Cal. Government Code | subject Government | 815.2 |  | ✓ |
| 15 | CAP cal-4th 50, 0802-01 | …r the exercise of [this court’s] inherent power to look to [⟦Penal Code section 1367⟧] in order to fill the gap in … | California | Cal. Penal Code | subject Penal | 1367 |  | ✓ |
| 16 | CAP idaho 133, 0489-01 | …he district court’s denial of his motion for such services. ⟦Idaho Code Section 19-852(a)(2) ⟧states, in relevant part, that… | Idaho | Idaho Code |  | 19-852 | (a)(2) | ✓ |
| 17 | CAP w-va 163, 0107-01 | …subsequent right to protest continuation of the award under ⟦W. Va. Code, 23-5⟧-lc, unnecessary disruption is… | West Virginia | W. Va. Code |  | 23-5 |  | code right; the section is `23-5` for the OCR's `23-5-lc` (`23-5-1c`) |
| 18 | CAP idaho 138, 0632-01 | … P.2d at 1101; Knutson, 121 Idaho at 104, 822 P.2d at 1001. ⟦Idaho Code Section 18-2202(3) ⟧provides that any “person who … | Idaho | Idaho Code |  | 18-2202 | (3) | ✓ |
| 19 | CAP w-va 172, 0413-01 | …s of the new municipal law and existing charter provisions, ⟦W.Va.Code, 8-1-6⟧.” (Emphasis added; footnotes … | West Virginia | W. Va. Code |  | 8-1-6 |  | ✓ |
| 20 | CAP w-va 163, 0620-01 | …ny condition of his release on parole.” (emphasis supplied) ⟦W. Va. Code, 1931, 62-12-19⟧, as amended. The Morrissey co… | West Virginia | W. Va. Code |  | 62-12-19 |  | ✓ |
| 21 | CAP w-va 172, 0580-01 | …one by analogy to the one-term rule in circuit courts under ⟦W.Va. Code, 62-3-1 (1975)⟧. We stated in Stiltner that "… | West Virginia | W. Va. Code |  | 62-3-1 |  | ✓ |
| 22 | CAP w-va 163, 0400-01 | …unicipal officers and employees. ... Appellees contend that ⟦W. Va. Code 8-5-11 ⟧[1969] and the City Charter re… | West Virginia | W. Va. Code |  | 8-5-11 |  | ✓ |
| 23 | CAP w-va 163, 0085-01 | …courts on the basis of fraud or on those grounds set out in ⟦W. Va. Code, 55-10-4⟧. These principles are more fu… | West Virginia | W. Va. Code |  | 55-10-4 |  | ✓ |
| 24 | CAP w-va 172, 0575-01 | …cited the West Virginia Consumer Credit and Protection Act, ⟦W.Va.Code, 46A-2-107(4)⟧, which provides in relevant p… | West Virginia | W. Va. Code |  | 46A-2-107 | (4) | ✓ |
| 25 | CAP w-va 172, 0158-01 | …e, 135 W.Va. 887, 65 S.E.2d 483 (1951); W.Va.Code, 48-2-28; ⟦W.Va.Code, 48-3-24⟧. Thus, to some extent, it may… | West Virginia | W. Va. Code |  | 48-3-24 |  | ✓ |
| 26 | CAP cal-4th 50, 0068-01 | …n). (Ornelas, supra, 4 Cal.4th 1095, 1100-1102, 1105-1108.) ⟦Civil Code section 846 ⟧is indeed broad in each of tho… | California | Cal. Civil Code | subject Civil | 846 |  | ✓ |
| 27 | CAP w-va 172, 0616-01 | …ntirely because she was not alive at the testatrix’s death. ⟦W.Va.Code 41-3-3 ⟧[1923] provides that the heirs… | West Virginia | W. Va. Code |  | 41-3-3 |  | ✓ |
| 28 | CAP w-va 163, 0361-01 | …idence — a result clearly not envisaged by the Legislature. ⟦W. Va. Code, 38-5-2 ⟧[1923] must, therefore, be int… | West Virginia | W. Va. Code |  | 38-5-2 |  | ✓ |
| 29 | CAP ne3d 27, 0802-01 | …al review, Lebamoff argued that the ATC’s interpretation of ⟦Indiana Code section 7.1-3-10-7 ⟧was unreasonable. On March 28,… | Indiana | Ind. Code |  | 7.1-3-10-7 |  | ✓ |
| 30 | CAP cal-4th 19, 1182-01 | …arsh result for the applicant in this matter, they construe ⟦Labor Code section 4702⟧, subdivision (a)(4), to provi… | California | Cal. Labor Code | subject Labor | 4702 |  | ✓ |
| 31 | CAP cal-4th 50, 1301-01 | … (Ewoldt, supra, 7 Cal.4th at p. 404.) “Rulings made under [⟦Evidence Code sections 1101 ⟧and 352] are reviewed for an a… | California | Cal. Evidence Code | subject Evidence | 1101 |  | ✓ |
| 32 | CAP cal-4th 50, 0068-01 | ….) The Ninth Circuit explained that it had found nothing in ⟦Civil Code section 846⟧’s language, or in the circums… | California | Cal. Civil Code | subject Civil | 846 |  | ✓ |
| 33 | CAP ne3d 27, 0795-01 | …nd life insurance premiums on their behalf, in violation of ⟦Indiana Code Section 36-12-2-21⟧, which states that “[a] membe… | Indiana | Ind. Code |  | 36-12-2-21 |  | ✓ |
| 34 | CAP w-va 172, 0689-01 | …limitation of actions to establish paternity established by ⟦W.Va.Code, 48-7-1 ⟧[1969] is unconstitutional und… | West Virginia | W. Va. Code |  | 48-7-1 |  | ✓ |
| 35 | CAP ne3d 27, 0306-01 | …ervs. Admin., 915 N.E.2d 169, 176 (Ind.Ct.App.2009) (citing ⟦Ind.Code § 4-21.5-5-11⟧). Instead, courts must defer … | Indiana | Ind. Code |  | 4-21.5-5-11 |  | ✓ |
| 36 | CAP cal-4th 19, 0332-01 | …permanent partial solution—to this problem. At its core, in ⟦Insurance Code section 11580.2⟧, the law states that, general… | California | Cal. Insurance Code | subject Insurance | 11580.2 |  | ✓ |
| 37 | CAP cal-4th 19, 0332-01 | …For, under the contractual arbitration law as it appears in ⟦Code of Civil Procedure section 1281.2⟧, it could reasonably have det… | California | Cal. Civil Procedure Code | subject Civil Procedure | 1281.2 |  | ✓ |
| 38 | CAP w-va 163, 0705-01 | …ght to two peremptory challenges. ...” The material part of ⟦W. Va. Code, 50-4-2 (1976)⟧, states: “Criminal actions sh… | West Virginia | W. Va. Code |  | 50-4-2 |  | ✓ |
| 39 | CAP cal-4th 19, 0629-01 | …t only on the amount of the defendant’s liability therefor. ⟦Civil Code section 3333.2 ⟧applies to the total of all no… | California | Cal. Civil Code | subject Civil | 3333.2 |  | ✓ |
| 40 | CAP ne3d 27, 1198-01 | … of limiting or extending their operation). The language of ⟦Indiana Code § 6-9-29-3 ⟧lends further support to this … | Indiana | Ind. Code |  | 6-9-29-3 |  | ✓ |
