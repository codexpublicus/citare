# Evidence: statutes cited section first

The hand-checked sample behind the #116 section of [`docs/laws-local.md`](../laws-local.md#section-first)
and [the `state-code-forms` entry](../divergences.md#state-code-forms): 40 statute citations
drawn at random (seed 116) from the 3,306 that citare finds with the #116 entries of
`data/laws-local.json` and didn't find before, over the CAP statute sample (12,337 opinions)
and citation-bench's stare.pub sample (1,000 opinions). Columns: the source opinion, the text
with the citation's full span in `⟦ ⟧`, and the fields citare gives. The last column is
whether the jurisdiction, code, section and pin cite are what the opinion cites.

All 40 have the right jurisdiction, code, section and pin cite. In the first run, before the
token stopped taking the space after the code, rows 5, 15 and 17 read a year in parentheses
after `C.R.S.` as the pin cite (`§ 8-4-122, C.R.S. (2017)`); the table is the run after that fix.

| Row | Source | Text | Jurisdiction | Code | Title, chapter, subject | Section | Pin cite | Right |
|---|---|---|---|---|---|---|---|---|
| 1 | CAP so2d 104, 0803-01 | …t.” (Italics ours.) The above rule has the effect of making ⟦Section 59.45, Fla.Stat., F.S.A.⟧, a part of the Florida Appell… | Florida | Fla. Stat. |  | 59.45 |  | ✓ |
| 2 | CAP p2d 646, 0948-01 | … set forth as an element of the crime charged. We disagree. ⟦Section 18-3-402(2), C.R.S. 1973 (1978 Repl.Vol. 8)⟧ enumerates three situations w… | Colorado | Colo. Rev. Stat. |  | 18-3-402 | (2) | ✓ |
| 3 | CAP sw2d 339, 0499-01 | …ima facie to have been issued for a valuable consideration, ⟦§ 401.024 RSMo 1949⟧, 21 V.A.M.S., and with § 401.… | Missouri | Mo. Rev. Stat. |  | 401.024 |  | ✓ |
| 4 | CAP wis-2d 101, 0296-01 | …imely filed a request for substitution of judge pursuant to ⟦sec. 801.68(1), Stats.⟧, the trial judge had no juris… | Wisconsin | Wis. Stat. |  | 801.68 | (1) | ✓ |
| 5 | CAP p3d 414, 0700-01 | … (three for willful violations), "and not after that time." ⟦§ 8-4-122, C.R.S. (2017)⟧. The certified question asks:… | Colorado | Colo. Rev. Stat. |  | 8-4-122 |  | ✓ |
| 6 | CAP wis-2d 102, 0079-01 | …ns restricting the specific events that revoke a will. See, ⟦sec. 853.11(4), Stats. (1979-80)⟧. Instead, the murderer is com… | Wisconsin | Wis. Stat. |  | 853.11 | (4) | ✓ |
| 7 | CAP so2d 528, 1279-01 | … fear in some other person that such violence is imminent,” ⟦Section 784.011, Florida Statutes⟧, and that there could be no w… | Florida | Fla. Stat. |  | 784.011 |  | ✓ |
| 8 | CAP wis-2d 101, 0396-01 | …, 96 Wis.2d 590, 602, 292 N.W.2d 622 (1980). The purpose of ⟦sec. 102.66(1), Stats. ⟧1975, is to direct payment or … | Wisconsin | Wis. Stat. |  | 102.66 | (1) | ✓ |
| 9 | CAP wis-2d 101, 0396-01 | …e shall be prescribed.” A similar provision is contained in ⟦sec. 990.06, Stats. ⟧I would hold that these statut… | Wisconsin | Wis. Stat. |  | 990.06 |  | ✓ |
| 10 | CAP so2d 528, 0066-01 | …degree, punishable as provided in s. 775.082 or s. 775.083. ⟦§ 843.02, Fla.Stat. (1985)⟧.… | Florida | Fla. Stat. |  | 843.02 |  | ✓ |
| 11 | CAP wis-2d 101, 0472-01 | …ther grants and shall be limited only by express language.” ⟦Sec. 61.34 (1), Stats. ⟧1979-80. The village board’s p… | Wisconsin | Wis. Stat. |  | 61.34 | (1) | ✓ |
| 12 | CAP wis-2d 102, 0049-01 | …nt) constitutes the acquisition of property contemplated in ⟦sec. 49.195, Stats. ⟧1975, which will trigger the o… | Wisconsin | Wis. Stat. |  | 49.195 |  | ✓ |
| 13 | CAP wis-2d 101, 0396-01 | … barred claim for occupational disease, can not occur under ⟦sec. 102.17 (4), Stats. ⟧1975. The department, therefor… | Wisconsin | Wis. Stat. |  | 102.17 | (4) | ✓ |
| 14 | CAP idaho 133, 0854-01 | …preliminary, procedural or intermediate agency action under ⟦section 67-5271(2), Idaho Code⟧, must be filed within twenty-… | Idaho | Idaho Code |  | 67-5271 | (2) | ✓ |
| 15 | CAP p3d 114, 0862-01 | … to property owners for negligent construction defects. See ⟦§ 13-20-802.5(4), C.R.S. (2004) (defining “construction professional” as “an architect, contractor, subcontractor, developer, builder, builder vendor, engineer, or inspector .... ” (emphasis added))⟧; see also § 13-20-803(4), C.R… | Colorado | Colo. Rev. Stat. |  | 13-20-802.5 | (4) | ✓ |
| 16 | CAP p2d 665, 1037-01 | …decision without sufficient evidence to do so. We disagree. ⟦Section 8-74-104, C.R.S.1973 (1982 Cum. Supp.)⟧ provides as follows: “The Com… | Colorado | Colo. Rev. Stat. |  | 8-74-104 |  | ✓ |
| 17 | CAP p3d 257, 0201-01 | …risdiction & Enforcement Act § 206(a), 9 U.L.A. 680 (1999); ⟦§ 14-13-206(1), C.R.S. (2010)⟧. Nowhere, however, does the U… | Colorado | Colo. Rev. Stat. |  | 14-13-206 | (1) | ✓ |
| 18 | CAP wis-2d 102, 0079-01 | … of gross negligence (which is incorporated by reference to ⟦sec. 940.06(2), Stats.⟧). In Hart v. State, the court… | Wisconsin | Wis. Stat. |  | 940.06 | (2) | ✓ |
| 19 | CAP wis-2d 101, 0090-01 | …ive purchasers were incomplete — misleading in violation of ⟦sec. 628.34 (1), Stats.⟧, in that these materials did … | Wisconsin | Wis. Stat. |  | 628.34 | (1) | ✓ |
| 20 | CAP ohio-app 64, 0251-01 | …re involved, computed according to our statutory provision (⟦Section 10216, General Code⟧), shows that the period began… | Ohio | Ohio Gen. Code |  | 10216 |  | ✓ |
| 21 | CAP wis-2d 101, 0012-01 | …ioned by the consideration of ‘the interest of the public.’ ⟦Sec. 48.01(3), Stats. ⟧The interest of the public is … | Wisconsin | Wis. Stat. |  | 48.01 | (3) | ✓ |
| 22 | CAP ohio-app 37, 0571-01 | …sisted of two counts, differing only in the intent charged. ⟦Section 13437-6, General Code (113 Ohio Laws, 164)⟧, provides that under the refo… | Ohio | Ohio Gen. Code |  | 13437-6 |  | ✓ |
| 23 | CAP ohio-app-2d 1, 0143-01 | …iomyelitis, smallpox, pertussis, and tetanus as provided by ⟦Section 3313.671, Revised Code⟧; but that respondent board re… | Ohio | Ohio Rev. Code Ann. |  | 3313.671 |  | ✓ |
| 24 | CAP ohio-app-2d 1, 0001-01 | … the syllabus reads as follows: “1. Under the provisions of ⟦Section 1639-16, General Code ⟧(Section 2151.23, Revised Code… | Ohio | Ohio Gen. Code |  | 1639-16 |  | ✓ |
| 25 | CAP wis-2d 101, 0315-01 | …nd vehicle contravened the terms of the “two-vehicle rule,” ⟦sec. 49.19(4) (em), Stats.⟧, and disqualified him as an a… | Wisconsin | Wis. Stat. |  | 49.19 | (4) (em) | ✓ |
| 26 | CAP wis-2d 101, 0472-01 | …equirement, neither of which are applicable to Jessen. See: ⟦sec. 165.85(4) (a), Stats.⟧ Analysis of the Legislative R… | Wisconsin | Wis. Stat. |  | 165.85 | (4) (a) | ✓ |
| 27 | CAP p2d 646, 0375-01 | …. Section 18-3-102(l)(b), C.R.S.1973 (1978 Repl. Vol. 8). . ⟦Section 18-4-301, C.R.S.1973 (1978 Repl. Vol. 8)⟧. . Sections 18-2-101 and 18^-… | Colorado | Colo. Rev. Stat. |  | 18-4-301 |  | ✓ |
| 28 | CAP wis-2d 101, 0670-01 | ….05(1) (a), Stats. Service must conform to the standards of ⟦sec. 801.11, Stats. ⟧If the owner or a lienholder c… | Wisconsin | Wis. Stat. |  | 801.11 |  | ✓ |
| 29 | CAP so2d 171, 0568-01 | …nstitution and the statutes, is buttressed by the fact that ⟦§ 731.34, Fla. Stat., F.S.A. ⟧grants dower of “ * * * one th… | Florida | Fla. Stat. |  | 731.34 |  | ✓ |
| 30 | CAP ohio-app 64, 0105-01 | …le anything of value shall be punished as therein provided. ⟦Section 13066, General Code⟧, provides, in substance, that… | Ohio | Ohio Gen. Code |  | 13066 |  | ✓ |
| 31 | CAP ohio-app 64, 0300-01 | …child legally adopted prior to the injury.” (Italics ours.) ⟦Section 10503-15, General Code⟧, reads: “When by a woman a ma… | Ohio | Ohio Gen. Code |  | 10503-15 |  | ✓ |
| 32 | CAP ohio-app-2d 1, 0330-01 | …e made such proceedings necessary.” The second paragraph of ⟦Section 3903.09, Revised Code⟧, explicitly requires that, if… | Ohio | Ohio Rev. Code Ann. |  | 3903.09 |  | ✓ |
| 33 | CAP idaho 138, 0470-01 | …ohol, drugs or other intoxicating substances as provided in ⟦section 67-7035, Idaho Code⟧; or (iv) Any criminal homicid… | Idaho | Idaho Code |  | 67-7035 |  | ✓ |
| 34 | CAP so2d 431, 0685-01 | …ppellee’s motion for an appellate attorney’s fee is DENIED. ⟦§ 440.34, Fla.Stat. (1979)⟧. SHIVERS and WIGGINTON, JJ., … | Florida | Fla. Stat. |  | 440.34 |  | ✓ |
| 35 | CAP wis-2d 101, 0472-01 | …6 felony counts of misconduct in public office, contrary to ⟦sec. 946.12 (4), Stats. ⟧1973, after being found guilty… | Wisconsin | Wis. Stat. |  | 946.12 | (4) | ✓ |
| 36 | CAP ohio-app 64, 0271-01 | … this, county unjess the right to bring it was conferred by ⟦Section 11271, General Code⟧. That section provides that a… | Ohio | Ohio Gen. Code |  | 11271 |  | ✓ |
| 37 | CAP ohio-app 64, 0084-01 | … this case that the Legislature did not by the amendment of ⟦Section 11656, General Code⟧, intend to disturb in any way… | Ohio | Ohio Gen. Code |  | 11656 |  | ✓ |
| 38 | CAP so2d 171, 0630-01 | …Second), Trusts §§ 184, 224, illustration e (1959). . Under ⟦§ 732.55, Fla.Stat., F.S.A.⟧, due to obvious conflict in i… | Florida | Fla. Stat. |  | 732.55 |  | ✓ |
| 39 | CAP so2d 528, 1279-01 | …d abetting another in the commission of a crime pursuant to ⟦Section 777.011, Florida Statutes⟧, “it is necessary that he [th… | Florida | Fla. Stat. |  | 777.011 |  | ✓ |
| 40 | CAP so2d 179, 0628-01 | … and order departed from the essential requirements of law. ⟦Section 561.60, Florida Statutes, F.S.A.⟧, provides: “Each and every ta… | Florida | Fla. Stat. |  | 561.60 |  | ✓ |
