# Evidence: the longer section pin cites (#48)

The 117 pin cites that stopped early and the 46 that were missed in the
[`id-section-pin` study](id-section-pin.md), re-run through the grammar from #48 on the same
opinions. "Before" is citare at `f94fa63` (v0.5.1), "now" is #48. Each answer was judged by
reading the text after the match: "right" means the pin cite is all of the section cite and
nothing after it.

| | Stopped early (117) | Missed (46) |
|---|---|---|
| Right now | 93 | 33 |
| Still stops early | 24 | 3 |
| Still missed | 0 | 9 |
| Wrong | 0 | 1 |

What's left:

- **A capitalized `Id.` with the `§` on the next line** (6 missed, all CourtListener):
  `Id.\n\n§ 232.103(4)(a), (d).` In CAP's digests the same layout ends a paragraph, and 10 of
  the 10 CAP matches with it aren't pin cites (`Id.\n§ 12 makes all the trustees liable`), so
  citare keeps a line break between `Id.` and `§` as a stop. After a lowercase `id.` or after
  `at` it isn't one. Of the 28 missed rows with a line break before or after the `§`, 21 are
  right now.
- **OCR damage** (3 missed, 13 stop early): `§ 11'005-A`, `§-103(a)`, `§ 76‐2‐\n202`,
  `§ 2401 e(b)` (a space inside the section number, 5 rows), ``§ 355(]`)(2)`` (3 rows),
  `§ 5.6d(/)`, `§ 1.381(c)(4)-l(e)(l)`, `emt. b`.
- **Lists whose items have their own parentheticals, pages or letters** (6 stop early):
  `§§ 2007.021 (suit), .022 (administrative proceeding)`, `§§ 1704.101 (board's powers),
  1704.102`, `§§ 331(u) (1994); 342(f)`, `§§ 1692f, f(l)`, `§§ 4 & 5 (Sherman Act), 9 & 10`,
  `§ 108 p. 389, § 104`.
- **Other designations** (5 stop early): `, Reporter's Note`, `, h`, `, also § 8`,
  `, Q & A-2(b)(2)(viii)` (2 rows).
- **A page break** (1 wrong): `Id. §\n\n 4\n\f1926.1101(a)(3)` takes the page number `4`.
- **Section `III, ¶ D(1)`** (3 stop early at `§ III`).

No pin cite that was right before changed, except to grow where the old answer had been
counted as right but stopped early (`§ 33-501 (15)`, `§ 121 cmt. i`, `§ 1681h(e), and
§ 1681t(b)`; 6 rows). Resolution changed in 10 matches, all to the right answer: 9 id.
citations in CourtListener that follow one whose pin cite was missed now resolve to the code
or regulation (`2 U.S.C. § 1402`, `29 C.F.R. § 1926.1101`, `5 U.S.C. §§ 3345`, `Tex. Gov't
Code Ann. § 2001.033`), because a `§` that an earlier pin cite missed or left out no longer breaks the id. chain; and
`id.\n § 156.101` (Texas Family Code) no longer resolves to the case before it.

Columns: the row id from the study; what citare did before; the first 80 characters from the
`Id.`; the pin cite before and now (`⏎` is a line break, `⍽` a non-breaking space); and the
judgment now.

| Row | Was | Text after the match | Pin cite before | Pin cite now | Now |
|---|---|---|---|---|---|
| cap0000 | missed | id. § l-618.3(a). The following month, Hairston amended the June 15 complaint to… | — | `§ l-618.3(a)` | right |
| cap0015 | stops early | id. §§ 22-501, -3202, attempted robbery while armed, id. §§ 22-2902, -3202, seco… | `§§ 22-501` | `§§ 22-501, -3202` | right |
| cap0016 | stops early | id. §§ 22-2902, -3202, second-degree murder while armed, id. §§ 22-2403, -3202, … | `§§ 22-2902` | `§§ 22-2902, -3202` | right |
| cap0017 | stops early | id. §§ 22-2403, -3202, and possession of a prohibited weapon, id. § 22-3214(a). … | `§§ 22-2403` | `§§ 22-2403, -3202` | right |
| cap0034 | stops early | Id. § 6105(a.l)(l). Importantly, for purposes of the present case, the subsectio… | `§ 6105` | `§ 6105(a.l)(l)` | right |
| cap0035 | stops early | id. § 6105(a.l)(l).⏎.The trial record presented to the Court is not clear as to … | `§ 6105` | `§ 6105(a.l)(l)` | right |
| cap0037 | stops early | Id., § 3 — 201(b).⏎. Under D.C.Code § 28:3-110(d) (2012 Repl.), ”[i]f an instrum… | `§ 3` | `§ 3 — 201(b)` | right |
| cap0041 | stops early | id., § 9799.14(c)(1.3), was required to register for 25 years, id., § 9799.15(a)… | `§ 9799.14(c)` | `§ 9799.14(c)(1.3)` | right |
| cap0048 | stops early | id. § 9799.15(a.l) (providing credit for time registered prior to -December 20, … | `§ 9799.15` | `§ 9799.15(a.l)` | right |
| cap0062 | stops early | Id. § 3, subd. 6, as amd. by Laws of 1895, chap. 672 [now Gen. Corp. Law (Consol… | `§ 3` | `§ 3, subd. 6` | right |
| cap0095 | stops early | Id., § 5.34a(1), h.⏎The Court previously acknowledged that it was “ill-equipped … | `§ 5.34a(1)` | `§ 5.34a(1)` | stops early |
| cap0096 | stops early | Id., § 5.6d(/).⏎. The Supreme Court previously rejected an appellant’s attempt t… | `§ 5.6d` | `§ 5.6d` | stops early |
| cap0098 | stops early | id., § 5.30e(1-2). An employee, however, must receive pre-deprivation minimal pr… | `§ 5.30e` | `§ 5.30e(1-2)` | right |
| cap0102 | missed | Id. at § IV(B)(6)(j). The Chapter Manager, as defined by the LGA, is specificall… | — | `at § IV(B)(6)(j)` | right |
| cap0111 | stops early | id. § 805.416(a)( 1 )(ii). The homebuyers were responsible for maintenance of th… | `§ 805.416(a)` | `§ 805.416(a)( 1 )(ii)` | right |
| cap0129 | stops early | Id. §§ 216.103 to 216.111.⏎Similarly in 1984, the Department of the Interior pro… | `§§ 216.103` | `§§ 216.103 to 216.111` | right |
| cap0181 | stops early | id., § 5304, subd. (a).) A person may be recommitted under section 5300 if durin… | `§ 5304` | `§ 5304, subd. (a)` | right |
| cap0188 | stops early | id., §§ 2962, subd. (d)(1), 2966, subd. (c), 2970; see generally id., § 2960.) A… | `§§ 2962` | `§§ 2962, subd. (d)(1), 2966, subd. (c), 2970` | right |
| cap0190 | stops early | id., § 2972, subd. (c).)⏎Moreover, the treatment of MDO’s already has considerab… | `§ 2972` | `§ 2972, subd. (c)` | right |
| cap0214 | stops early | id., § 1343, subd. (e)(1), and Ins. Code, § 10290 et seq.)⏎The court in Lemon v.… | `§ 1343` | `§ 1343, subd. (e)(1)` | right |
| cap0228 | stops early | id., § 224.1, subd. (a); 25 U.S.C. § 1903(4)), thus leaving cases not involving … | `§ 224.1` | `§ 224.1, subd. (a)` | right |
| cap0232 | stops early | id., §§ 352, subd. (b), 366.26, subd. (c)(3) [limits on continuances]; rule 5.55… | `§§ 352` | `§§ 352, subd. (b), 366.26, subd. (c)(3)` | right |
| cap0238 | stops early | id., § 1150, subd. (a) [defining the term “state employee” to mean “all persons … | `§ 1150` | `§ 1150, subd. (a)` | right |
| cap0241 | stops early | id., § 245.5, subd. (b) [defining “employer” for purposes of the Healthy Workpla… | `§ 245.5` | `§ 245.5, subd. (b)` | right |
| cap0252 | stops early | id., §§21080, subd. (a), 21100, subd. (a).) The purpose of the EIR is “to provid… | `§§21080` | `§§21080, subd. (a), 21100, subd. (a)` | right |
| cap0257 | stops early | id., § 21080.3, subd. (a) [“Prior to determining whether a negative declaration … | `§ 21080.3` | `§ 21080.3, subd. (a)` | right |
| cap0264 | stops early | id., § 1300.75.4.3, subd. (e).)⏎In addition to imposing these reporting requirem… | `§ 1300.75.4.3` | `§ 1300.75.4.3, subd. (e)` | right |
| cap0268 | stops early | id., § 1300.75.4.2, subd. (b)(1)(B), (2)(A).)⏎Health care service plans have a l… | `§ 1300.75.4.2` | `§ 1300.75.4.2, subd. (b)(1)(B), (2)(A)` | right |
| cap0307 | stops early | id., § 60300, subd. (j).) If so, CCS proposes a therapy assessment, obtains the … | `§ 60300` | `§ 60300, subd. (j)` | right |
| cap0316 | stops early | id., § 452, subd. (c); Conservatorship of Pamela J. (2005) 133 Cal.App.4th 807, … | `§ 452` | `§ 452, subd. (c)` | right |
| cap0330 | stops early | Id. at § 780.4, § 780.5.⏎B. Facts Re Sam Foos⏎Plaintiff Foos received two farm-s… | `at § 780.4` | `at § 780.4, § 780.5` | right |
| cap0351 | stops early | id., §§ 109-111 and § 113 as amended in the 1948 Sup., p. 111. Indeed, the defen… | `§§ 109-111` | `§§ 109-111 and § 113` | right |
| cap0352 | stops early | id., §§ 58 through 61, pp. 393-426. The duty does not, however, extend to uncont… | `§§ 58` | `§§ 58 through 61` | right |
| cap0354 | stops early | Id., § 157, comment (a), p. 373.⏎We conclude that the record conclusively demons… | `§ 157` | `§ 157, comment (a)` | right |
| cap0355 | stops early | Id. § 108 p. 389, § 104.⏎The defendant further claims that failure on the part o… | `§ 108` | `§ 108` | stops early |
| cap0359 | stops early | Id. § 381, comment a.⏎The trustees argue further that it is hardly to he suppose… | `§ 381` | `§ 381, comment a` | right |
| cap0364 | stops early | Id., § 2481, subd. 11. It seems to me clear, therefore, that the Surrogate’s cou… | `§ 2481` | `§ 2481, subd. 11` | right |
| cap0387 | missed | Id. at § III, ¶£(1). If the Commissioner cannot make a specific finding regardin… | — | `at § III` | stops early |
| cap0388 | missed | Id. at § III, ¶0(2). The de minimis rule is intended to prevent an agency from d… | — | `at § III` | stops early |
| cap0389 | missed | Id. at § III, ¶ D(l). As argued by plaintiff here, the de minimis issue is not s… | — | `at § III` | stops early |
| cap0411 | stops early | Id. § 23.03[1]. The commonly used factors include: the degree of resemblance bet… | `§ 23.03` | `§ 23.03[1]` | right |
| cap0412 | stops early | Id. § 23.15[9]. In fact, under certain circumstances, use of a disclaimer may ev… | `§ 23.15` | `§ 23.15[9]` | right |
| cap0416 | stops early | Id. § 1677(7)(C) (iii).⏎To find whether threat of material injury caused by LTFV… | `§ 1677(7)(C)` | `§ 1677(7)(C) (iii)` | right |
| cap0452 | stops early | Id. § 924(e) (2) (B) (i) — (ii).⏎To determine whether a state offense falls with… | `§ 924(e)` | `§ 924(e) (2) (B) (i) — (ii)` | right |
| cap0469 | stops early | Id. § 2, art. 2; The Caroline v. U. S., 7 Cranch [11 U. S.] 496; The Anne v. U. … | `§ 2` | `§ 2, art. 2` | right |
| cap0478 | stops early | Id. § 1.381(c)(4)-l(e)(l). Here the acquisition was completed on December 31, 19… | `§ 1.381(c)` | `§ 1.381(c)` | stops early |
| cap0479 | stops early | Id. § 1.381 (c)(4) — 1 (e)(2)(ii). Under these provisions RCA would have been en… | `§ 1.381` | `§ 1.381 (c)` | stops early |
| cap0514 | stops early | Id., § 2, cl. 1. Congress has generally authorized this Court to exercise remova… | `§ 2` | `§ 2, cl. 1` | right |
| cap0515 | stops early | id. §§ 1692f, f(l).⏎Although they assert multiple violations of the FDCPA, plain… | `§§ 1692f` | `§§ 1692f` | stops early |
| cap0546 | stops early | Id. §§ 3084.1(b); 3084.7(c)(3); Barry v. Ratelle, 985 F.Supp. 1235, 1237-38 (S.D… | `§§ 3084.1(b)` | `§§ 3084.1(b); 3084.7(c)(3)` | right |
| cap0572 | stops early | Id. § 2B1.1, cmt. (n.2) (2002). “[Ajctual loss” is “the reasonably foreseeable p… | `§ 2B1.1` | `§ 2B1.1, cmt. (n.2)` | right |
| cap0573 | stops early | Id. § 2B1.1, cmt. (n.2(A)(I) and (ii)). In situations where the losses occasione… | `§ 2B1.1` | `§ 2B1.1, cmt. (n.2(A)(I) and (ii))` | right |
| cap0574 | stops early | Id. § 2B1.1, cmt. (n.2(C)). Such estimates “need not be determined with precisio… | `§ 2B1.1` | `§ 2B1.1, cmt. (n.2(C))` | right |
| cap0591 | stops early | id. at §§ 2101(a)(1); 2101(a)(3)(B)(i)(I). Applying § 2102(d) to this case, enou… | `at §§ 2101(a)(1)` | `at §§ 2101(a)(1); 2101(a)(3)(B)(i)(I)` | right |
| cap0624 | stops early | id. § 1.148 — 1(b) (defining “reasonable expectations”). Therefore, the question… | `§ 1.148` | `§ 1.148 — 1(b)` | right |
| cap0637 | stops early | Id. § 2L1.2 application n. 5.⏎We must now determine whether ReinaRodriguez’s pri… | `§ 2L1.2` | `§ 2L1.2 application n. 5` | right |
| cap0640 | stops early | Id. § 2L1.2 application n. 5. However, under Utah law, one can be convicted of a… | `§ 2L1.2` | `§ 2L1.2 application n. 5` | right |
| cap0646 | stops early | id. §§ 4 & 5 (Sherman Act), 9 & 10 (Wilson Tariff Act).⏎. The district court’s f… | `§§ 4 & 5` | `§§ 4 & 5` | stops early |
| cap0651 | stops early | Id. § 2401 e(b). The maximum yearly cash benefit for any producer under the prog… | `§ 2401` | `§ 2401` | stops early |
| cap0653 | stops early | Id. § 2401 e(a)(2).⏎Following the enactment of the 2002 Act, the Secretary of Ag… | `§ 2401` | `§ 2401` | stops early |
| cap0657 | stops early | id. § 2401 a(c)(2), the amount of the certified commodity covered by the produce… | `§ 2401` | `§ 2401` | stops early |
| cap0658 | stops early | id. §§ 2401 e(a)(l)(A), 2401 e(b)(l)(B), and the information and technical assis… | `§§ 2401` | `§§ 2401` | stops early |
| cap0659 | stops early | id. § 2401 e(a)(l)(D). In this setting, where Congress expressly referred to the… | `§ 2401` | `§ 2401` | stops early |
| cap0672 | stops early | Id. § 152 cmt. a.⏎Generally, where the parties to a transaction fail to recogniz… | `§ 152` | `§ 152 cmt. a` | right |
| cap0686 | stops early | Id. § 14 — 501(b)(5). Faculty members as well as staff contributed to the error … | `§ 14` | `§ 14 — 501(b)(5)` | right |
| cap0688 | stops early | Id. at § 41.02[4]. Here, since dismissal would be appropriate under Rule 41(a)(l… | `at § 41.02` | `at § 41.02[4]` | right |
| cap0692 | stops early | Id. § 314A, comment b. Moreover, the drafters expressed their view that the “law… | `§ 314A` | `§ 314A, comment b` | right |
| cap0707 | stops early | Id. § 147, 154, 156 to 159. 1 Stair, Inst, by Brodie, B. 1, tit. 12, § 16. 3 Chi… | `§ 147, 154, 156` | `§ 147, 154, 156 to 159` | right |
| cap0723 | stops early | id. at § 335 comment a. In other words, the jury should have been alerted, as Ke… | `at § 335` | `at § 335 comment a` | right |
| cap0757 | stops early | Id., § 458 comment d.⏎Id., § 457 and § 459.⏎Id., § 458 Comment h.⏎"Acquiescence … | `§ 458` | `§ 458 comment d` | right |
| cap0758 | stops early | Id., § 457 and § 459.⏎Id., § 458 Comment h.⏎"Acquiescence is here used in its or… | `§ 457` | `§ 457 and § 459` | right |
| cap0759 | stops early | Id., § 458 Comment h.⏎"Acquiescence is here used in its ordinary sense; it does … | `§ 458` | `§ 458 Comment h` | right |
| cap0760 | stops early | Id., § 458 comment h. See also, 2 Thompson on Real Property. § 340 (1961) which … | `§ 458` | `§ 458 comment h` | right |
| cap0793 | stops early | Id., § 686, subd. 2. His punishment, therefore, will not exceed that to which he… | `§ 686` | `§ 686, subd. 2` | right |
| cap0812 | stops early | id. § 26-516 [a] [“Where the amount of rent set forth in the annual rent registr… | `§ 26-516` | `§ 26-516 [a]` | right |
| cap0813 | stops early | id. § 26-516 [a] [“Where the amount of rent set forth in the annual rent registr… | `§ 26-516` | `§ 26-516 [a]` | right |
| cap0814 | stops early | id. § 10-603 [a]) and personally appear at the NYPD Gun Offender Monitoring Unit… | `§ 10-603` | `§ 10-603 [a]` | right |
| cap0815 | stops early | id. § 10-603 [d]). Failure to comply with GORA requirements is a misdemeanor pun… | `§ 10-603` | `§ 10-603 [d]` | right |
| cap0831 | stops early | id. § 24 emt. b (explaining what constitutes a transaction). Accordingly, "res j… | `§ 24` | `§ 24` | stops early |
| cap0832 | missed | Id. §l(h), 75 P.8. §624.1 (h).⏎The correctness of appellant’s second and third a… | — | `§l(h)` | right |
| cap0833 | stops early | Id., §501 (b), 17 P.S. §211.501 (b); or, (c) the appellee has waived an objectio… | `§501` | `§501 (b)` | right |
| cap0839 | stops early | id., § 28, Reporter's Note, at 284 (noting "the pervasive importance of reviewab… | `§ 28` | `§ 28` | stops early |
| cap0840 | stops early | id., § 28, Comment j, at 283 (Issue preclusion may be denied where it is "eviden… | `§ 28` | `§ 28, Comment j` | right |
| cap0843 | stops early | id., § 25.3, and Comment a (describing contingent future interests subject to di… | `§ 25.3` | `§ 25.3, and Comment a` | right |
| cap0877 | stops early | id. at § 675, comment (c) (adapting this comment to wrongful civil proceedings).… | `at § 675` | `at § 675, comment (c)` | right |
| cap0878 | stops early | Id. § 675, comment (d).⏎The evidence before the Court shows that Mussler had pro… | `§ 675` | `§ 675, comment (d)` | right |
| cap0890 | stops early | id. §§ 8.01-.07.⏎. Mendenhall, 77 S.W.3d at 817-18; Tones, 585 S.W.2d at 748-50;… | `§§ 8` | `§§ 8.01-.07` | right |
| cap0899 | stops early | id. §§ 2007.023(b), .024(a). In response to such an order, the governmental enti… | `§§ 2007.023(b)` | `§§ 2007.023(b), .024(a)` | right |
| cap0901 | stops early | id. §§ 2007.004(a), .024(c).⏎PRPRPA sets forth two means through which a “privat… | `§§ 2007.004(a)` | `§§ 2007.004(a), .024(c)` | right |
| cap0902 | stops early | Id. §§ 2007.021 (suit), .022 (administrative proceeding). The former “must be fi… | `§§ 2007.021` | `§§ 2007.021` | stops early |
| cap0905 | stops early | Id. §§ 2007.021(b), .022(b). Furthermore, a person “who has exhausted all admini… | `§§ 2007.021(b)` | `§§ 2007.021(b), .022(b)` | right |
| cap0907 | stops early | id. §§ 2007.022, .025(b). Similarly, BP’s pleadings do not demonstrate that it f… | `§§ 2007.022` | `§§ 2007.022, .025(b)` | right |
| cap0931 | stops early | id. § 16.012(d-l) (mentioning "limitations period under this section”). Only sec… | `§ 16.012` | `§ 16.012(d-l)` | right |
| cap0947 | stops early | Id., § 3, subd. 3, also § 8. The association, when formed, has no other residenc… | `§ 3` | `§ 3, subd. 3` | stops early |
| cap0984 | stops early | id. § 75.1704 to .1704-2, 75.-1707, 75.1714, and 75.1714-3 (escapeway standards)… | `§ 75.1704` | `§ 75.1704 to .1704-2, 75.-1707, 75.1714, and 75.1714-3` | right |
| cap0985 | stops early | id. §§ 75.-316, 75.316-1. An MSHA official testified that the agency would not r… | `§§ 75` | `§§ 75.-316, 75.316-1` | right |
| cap0989 | stops early | id., § 199, illus. 1 (1958); cf. Allred v. Hinkley, 8 Utah 2d 73, 77 at n. 2, 32… | `§ 199` | `§ 199, illus. 1` | right |
| cl0005 | missed | Id.⏎⏎§ 232.103(4)(a), (d).⏎⏎ IV. Conclusion.⏎⏎ Under the statutory standard appl… | — | — | missed |
| cl0006 | missed | Id.⏎⏎§ 232.103(4)(a), (d).⏎⏎ IV. Conclusion.⏎⏎ Under the statutory standard appl… | — | — | missed |
| cl0014 | missed | id.⏎⏎§ 1408(a), and that federal district courts have jurisdiction over such a c… | — | `§ 1408(a)` | right |
| cl0016 | missed | Id. §⏎401. Pursuant to the Charter, the mayor has appointment and removal power,… | — | `§⏎401` | right |
| cl0021 | stops early | Id. §§ 23-602(A)(8); 23-603(A)(4). The Investigating Officer works independently… | `§§ 23-602(A)(8)` | `§§ 23-602(A)(8); 23-603(A)(4)` | right |
| cl0024 | missed | id.⏎ § 156.101 (setting forth grounds for modification of order establishing⏎ co… | — | `§ 156.101` | right |
| cl0026 | missed | id.⏎⏎§ 171.098(a)(1) (TAA); see also Ellis v. Schlimmer, 337 S.W.3d 860, 862 (Te… | — | `§ 171.098(a)(1)` | right |
| cl0031 | missed | id.⏎⏎§ 13.00(I)(8) (stating that “[u]nresectable means surgery . . . did not com… | — | `§ 13.00(I)(8)` | right |
| cl0046 | stops early | Id. § 3222.1(b)(10)(i—iii). Section 3222.1(b)(ll) affords health professionals a… | `§ 3222.1(b)(10)` | `§ 3222.1(b)(10)(i—iii)` | right |
| cl0064 | missed | id. § 11'005-A. By contrast, as we recognized in Robinson II, under Act 13, in i… | — | — | missed |
| cl0069 | missed | id. § 76‐2‐⏎202; id. § 76‐3‐203.1 (Supp. 2011), and riot, a third degree felony,… | — | — | missed |
| cl0070 | missed | id. § 76‐3‐203.1 (Supp. 2011), and riot, a third degree felony, see id. § 76‐9‐1… | — | `§ 76‐3‐203.1` | right |
| cl0071 | missed | id. § 76‐9‐101⏎(2008).1 We affirm.⏎⏎⏎ 1⏎ Since the events giving rise to the cha… | — | `§ 76‐9‐101` | right |
| cl0072 | missed | id. § 76‐5‐103 (Supp. 2011). Because the legislature has not made⏎any substantiv… | — | `§ 76‐5‐103` | right |
| cl0073 | missed | Id. § 76‐9‐101(1)(b) (2008).⏎⏎⏎⏎⏎20100668‐CA 6⏎affiliation is particularly rele… | — | `§ 76‐9‐101(1)(b)` | right |
| cl0079 | stops early | id. §§ 11, 11-a, 22—23-c; see also Matter of City of New York [Rudnick], 25 NY2d… | `§§ 11, 11-a` | `§§ 11, 11-a, 22—23-c` | right |
| cl0084 | missed | Id. §⏎⏎ 4⏎1926.1101(a)(3). The regulation requires building owners 1 to⏎test fo… | — | `§⏎⏎ 4` | wrong |
| cl0086 | missed | id.⏎§ 1926.1101(k)(2)(ii), provisions are those at issue in this case.⏎Pursuant … | — | `§ 1926.1101(k)(2)(ii)` | right |
| cl0099 | missed | id.⏎⏎§ 3348(b)(2), “required by statute,” id. § 3348(a)(2)(A)(ii), “or . . . reg… | — | `§ 3348(b)(2)` | right |
| cl0103 | missed | id.⏎⏎§ 3348(a)(2)(B)(i)(II). Indian acknowledgment decisions did not fall within… | — | `§ 3348(a)(2)(B)(i)(II)` | right |
| cl0109 | stops early | Id. §§ 404.510a; 404.512. Individual financial circumstances "are not material t… | `§§ 404.510a` | `§§ 404.510a; 404.512` | right |
| cl0112 | stops early | id. at § 24 cmt. c, it need not be evidenced by words, whether written or spoken… | `at § 24` | `at § 24 cmt. c` | right |
| cl0113 | stops early | Id. at §§ 4 cmt. a, 24(1). In litigation such as that at bar, all circumstances … | `at §§ 4` | `at §§ 4 cmt. a, 24(1)` | right |
| cl0114 | stops early | id. at § 24 cmt. b. The Court concludes from the entire evidentiary record of th… | `at § 24` | `at § 24 cmt. b` | right |
| cl0122 | missed | id. §§⏎⏎157.167, 157.263. Therefore, the trial court was required to award Speck… | — | `§§⏎⏎157.167, 157.263` | right |
| cl0147 | stops early | id., §§ 451.910, .914, .917, and the Uniform Securities Act, id., §§ 451.501, .8… | `§§ 451.910` | `§§ 451.910, .914, .917` | right |
| cl0148 | stops early | id., §§ 451.501, .808, violated the Commerce and Supremacy Clauses of the United… | `§§ 451.501` | `§§ 451.501, .808` | right |
| cl0172 | stops early | id. § 152, comment m. Cf. Vaughan v. Shirey, 212 Ark. 935, 208 S.W.2d 441 (1948)… | `§ 152` | `§ 152, comment m` | right |
| cl0174 | stops early | Id. § 2L1.2 cmt. n. 1(B)(iii). The Government concedes that Defendant's convicti… | `§ 2L1.2` | `§ 2L1.2 cmt. n. 1(B)(iii)` | right |
| cl0179 | missed | Id.⏎⏎§ 15.393(2)(a)(1)–(3), (b)(1)–(2).⏎⏎ To obtain a tax credit, the taxpayer m… | — | — | missed |
| cl0183 | missed | Id.⏎§ 7407(d)(1).⏎⏎ A⏎⏎ The CAA requires states to address nonattainment areas⏎b… | — | — | missed |
| cl0185 | missed | Id.⏎§ 7410(a)(2)(A). The CAA provides a private right of action⏎for citizens to … | — | — | missed |
| cl0205 | missed | Id. at § (i)(2), (3). Any amended complaint must comply with the limitations imp… | — | `at § (i)(2), (3)` | right |
| cl0218 | missed | Id. §⍽29.03(a )( 2). ⍽ The portion of appellant’s indictment⏎concerning the dead… | — | `§⍽29.03(a )( 2)` | right |
| cl0219 | missed | id. §⍽46.01(3) (Vernon Supp. 2011). ⍽ That definition, however,⏎is limited to st… | — | `§⍽46.01(3)` | right |
| cl0220 | missed | Id. §⍽46.01. As the State points out, the Court of Criminal Appeals has⏎held tha… | — | `§⍽46.01` | right |
| cl0228 | missed | id. §⏎21.11 (Vernon Supp. 2009). The jury assessed punishment at ten years' conf… | — | `§⏎21.11` | right |
| cl0268 | stops early | Id. § 3553(a)(1), (3)-(7).⏎ 2⏎ Case: 14-11339 Date Filed: 10/28/2014 Page: 3 of… | `§ 3553(a)(1)` | `§ 3553(a)(1), (3)-(7)` | right |
| cl0270 | missed | id. §⏎⏎232.116(1)(f)(2), (4), (h)(2), (4); see A.M., 843 N.W.2d at 111 (describi… | — | `§⏎⏎232.116(1)(f)(2), (4), (h)(2), (4)` | right |
| cl0276 | stops early | Id. § 1.411(d)-4, Q & A-2(b)(2)(viii). The only sensible construction of this la… | `§ 1.411` | `§ 1.411(d)-4` | stops early |
| cl0277 | stops early | Id. § 1.411(d)-4, Q & A-3(a). In the plaintiff's view, his situation is analogou… | `§ 1.411` | `§ 1.411(d)-4` | stops early |
| cl0288 | missed | id., § (h), App. (CA1) 787-788; House Report 5. As a result, the coal operators'… | — | `§ (h)` | right |
| cl0317 | missed | id.⏎§⍽ 20109(d)(2)(A)⍽ (incorporating⍽ by⍽ reference⍽ the⍽ rules⍽ and⏎procedures… | — | `§⍽ 20109(d)(2)(A)` | right |
| cl0320 | missed | Id. at⏎⏎§ 2149(c).⏎⏎ B. The Incident⏎⏎ The following allegations are from Big Ca… | — | `at⏎⏎§ 2149(c)` | right |
| cl0321 | missed | id. at⏎⏎§ 7606. The Court rejected the government’s contention that the statute⏎… | — | `at⏎⏎§ 7606` | right |
| cl0326 | stops early | id., § 261[1][a]). Tax Law, § 261 also provides as follows: "with respect to the… | `§ 261` | `§ 261[1][a]` | right |
| cl0327 | missed | Id. §-103(a); see also KSR Int’l Co. v. Teleflex Inc., 550 U.S. 398, 406-07, 127… | — | — | missed |
| cl0329 | stops early | id., § 2034.260, subd. (b)(1).)⏎ Chase states that more than three months before… | `§ 2034.260` | `§ 2034.260, subd. (b)(1)` | right |
| cl0330 | stops early | id. §§ 331(u) (1994); 342(f), (g) (1994); 343(r) (1994 Supp. V 1999) (health cla… | `§§ 331(u)` | `§§ 331(u)` | stops early |
| cl0337 | stops early | id. §§ 1704.101 (board's powers), 1704.102 (enforcement authority). In a county … | `§§ 1704.101` | `§§ 1704.101` | stops early |
| cl0344 | missed | id. §⏎355(j)(4)(A).⏎⏎Although the Hatch-Waxman Act streamlined the process for b… | — | `§⏎355(j)(4)(A)` | right |
| cl0347 | stops early | Id. § 355(]')(2)(A)(vii)(ll). A “paragraph III” certification identifies the dat… | `§ 355` | `§ 355` | stops early |
| cl0349 | stops early | Id. § 355(]')(5)(B)(iii).⏎⏎Would-be generic drug manufacturers thus risk costly … | `§ 355` | `§ 355` | stops early |
| cl0350 | stops early | Id. § 355(]')(5)(D)(i)(lV) (emphasis added). For present purposes, then, the que… | `§ 355` | `§ 355` | stops early |
| cl0354 | missed | id.⏎⏎§ 161.001(b)(1)(D), (E), (P). Nor has Mother challenged the sufficiency of … | — | `§ 161.001(b)(1)(D), (E), (P)` | right |
| cl0362 | missed | id.⏎§ 7454. In their complaint and in subsequent pleadings, Plaintiffs alleged t… | — | `§ 7454` | right |
| cl0375 | missed | id.⏎§ 2344, this court lacks jurisdiction to address Matson’s⏎petition for revie… | — | `§ 2344` | right |
| cl0377 | missed | Id. §⏎⏎27.011(b).⏎⏎ The Act provides a two-step process to protect the exercise … | — | `§⏎⏎27.011(b)` | right |
| cl0394 | stops early | Id. § 4513(a)(l)(A),⏎(B)(i), (B)(V)-⏎⏎Additionally, HERA authorized the Director… | `§ 4513(a)(l)(A)` | `§ 4513(a)(l)(A),⏎(B)(i), (B)` | stops early |
| cl0400 | stops early | Id. §2901(a.1)(2) (emphasis added). As⏎⏎commonly understood, see 1 Pa.C.S. §1903… | `§2901` | `§2901(a.1)(2)` | right |
| cl0404 | missed | Id. at⏎⏎§ 32.51(a)(1). And the Texas Penal Code defines “harm” as “anything reas… | — | `at⏎⏎§ 32.51(a)(1)` | right |
| cl0407 | stops early | Id. § 780-113 (b)-(o). Specifically, the⏎⏎penalty for PWID, cocaine base, is def… | `§ 780-113` | `§ 780-113 (b)-(o)` | right |
| cl0408 | stops early | Id. § 780-113(f)(1.1) (footnote omitted).⏎⏎ It is clear that The Controlled Subs… | `§ 780-113(f)` | `§ 780-113(f)(1.1)` | right |
| cl0471 | missed | Id.⏎⏎§ 5(a)(1)(A). Attempted robbery is not one of the enumerated offenses in th… | — | — | missed |
| cl0483 | missed | id.⏎§ 2B3.1(b)(4)(B). Finally, the court granted Harris a three-level reduction … | — | `§ 2B3.1(b)(4)(B)` | right |
| cl0501 | stops early | Id. § 2001.033(a)(1), § 2001.035(a) (West 2000). A reasoned justification must i… | `§ 2001.033(a)(1)` | `§ 2001.033(a)(1), § 2001.035(a)` | right |
