"""eyecite's test_FindTest.py: test_find_citations, test_find_law_citations,
test_find_journal_citations, test_find_tc_citations, test_custom_tokenizer
and test_citation_fullspan (eyecite v2.7.8). Provenance: engine-tests, not
ground truth; see eyecite_port/__init__.py.

The test pairs are eyecite's, copied unchanged. Each runs with citare's
default tokenizer; eyecite also runs its base Tokenizer and Hyperscan, and
test_api.py checks citare's base Tokenizer on a sample.
"""

from copy import copy

import pytest
from citare.tokenizers import EXTRACTORS, Tokenizer

from citare import get_citations
from eyecite_port.factories import (
    case_citation,
    comparison_attrs,
    expected_attrs,
    id_citation,
    journal_citation,
    law_citation,
    reference_citation,
    supra_citation,
    unknown_citation,
)

# fmt: off
FIND_CITATIONS = (
    # Basic test
    ('1 U.S. 1',
     [case_citation()]),
    # Basic test with a line break
    ('1 U.S.\n1',
     [case_citation()],
     {'clean_steps': ['all_whitespace']}),
    # Basic test with a line break within a reporter
    ('1 U.\nS. 1',
     [case_citation(reporter_found='U. S.')],
     {'clean_steps': ['all_whitespace']}),
    # Basic test of non-case name before citation (should not be found)
    ('lissner test 1 U.S. 1',
     [case_citation()]),
    # Test with plaintiff and defendant
    ('Lissner v. Test 1 U.S. 1',
     [case_citation(metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test'})]),
    # Test with plaintiff, defendant and year
    ('Lissner v. Test 1 U.S. 1 (1982)',
     [case_citation(metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test'},
                    year=1982)]),
    # Don't choke on misformatted year
    ('Lissner v. Test 1 U.S. 1 (198⁴)',
     [case_citation(metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test'})]),
    # Test with different reporter than all of above.
    ('bob Lissner v. Test 1 F.2d 1 (1982)',
     [case_citation(reporter='F.2d', year=1982,
                    metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test'})]),
    # Test with comma after defendant's name
    ('Lissner v. Test, 1 U.S. 1 (1982)',
     [case_citation(metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test'},
                    year=1982)]),
    # can we handle variations with parenthesis
    ('1 So.2d at 1',
     [case_citation(volume="1", reporter="So.2d", page="1", short=True,
      metadata={'pin_cite': '1'})]),
    # Test with court and extra information
    ('bob Lissner v. Test 1 U.S. 12, 347-348 (4th Cir. 1982)',
     [case_citation(page='12', year=1982,
                    metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test',
                              'court': 'ca4',
                              'pin_cite': '347-348'})]),
    # Test with court string without space
    ('bob Lissner v. Test 1 U.S. 12, 347-348 (Pa.Super. 1982)',
     [case_citation(page='12', year=1982,
                    metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test',
                              'court': 'pasuperct',
                              'pin_cite': '347-348'})]),
    # Test with court string exact match
    ('Commonwealth v. Muniz, 164 A.3d 1189 (Pa. 2017)',
     [case_citation(page='1189', reporter='A.3d', volume='164', year=2017,
                    metadata={'plaintiff': 'Commonwealth',
                              'defendant': 'Muniz',
                              'court': 'pa'})]),
    # Test with month/day in court parenthetical
    ('Commonwealth v. Muniz, 164 A.3d 1189 (Pa. Feb. 9, 2017)',
     [case_citation(page='1189', reporter='A.3d', volume='164', year=2017,
                    metadata={'plaintiff': 'Commonwealth',
                              'defendant': 'Muniz',
                              'month': 'Feb.',
                              'day': '9',
                              'court': 'pa'})]),
    # Parallel cite with parenthetical
    ('Bob Lissner v. Test 1 U.S. 12, 347-348, 1 S. Ct. 2, 358 (4th Cir. 1982) (overruling foo)',
     [case_citation(page='12', year=1982,
                    metadata={'plaintiff': 'Bob Lissner',
                              'defendant': 'Test',
                              'court': 'ca4',
                              'pin_cite': '347-348',
                              'extra': "1 S. Ct. 2, 358",
                              'parenthetical': 'overruling foo'}),
      case_citation(page='2', reporter='S. Ct.', year=1982,
                    metadata={'plaintiff': 'Bob Lissner',
                              'defendant': 'Test',
                              'court': 'ca4',
                              'pin_cite': '358',
                              'parenthetical': 'overruling foo'}),
      ]),
    # Test full citation with nested parenthetical
    ('Lissner v. Test 1 U.S. 1 (1982) (discussing abc (Holmes, J., concurring))',
     [case_citation(metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test',
                              'parenthetical': 'discussing abc (Holmes, J., concurring)'},
                    year=1982)]),
    # Test full citation with parenthetical and subsequent unrelated parenthetical
    ('Lissner v. Test 1 U.S. 1 (1982) (discussing abc); blah (something).',
     [case_citation(metadata={'plaintiff': 'Lissner',
                              'defendant': 'Test',
                              'parenthetical': 'discussing abc'},
                    year=1982)]),
    # Test with text before and after and a variant reporter
    ('asfd 22 U. S. 332 (1975) asdf',
     [case_citation(page='332', volume='22',
                    reporter_found='U. S.', year=1975)]),
    # Test with finding reporter when it's a second edition
    ('asdf 22 A.2d 332 asdf',
     [case_citation(page='332', reporter='A.2d', volume='22')]),
    # Test if reporter in string will find proper citation string
    ('A.2d 332 11 A.2d 333',
     [case_citation(page='333', reporter='A.2d', volume='11')]),
    # Test finding a variant second edition reporter
    ('asdf 22 A. 2d 332 asdf',
     [case_citation(page='332', reporter='A.2d', volume='22',
                    reporter_found='A. 2d')]),
    # Test finding a variant of an edition resolvable by variant alone.
    ('171 Wn.2d 1016',
     [case_citation(page='1016', reporter='Wash. 2d', volume='171',
                    reporter_found='Wn.2d')]),
    # Test finding two citations where one of them has abutting
    # punctuation.
    ('2 U.S. 3, 4-5 (3 Atl. 33)',
     [case_citation(page='3', volume='2', metadata={'pin_cite': '4-5'}),
      case_citation(page='33', reporter="A.", volume='3',
                    reporter_found="Atl.")]),
    # Test with the page number as a Roman numeral
    ('12 Neb. App. lxiv (2004)',
     [case_citation(page='lxiv', reporter='Neb. Ct. App.',
                    volume='12',
                    reporter_found='Neb. App.', year=2004)]),
    # Test with page range with a weird suffix
    ('559 N.W.2d 826|N.D.',
     [case_citation(page='826', reporter='N.W.2d', volume='559')]),
    # Test with malformed page number
    ('1 U.S. f24601', []),
    # Test with page number that is indicated as missing
    ('1 U.S. ___',
     [case_citation(volume='1', reporter='U.S.', page=None)]),
    # Test with page number that is indicated as missing, followed by
    # a comma (cf. eyecite#137)
    ('1 U. S. ___,',
     [case_citation(volume='1', reporter_found='U. S.', page=None)]),
    # Test with the 'digit-REPORTER-digit' corner-case formatting
    ('2007-NMCERT-008',
     [case_citation(source_text='2007-NMCERT-008', page='008',
                    reporter='NMCERT', volume='2007')]),
    ('2006-Ohio-2095',
     [case_citation(source_text='2006-Ohio-2095', page='2095',
                    reporter='Ohio', volume='2006')]),
    ('2017 IL App (4th) 160407',
     [case_citation(page='160407', reporter='IL App (4th)',
                    volume='2017')]),
    ('2017 IL App (1st) 143684-B',
     [case_citation(page='143684-B', reporter='IL App (1st)',
                    volume='2017')]),
    # Test first kind of short form citation (meaningless antecedent)
    ('before Foo 1 U. S., at 2',
     [case_citation(page='2', reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo'})]),
    # Test second kind of short form citation (meaningful antecedent)
    ('before Foo, 1 U. S., at 2',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo'})]),
    # Test short form citation with preceding ASCII quotation
    ('before Foo,” 1 U. S., at 2',
     [case_citation(page='2', reporter_found='U. S.',
                    short=True)]),
    # Test short form citation when case name looks like a reporter
    ('before Johnson, 1 U. S., at 2',
     [case_citation(page='2', reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Johnson'})]),
    # Test short form citation with no comma after reporter
    ('before Foo, 1 U. S. at 2',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo'})]),
    # Test short form citation at end of document (issue #1171)
    ('before Foo, 1 U. S. end', []),
    # Test supra citation across line break
    ('before Foo, supra,\nat 2',
     [supra_citation("supra,",
                     metadata={'pin_cite': 'at 2',
                               'antecedent_guess': 'Foo'})],
     {'clean_steps': ['all_whitespace']}),
    # Test short form citation with at p. format
    ('174 Cal.App.2d at p. 651',
     [case_citation(volume='174', page='651', reporter='Cal.App.2d', short=True,
                    metadata={'pin_cite': '651'})]),
    # Test short form citation with a page range
    ('before Foo, 1 U. S., at 20-25',
     [case_citation(page='20', reporter_found='U. S.', short=True,
                    metadata={'pin_cite': '20-25',
                              'antecedent_guess': 'Foo'})]),
    # Test short form citation with a page range with weird suffix
    ('before Foo, 1 U. S., at 20-25\\& n. 4',
     [case_citation(page='20', reporter_found='U. S.', short=True,
                    metadata={'pin_cite': '20-25',
                              'antecedent_guess': 'Foo'})]),
    # Test short form citation with a parenthetical
    ('before Foo, 1 U. S., at 2 (overruling xyz)',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo',
                              'parenthetical': 'overruling xyz'}
                    )]),
    # Test short form citation with no space before parenthetical
    ('before Foo, 1 U. S., at 2(overruling xyz)',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo',
                              'parenthetical': 'overruling xyz'}
                    )]),
    # Test short form citation with nested parentheticals
    ('before Foo, 1 U. S., at 2 (discussing xyz (Holmes, J., concurring))',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo',
                              'parenthetical': 'discussing xyz (Holmes, J., concurring)'}
                    )]),
    # Test that short form citation doesn't treat year as parenthetical
    ('before Foo, 1 U. S., at 2 (2016)',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo'}
                    )]),
    # Test short form citation with page range and parenthetical
    ('before Foo, 1 U. S., at 20-25 (overruling xyz)',
     [case_citation(page='20', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo',
                              'pin_cite': '20-25',
                              'parenthetical': 'overruling xyz'}
                    )]),
    # Test short form citation with subsequent unrelated parenthetical
    ('Foo, 1 U. S., at 4 (discussing abc). Some other nonsense (clarifying nonsense)',
     [case_citation(page='4', reporter='U.S.',
                    reporter_found='U. S.', short=True,
                    metadata={'antecedent_guess': 'Foo',
                              'parenthetical': 'discussing abc'}
                    )]
     ),
    # Test short form citation generated from non-standard regex for full cite
    ('1 Mich. at 1',
     [case_citation(reporter='Mich.', short=True)]),
    # Test parenthetical matching with multiple citations
    ('1 U. S., at 2. Foo v. Bar 3 U. S. 4 (2010) (overruling xyz).',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.',
                    short=True, volume='1',
                    metadata={'pin_cite': '2'}),
      case_citation(page='4', reporter='U.S.',
                    reporter_found='U. S.', short=False,
                    year=2010, volume='3',
                    metadata={'parenthetical': 'overruling xyz',
                              'plaintiff': 'Foo', 'defendant': 'Bar'})
      ]),
    # Test with multiple citations and parentheticals
    ('1 U. S., at 2 (criticizing xyz). Foo v. Bar 3 U. S. 4 (2010) (overruling xyz).',
     [case_citation(page='2', reporter='U.S.',
                    reporter_found='U. S.',
                    short=True, volume='1',
                    metadata={'pin_cite': '2',
                              'parenthetical': 'criticizing xyz'}),
      case_citation(page='4', reporter='U.S.',
                    reporter_found='U. S.', short=False,
                    year=2010, volume='3',
                    metadata={'parenthetical': 'overruling xyz',
                              'plaintiff': 'Foo', 'defendant': 'Bar'})
      ]),
    # Test first kind of supra citation (standard kind)
    ('before asdf, supra, at 2',
     [supra_citation("supra,",
                     metadata={'pin_cite': 'at 2',
                               'antecedent_guess': 'asdf'})]),
    # Test second kind of supra citation (with volume)
    ('before asdf, 123 supra, at 2',
     [supra_citation("supra,",
                     metadata={'pin_cite': 'at 2',
                               'volume': '123',
                               'antecedent_guess': 'asdf'})]),
    # Test third kind of supra citation (sans page)
    ('before Asdf, supra, foo bar',
     [supra_citation("supra,",
                     metadata={'antecedent_guess': 'Asdf'})]),
    # Test third kind of supra citation (with period)
    ('before Asdf, supra. foo bar',
     [supra_citation("supra,",
                     metadata={'antecedent_guess': 'Asdf'})]),
    # Test supra citation at end of document (issue #1171)
    ('before asdf, supra end',
     [supra_citation("supra,",
                     metadata={'antecedent_guess': 'asdf'})]),
    # Supra with parenthetical
    ('Foo, supra (overruling ...) (ignore this)',
     [supra_citation("supra",
                     metadata={'antecedent_guess': 'Foo',
                               'parenthetical': 'overruling ...'})]),
    ('Foo, supra, at 2 (overruling ...)',
     [supra_citation("supra",
                     metadata={'antecedent_guess': 'Foo',
                               'pin_cite': 'at 2',
                               'parenthetical': 'overruling ...'})]),
    # Test Ibid. citation
    ('Foo v. Bar 1 U.S. 12. asdf. Ibid. foo bar lorem ipsum.',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar'}),
      id_citation('Ibid.')]),
    # Test italicized Ibid. citation
    ('<p>before asdf. <i>Ibid.</i></p> <p>foo bar lorem</p>',
     [id_citation('Ibid.')],
     {'clean_steps': ['html', 'inline_whitespace']}),
    # Test Id. citation
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id., at 123. foo bar',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.,',
                  metadata={'pin_cite': 'at 123'})]),
    # Test Id. citation across line break
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id.,\nat 123. foo bar',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.,', metadata={'pin_cite': 'at 123'})],
     {'clean_steps': ['all_whitespace']}),
    # Test italicized Id. citation
    ('<p>before asdf. <i>Id.,</i> at 123.</p> <p>foo bar</p>',
     [id_citation('Id.,', metadata={'pin_cite': 'at 123'})],
     {'clean_steps': ['html', 'inline_whitespace']}),
    # Test italicized Id. citation with another HTML tag in the way
    ('<p>before asdf. <i>Id.,</i> at <b>123.</b></p> <p>foo bar</p>',
     [id_citation('Id.,', metadata={'pin_cite': 'at 123'})],
     {'clean_steps': ['html', 'inline_whitespace']}),
    # Test weirder Id. citations (#1344)
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id. ¶ 34. foo bar',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.', metadata={'pin_cite': '¶ 34'})]),
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id. at 62-63, 67-68. f b',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.', metadata={'pin_cite': 'at 62-63, 67-68'})]),
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id., at *10. foo bar',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.,', metadata={'pin_cite': 'at *10'})]),
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id. at 7-9, ¶¶ 38-53. f b',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.', metadata={'pin_cite': 'at 7-9, ¶¶ 38-53'})]),
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id. at pp. 45, 64. foo bar',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.', metadata={'pin_cite': 'at pp. 45, 64'})]),
    # Cleanup parentheses and square brackets
    (' (Newbold v Arvidson, 105 Idaho 663, 672 P2d 231 [1983]); and something else (Newbold, 105 Idaho at 667, 672 P2d at 235).',
     [case_citation(volume='105', reporter='Idaho', page='663',
                    metadata={'plaintiff': 'Newbold',
                              'defendant': 'Arvidson',
                              'extra': '672 P2d 231',
                              'year': '1983'}),
      case_citation(volume='672', reporter='P2d', page='231',
                    metadata={'plaintiff': 'Newbold',
                              'defendant': 'Arvidson',
                              'year': '1983'}),
      case_citation(volume='105', reporter='Idaho', page='667',
                    short=True,
                    metadata={'antecedent_guess': 'Newbold', 'pin_cite': '667'}),
      case_citation(volume='672', reporter='P2d', page='235',
                    short=True,
                    metadata={'antecedent_guess': "Newbold",  'pin_cite': '235'})
      ]),
    # Square brackets around year
    ('Rogers v Rogers (63 NY2d 582 [1984])',
     [case_citation(volume='63', reporter='NY2d', page='582',
                    metadata={'plaintiff': 'Rogers',
                              'defendant': 'Rogers',
                              'year': '1984'})]
     ),
    # test citation always breaks at ; even at year
    ('(1963); Reece v. State of Washington, 310 F.2d 139 (1962)',
     [case_citation(volume='310', reporter='F.2d', page='139',
                    metadata={'plaintiff': 'Reece',
                              'defendant': 'State of Washington',
                              'year': '1962'})]
     ),
    # As seen in /107252/miranda-v-arizona/
    ('(Mo.); Bean v. State, — Nev. —, 398 P. 2d 251; ',
     [case_citation(volume='398', reporter='P. 2d', page='251',
                    metadata={'plaintiff': 'Bean',
                              'defendant': 'State'})]
     ),
    # https://www.courtlistener.com/opinion/4439963/mark-a-twilegar/
    # Common em-dash format
    ('Hurst v. Florida, — U.S. —, 136 S.Ct. 616, 193 L.Ed.2d 504 (2016)',
     [case_citation(volume='136', reporter='S.Ct.', page='616',
                    metadata={'plaintiff': 'Hurst',
                              'defendant': 'Florida',
                              'year': '2016',
                              'court': 'scotus',
                              'extra': '193 L.Ed.2d 504'
                              }),
      case_citation(volume='193', reporter='L.Ed.2d', page='504',
                    metadata={'plaintiff': 'Hurst',
                              'defendant': 'Florida',
                              'year': '2016',
                              })
      ]
     ),
    # https://www.courtlistener.com/opinion/9502461/balderas
    # Single underscore placeholder
    ('In Viking River Cruises v. Moriana (2022) _ U.S. _ [213 L.Ed.2d 179, 200-201] (Viking River)',
     [case_citation(volume='213', reporter='L.Ed.2d', page='179',
                    metadata={'plaintiff': 'Viking River Cruises',
                              'defendant': 'Moriana',
                              'year': '2022',
                              'pin_cite': '200-201',
                              })
      ]
     ),
    # Two underscore placeholder
    # https://www.courtlistener.com/opinion/5290606/williams
    ('decision in Epic Systems Corporation v. Lewis (2018) __ U.S. __ [138 S.Ct. 1612] (Epic Systems)',
     [case_citation(volume='138', reporter='S.Ct.', page='1612',
                    metadata={'plaintiff': 'Epic Systems Corporation',
                              'defendant': 'Lewis',
                              'year': '2018',
                              'court': 'scotus',
                              })
      ]
     ),
    # https://www.courtlistener.com/opinion/9780755/touchard
    # test complicated multiple citations with placeholders
    ('speech.” Houston Cmty. Coll. Sys. v. Wilson, ---- U.S. ----, '
     '142 S. Ct. 1253, 1259, ---- L. Ed. 2d ---- (2022) (emphasis added) '
     '(quoting Nieves v. Bartlett, ---- U.S. ----, 139 S. Ct. 1715, '
     '1722, 204 L. Ed. 2d 1 (2019);',
     [case_citation(volume='142', reporter='S. Ct.', page='1253',
                    metadata={'plaintiff': 'Sys.',
                              'defendant': 'Wilson',
                              'year': '2022',
                              'pin_cite': '1259',
                              'court': 'scotus',
                              'extra': '---- L. Ed. 2d ----',
                              'parenthetical': 'emphasis added',
                              }),
      case_citation(volume='139', reporter='S. Ct.', page='1715',
                    metadata={'plaintiff': 'Nieves',
                              'defendant': 'Bartlett',
                              'year': '2019',
                              'extra': '204 L. Ed. 2d 1',
                              'court': 'scotus',
                              'pin_cite': '1722',
                              }),
      case_citation(volume='204', reporter='L. Ed. 2d', page='1',
                    metadata={'plaintiff': 'Nieves',
                              'defendant': 'Bartlett',
                              'year': '2019',
                              })
      ]
     ),
    # test lower case sentence
    ('curiams. Spano v. People of State of New York, 360 U.S. 315',
     [case_citation(volume='360', reporter='U.S.', page='315',
                    metadata={'plaintiff': 'Spano',
                              'defendant': 'People of State of New York'})]
     ),
    # Test capitalized word
    ('Per Curiams. Spano v. People of State of New York, 360 U.S. 315',
     [case_citation(volume='360', reporter='U.S.', page='315',
                    metadata={'plaintiff': 'Spano',
                              'defendant': 'People of State of New York'})]
     ),
    # Square brackets around year and court
    ('Mavrovich v Vanderpool, 427 F Supp 2d 1084 [D Kan 2006]',
     [case_citation(volume='427', reporter='F Supp 2d', page='1084',
                    metadata={'plaintiff': 'Mavrovich',
                              'defendant': 'Vanderpool',
                              'court': 'ksd',
                              'year': '2006'})]),
    # Parentheses not square brackets
    ('Mavrovich v Vanderpool, 427 F Supp 2d 1084 (D Kan 2006)',
     [case_citation(volume='427', reporter='F Supp 2d', page='1084',
                    metadata={'plaintiff': 'Mavrovich',
                              'defendant': 'Vanderpool',
                              'court': 'ksd',
                              'year': '2006'})]),
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. id. 119:12-14. foo bar',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('id.', metadata={'pin_cite': '119:12-14'})]),
    # Test Id. citation without page number
    ('Foo v. Bar 1 U.S. 12, 347-348. asdf. Id. No page number.',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      id_citation('Id.')]),
    # Id. with parenthetical
    ('Id. (overruling ...) (ignore this)',
     [id_citation("Id.", metadata={'parenthetical': 'overruling ...'})]),
    ('Id. at 2 (overruling ...)',
     [id_citation("Id.",
                  metadata={'pin_cite': 'at 2',
                            'parenthetical': 'overruling ...'})]),
    # Test unknown citation
    ('lorem ipsum see §99 of the U.S. code.',
     [unknown_citation('§99')]),
    # Test address that's not a citation (#1338)
    ('lorem 111 S.W. 12th St.',
     [],),
    ('lorem 111 N. W. 12th St.',
     [],),
    # Test filtering overlapping citations - this finds four citations
    # but should filter down to three
    ("Miles v. Smith 1 Ga. 1; asdfasdf asd Something v. Else, 1 Miles 3; 1 Miles at 10",
     [case_citation(page='1',
                    volume="1",
                    reporter="Ga.",
                    metadata={'plaintiff': 'Miles',
                              'defendant': 'Smith'}),
      case_citation(page='3',
                    volume="1",
                    reporter="Miles",
                    metadata={'plaintiff': 'Something',
                              'defendant': 'Else'}
                    ),
      case_citation(volume="1", page='10', reporter='Miles',
                    short=True,
                    metadata={'pin_cite': '10'})]),
    ('General Casualty cites as compelling Amick v. Liberty Mut. Ins. Co., 455 A.2d 793 (R.I. 1983). In that case ... Stats, do. See Amick at 795',
     [case_citation(page='793',
                    volume="455",
                    reporter="A.2d",
                    year=1983,
                    metadata={'plaintiff': 'Amick',
                              'defendant': 'Liberty Mut. Ins. Co.',
                              'court': 'ri'
                              }),
      reference_citation('Amick at 795', metadata={'plaintiff': 'Amick', 'pin_cite': 'at 795'})]),
    # Test reference citation
    ('Foo v. Bar 1 U.S. 12, 347-348. something something, In Foo at 62, we see that',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'}),
      reference_citation('Foo at 62', metadata={'plaintiff': 'Foo', 'pin_cite': 'at 62'})]),
    ('Foo v. United States 1 U.S. 12, 347-348. something something ... the United States at 1776 we see that and Foo at 62',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'United States',
                              'pin_cite': '347-348'}),
      reference_citation('Foo at 62', metadata={'plaintiff': 'Foo', 'pin_cite': 'at 62'})]),
    # Test that reference citation must occur after full case citation
    ('In Foo at 62 we see that, Foo v. Bar 1 U.S. 12, 347-348. something something,',
     [case_citation(page='12',
                    metadata={'plaintiff': 'Foo',
                              'defendant': 'Bar',
                              'pin_cite': '347-348'})]),
    # Test reference against defendant name
    ('In re Foo 1 Mass. 12, 347-348. something something, in Foo at 62, we see that, ',
     [case_citation(page='12', reporter="Mass.", volume="1",
                    metadata={'defendant': 'Foo', 'pin_cite': '347-348'}),
      reference_citation('Foo at 62',
                         metadata={'defendant': 'Foo',
                                   "pin_cite": "at 62"})]),
    # Test reference citation that contains at
    ('In re Foo 1 Mass. 12, 347-348. something something, in at we see that',
     [case_citation(page='12', reporter="Mass.", volume="1",
                    metadata={'defendant': 'Foo', 'pin_cite': '347-348'})]),
    # Test U.S. as plaintiff with reference citations
    ('U.S. v. Boch Oldsmobile, Inc., 909 F.2d 657, 660 (1st Cir.1990); Piper Aircraft, 454 U.S. at 241',
     [case_citation(page='657', reporter="F.2d", volume="909",
                    metadata={'plaintiff': 'U.S.', 'defendant': 'Boch Oldsmobile, Inc.', 'pin_cite': '660'}),
      case_citation(volume="454", page='241', reporter_found='U.S.', short=True,
                    metadata={'antecedent_guess': 'Piper Aircraft', 'court': "scotus", 'pin_cite': "241"})]),
    # Test reference citation after an id citation
    ('we said in Morton v. Mancari, 417 U. S. 535, 552 (1974) “Literally every piece ....”. “asisovereign tribal entities . . . .” Id. In Mancari at 665',
     [case_citation(page='535', year=1974, volume="417",
                    reporter="U. S.",
                    metadata={'plaintiff': 'Morton', 'defendant': 'Mancari', "pin_cite": "552", "court": "scotus"}),
      id_citation('Id.,', metadata={}),
      reference_citation('Mancari',
                         metadata={'defendant': 'Mancari', "pin_cite": "at 665"})]),
    # Test Conn. Super. Ct. regex variation.
    ('Failed to recognize 1993 Conn. Super. Ct. 5243-P',
     [case_citation(volume='1993', reporter='Conn. Super. Ct.',
                    page='5243-P')]),
    # Test that the tokenizer handles commas after a reporter. In the
    # past, " U. S. " would match but not " U. S., "
    ('foo 1 U.S., 1 bar',
     [case_citation()]),
    # Test reporter with custom regex
    ('blah blah Bankr. L. Rep. (CCH) P12,345. blah blah',
     [case_citation(volume=None, reporter='Bankr. L. Rep.',
                    reporter_found='Bankr. L. Rep. (CCH)', page='12,345')]),
    ('blah blah, 2009 12345 (La.App. 1 Cir. 05/10/10). blah blah',
     [case_citation(volume='2009', reporter='La.App. 1 Cir.',
                    page='12345', groups={'date_filed': '05/10/10'})]),
    # Token scanning edge case -- incomplete paren at end of input
    ('1 U.S. 1 (', [case_citation()]),
    # Token scanning edge case -- missing plaintiff name at start of input
    ('v. Bar, 1 U.S. 1', [case_citation(metadata={'antecedent_guess': 'Bar'})]),
    # Token scanning edge case -- short form start of input
    ('1 U.S., at 1', [case_citation(short=True)]),
    (', 1 U.S., at 1', [case_citation(short=True)]),
    # Token scanning edge case -- supra at start of input
    ('supra.', [supra_citation("supra.")]),
    (', supra.', [supra_citation("supra.")]),
    ('123 supra.', [supra_citation("supra.", metadata={'volume': "123"})]),
    # Token scanning edge case -- Id. at end of input
    ('Id.', [id_citation('Id.,')]),
    ('Id. at 1.', [id_citation('Id.,', metadata={'pin_cite': 'at 1'})]),
    ('Id. foo', [id_citation('Id.,')]),
    # Reject citations that are part of larger words
    ('foo1 U.S. 1, 1. U.S. 1foo', [],),
    ("Corp. v. Nature's Farm Prods., No. 99 Civ. 9404 (SHS), 2000 U.S. Dist. LEXIS 12335 (S.D.N.Y. Aug. 25, 2000)",
     [case_citation(
         volume='2000',
         reporter='U.S. Dist. LEXIS',
         page='12335',
         year=2000,
         metadata={'plaintiff': "Corp.", 'defendant': "Nature's Farm Prods., No. 99 Civ. 9404 (SHS)", "month": "Aug.", "day": "25", "court": "nysd"})
      ],),
    # Long pin cite -- make sure no catastrophic backtracking in regex
    ('1 U.S. 1, 2277, 2278, 2279, 2280, 2281, 2282, 2283, 2284, 2286, 2287, 2288, 2289, 2290, 2291',
     [case_citation(metadata={'pin_cite': '2277, 2278, 2279, 2280, 2281, 2282, 2283, 2284, 2286, 2287, 2288, 2289, 2290, 2291'})]),
    ('Commonwealth v. Muniz, 164 A.3d 1189 (Pa. 2017)', [
        case_citation(volume='164', reporter='A.3d', year=2017,
                      page='1189',
                      metadata={'plaintiff': 'Commonwealth', 'defendant': 'Muniz',
                                'court': 'pa'})]),
    ('Foo v. Bar,  1 F.Supp. 1 (SC 1967)', [case_citation(volume='1', reporter='F.Supp.', year=1967, page='1', metadata={'plaintiff': 'Foo', 'defendant': 'Bar', 'court': 'sc'})]),
    ('trial court’s ruling. (See In re K.F. (2009) 1 U.S. 1 ', [
        case_citation(
         year=2009, metadata={'defendant': 'K.F.', "year": "2009"})]
     ),
    # extract no page citations
    ("\n 576 U.S. ___, 13",
     [case_citation(volume='576', reporter='U.S.', page=None, metadata={'pin_cite': '13', "court": 'scotus'})],
     {"clean_steps": ["all_whitespace"]},
     ),
    # test handling zero width whitespace
    ("Shady Grove Farms \u200bv Goldsmith Seeds 1 U.S. 1",
     [case_citation(metadata={'defendant': 'Goldsmith Seeds', "plaintiff": 'Shady Grove Farms'})],
     {"clean_steps": ["all_whitespace"]},
     ),
    # Test unprintable characters # /recap-documents/429621284"
    ("Shady Grove Farms v Goldsmith Seeds 1 U.S. 1  \x08*\x07\x07\u038bþİ\u038b\u202cڋ\u202a-\x14V\u202c\u202c",
     [case_citation(metadata={'defendant': 'Goldsmith Seeds', "plaintiff": 'Shady Grove Farms'})],
     {"clean_steps": ["all_whitespace"]},
     ),
    ('(See In re K.F. (2009) 1 U.S. 1, 4 [92 Cal.Rptr.3d 784]; Yield Dynamics, Inc. v. TEA Systems Corp. (2007) 154 Cal.App.4th 547, 558 [66 Cal.Rptr.3d 1].)”', [
        case_citation(
            year=2009,
            metadata={'defendant': 'K.F.', "year": "2009", 'pin_cite': '4'}
        ),
        case_citation(
            year=2009, volume='92', reporter='Cal.Rptr.3d', page='784',
            metadata={'defendant': 'K.F.', "year": "2009"}
        ),
        case_citation(
            year=2007, volume='154', reporter='Cal.App.4th', page='547',
            metadata={'plaintiff': 'Yield Dynamics, Inc.', 'defendant': 'TEA Systems Corp.', "year": "2007", "pin_cite": "558"}
        ),
        case_citation(
            year=2007, volume='66', reporter='Cal.Rptr.3d', page='1',
            metadata={'plaintiff': 'Yield Dynamics, Inc.', 'defendant': 'TEA Systems Corp.', "year": "2007"}
        ),
    ]),
    # ignore bad reference matching in plain text
    ("In Foo v. Bar 1 U.S. 1, ... yadda yadda yadda the case of Foo v. Bar, supra, was affirmed", [
        case_citation(metadata={'plaintiff': 'Foo', "defendant": "Bar",
                                "court": "scotus"}),
        supra_citation("supra.", metadata={'antecedent_guess': "Bar"})
    ]),
    # Dont make second foo a reference citation
    ('<p>In <i>Foo</i> v. <i>Bar,</i> 1 U.S. 1, ... yadda yadda yadda the case of <i>Foo</i> v. <i>Bar, supra</i><i>,</i> was affirmed.</p>',
     [
        case_citation(metadata={'plaintiff': 'Foo', "defendant": "Bar", "court": "scotus"}),
        supra_citation("supra.", metadata={'antecedent_guess': "Bar"})
     ], {'clean_steps': ['html', 'inline_whitespace']}),
    ("""<em>Grant </em>v. <em>Esquire, Inc. </em>(S.D.N.Y. 1973) 367 F.Supp. 876, 879.) ...
     <em>Grant </em>v. <em>Esquire, Inc., supra, </em>316 F.Supp. at 884.)</p>
     <p id="b883-5">III. </p> """,
     [
         case_citation(page='876', reporter='F.Supp.', volume='367',
                       metadata={'plaintiff': 'Grant', "defendant": "Esquire, Inc.", "pin_cite": "879"}),
         case_citation(volume="316", reporter="F.Supp.", page="884", short=True,
                       metadata={'antecedent_guess': "Esquire, Inc., supra", "pin_cite": "884"}),
     ], {'clean_steps': ['html', 'inline_whitespace']}),
    ("""However, as we explained in Kaiser Foundation, supra, 13 Cal.App.5th at 1146""",
     [
         case_citation(page='1146', reporter='Cal.App.5th', volume='13',
                       short=True,
                       metadata={"pin_cite": "1146",
                                 "antecedent_guess": "Kaiser Foundation"}),
      ]),
    ("Angelopoulos v. Keystone Orthopedic Specialists, S.C., Wachn, LCC, 2015 WL 2375225, at p. *4. yadda yadda yadda... Angelopoulos p. *4;",
     [
         case_citation(volume="2015", reporter="WL", page="2375225",
                       metadata={"plaintiff": "Angelopoulos",
                                 "defendant": "Keystone Orthopedic Specialists, S.C., Wachn, LCC",
                                 "pin_cite": "at p. *4"}),
         reference_citation("Angelopoulos at 4",
                            metadata={"plaintiff": "Angelopoulos", "pin_cite": "p. *4"}),
     ]),
    # properly not remove An from the start of the final word
    ("Angelopoulos v. Keystone Orthopedic Specialists, S.C., Wachn, LCC, 2015 WL 2375225",
     [case_citation(volume="2015", reporter="WL", page="2375225",
                    metadata={"plaintiff": "Angelopoulos", "defendant": "Keystone Orthopedic Specialists, S.C., Wachn, LCC"})]
     ),
    ("See <em>Bivens </em>v. <em>Six Unknown Fed. Narcotics Agents, </em>403 U. S. 388 (1971). "
     "The legal issue there was whether a <em>Bivens </em> at 122, action can be employed...",
     [case_citation(volume="403", reporter="U. S.", page="388",
                    metadata={"plaintiff": "Bivens",
                              "defendant": "Six Unknown Fed. Narcotics Agents",
                              "court": "scotus",
                              "year": "1971"},
                    ),
      reference_citation("Bivens", metadata={"plaintiff": "Bivens", "pin_cite": "at 122"})],
     {'clean_steps': ['html', 'inline_whitespace']}),
    # Fix for index error when searching for case name
    ("<p>State v. Luna-Benitez (S53965). Alternative writ issued, dismissed, 342 Or 255</p>",
    [case_citation(volume="342", reporter="Or", page="255")],
    {'clean_steps': ['html', 'inline_whitespace']}),
    # Test remove text with star-pagination class
    ("<p>The somewhat similar cases of <i>Crane</i> v. <i>Hyde Park,</i> 135 <span class=\"star-pagination\">*355</span> Mass. 147, and <i>Mahoning County</i> v. <i>Young,</i> 16 U.S. App. 253, also cited by the defendant, likewise turned upon a question of forfeiture for breach of a condition subsequent in a deed to a municipal corporation.</p>",
     [case_citation(volume="135", reporter="Mass.", page="147",
                    metadata={"plaintiff": "Crane",
                              "defendant": "Hyde Park"}
                    )],
     {'clean_steps': ['html', 'inline_whitespace']})
)


LAW_CITATIONS = (
    # Basic test
    ('Mass. Gen. Laws ch. 1, § 2',
     [law_citation('Mass. Gen. Laws ch. 1, § 2',
                   reporter='Mass. Gen. Laws',
                   groups={'chapter': '1', 'section': '2'})]),
    ('1 Stat. 2',
     [law_citation('1 Stat. 2',
                   reporter='Stat.',
                   groups={'volume': '1', 'page': '2'})]),
    # year
    ('Fla. Stat. § 120.68 (2007)',
     [law_citation('Fla. Stat. § 120.68 (2007)',
                   reporter='Fla. Stat.', year=2007,
                   groups={'section': '120.68'})]),
    # et seq, publisher, year
    ('Ariz. Rev. Stat. Ann. § 36-3701 et seq. (West 2009)',
     [law_citation('Ariz. Rev. Stat. Ann. § 36-3701 et seq. (West 2009)',
                   reporter='Ariz. Rev. Stat. Ann.',
                   metadata={'pin_cite': 'et seq.', 'publisher': 'West'},
                   groups={'section': '36-3701'},
                   year=2009)]),
    # multiple sections
    ('Mass. Gen. Laws ch. 1, §§ 2-3',
     [law_citation('Mass. Gen. Laws ch. 1, §§ 2-3',
                   reporter='Mass. Gen. Laws',
                   groups={'chapter': '1', 'section': '2-3'})]),
    # parenthetical
    ('Kan. Stat. Ann. § 21-3516(a)(2) (repealed) (ignore this)',
     [law_citation('Kan. Stat. Ann. § 21-3516(a)(2) (repealed)',
                   reporter='Kan. Stat. Ann.',
                   metadata={'pin_cite': '(a)(2)', 'parenthetical': 'repealed'},
                   groups={'section': '21-3516'})]),
    # Supp. publisher
    ('Ohio Rev. Code Ann. § 5739.02(B)(7) (Lexis Supp. 2010)',
     [law_citation('Ohio Rev. Code Ann. § 5739.02(B)(7) (Lexis Supp. 2010)',
                   reporter='Ohio Rev. Code Ann.',
                   metadata={'pin_cite': '(B)(7)', 'publisher': 'Lexis Supp.'},
                   groups={'section': '5739.02'},
                   year=2010)]),
    # Year range
    ('Wis. Stat. § 655.002(2)(c) (2005-06)',
     [law_citation('Wis. Stat. § 655.002(2)(c) (2005-06)',
                   reporter='Wis. Stat.',
                   metadata={'pin_cite': '(2)(c)'},
                   groups={'section': '655.002'},
                   year=2005)]),
    # 'and' pin cite
    ('Ark. Code Ann. § 23-3-119(a)(2) and (d) (1987)',
     [law_citation('Ark. Code Ann. § 23-3-119(a)(2) and (d) (1987)',
                   reporter='Ark. Code Ann.',
                   metadata={'pin_cite': '(a)(2) and (d)'},
                   groups={'section': '23-3-119'},
                   year=1987)]),
    # Cite to multiple sections
    ('Mass. Gen. Laws ch. 1, §§ 2-3',
     [law_citation('Mass. Gen. Laws ch. 1, §§ 2-3',
                   reporter='Mass. Gen. Laws',
                   groups={'chapter': '1', 'section': '2-3'})]),
)


JOURNAL_CITATIONS = (
    # Basic test
    ('1 Minn. L. Rev. 1',
     [journal_citation()]),
    # Pin cite
    ('1 Minn. L. Rev. 1, 2-3',
     [journal_citation(metadata={'pin_cite': '2-3'})]),
    # Year
    ('1 Minn. L. Rev. 1 (2007)',
     [journal_citation(year=2007)]),
    # Pin cite and year
    ('1 Minn. L. Rev. 1, 2-3 (2007)',
     [journal_citation(metadata={'pin_cite': '2-3'}, year=2007)]),
    # Pin cite and year and parenthetical
    ('1 Minn. L. Rev. 1, 2-3 (2007) (discussing ...) (ignore this)',
     [journal_citation(year=2007,
                       metadata={'pin_cite': '2-3', 'parenthetical': 'discussing ...'})]),
    # Year range
    ('77 Marq. L. Rev. 475 (1993-94)',
     [journal_citation(volume='77', reporter='Marq. L. Rev.',
                       page='475', year=1993)]),
)


TC_CITATIONS = (
    # Test with atypical formatting for Tax Court Memos
    ('the 1 T.C. No. 233',
     [case_citation(page='233', reporter='T.C. No.')]),
    ('word T.C. Memo. 2019-233',
     [case_citation('T.C. Memo. 2019-233',
                    page='233', reporter='T.C. Memo.',
                    volume='2019')]),
    ('something T.C. Summary Opinion 2019-233',
     [case_citation('T.C. Summary Opinion 2019-233',
                    page='233', reporter='T.C. Summary Opinion',
                    volume='2019')]),
    ('T.C. Summary Opinion 2018-133',
     [case_citation('T.C. Summary Opinion 2018-133',
                    page='133', reporter='T.C. Summary Opinion',
                    volume='2018')]),
)
# fmt: on

#: Pairs where citare deliberately finds something else, keyed by the
#: query text, with the reason (see docs/divergences.md).
DIVERGENCES: dict[str, str] = {}


def run_pair(q, expected_cites, kwargs=None, tokenizer=None):
    kwargs = dict(kwargs or {})
    clean_steps = kwargs.get("clean_steps", [])
    if "html" in clean_steps:
        kwargs["markup_text"] = q
    else:
        kwargs["plain_text"] = q
    if tokenizer is not None:
        kwargs["tokenizer"] = tokenizer
    found = get_citations(**kwargs)
    assert [type(c).__name__ for c in found] == [
        e.cls.__name__ for e in expected_cites
    ], f"Extracted cite count doesn't match for {q!r}"
    for a, b in zip(found, expected_cites):
        assert comparison_attrs(a) == expected_attrs(b), (
            f"Extracted cite attrs don't match for {q!r}"
        )


def _params(pairs, name):
    out = []
    for i, (q, expected, *kwargs) in enumerate(pairs):
        marks = []
        if q in DIVERGENCES:
            marks.append(pytest.mark.xfail(reason=DIVERGENCES[q], strict=True))
        out.append(
            pytest.param(
                q, expected, kwargs[0] if kwargs else {}, id=f"{name}-{i}", marks=marks
            )
        )
    return out


@pytest.mark.parametrize(
    "q,expected,kwargs",
    _params(FIND_CITATIONS, "find")
    + _params(LAW_CITATIONS, "law")
    + _params(JOURNAL_CITATIONS, "journal")
    + _params(TC_CITATIONS, "tc"),
)
def test_find(q, expected, kwargs):
    run_pair(q, expected, kwargs)


def test_custom_tokenizer():
    extractors = []
    for e in EXTRACTORS:
        e = copy(e)
        e.regex = e.regex.replace(r"\.", r"[.,]")
        if hasattr(e, "_compiled_regex"):
            del e._compiled_regex
        extractors.append(e)
    tokenizer = Tokenizer(extractors)
    run_pair("1 U,S, 1", [case_citation(reporter_found="U,S,")], tokenizer=tokenizer)


def test_citation_fullspan():
    """Check that the full_span function returns the correct indices."""
    combined_example = "citation number one is Wilson v. Mar. Overseas Corp., 150 F.3d 1, 6-7 ( 1st Cir. 1998); This is different from Commonwealth v. Bauer, 604 A.2d 1098 (Pa.Super. 1992), my second example"
    extracted = get_citations(combined_example)
    answers = [(0, (23, 86)), (1, (111, 164))]
    for cit_idx, (start, end) in answers:
        assert extracted[cit_idx].full_span()[0] == start
        assert extracted[cit_idx].full_span()[1] == end
    # full_span should cover the whole string
    simple_examples = [
        "66 B.U. L. Rev. 71 (1986)",
        "5 Minn. L. Rev. 1339, 1341 (1991)",
        "42 U.S.C. § 405(r)(2) (2019)",
        "37 A.L.R.4th 972, 974 (1985)",
        "497 Fed. Appx. 274 (4th Cir. 2012)",
        "Corp. v. Nature's Farm Prods., No. 99 Civ. 9404 (SHS), 2000 U.S. Dist. LEXIS 12335 (S.D.N.Y. Aug. 25, 2000)",
        "Alderson v. Concordia Par. Corr. Facility, 848 F.3d 415 (5th Cir. 2017)",
    ]
    for example in simple_examples:
        assert get_citations(example)[0].full_span() == (0, len(example)), example
    stopword_examples = [
        ("See 66 B.U. L. Rev. 71 (1986)", 4),
        ("Citing 66 B.U. L. Rev. 71 (1986)", 7),
    ]
    for sentence, start_idx in stopword_examples:
        assert get_citations(sentence)[0].full_span() == (start_idx, len(sentence))
