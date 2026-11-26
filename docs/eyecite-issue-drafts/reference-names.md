# Draft: `DISALLOWED_NAMES` never matches its surnames, and a fused entry lets "Commissioner" through

Not filed. Written for [freelawproject/eyecite](https://github.com/freelawproject/eyecite),
checked against eyecite 2.7.8.

---

**Title:** `is_valid_name` compares a lowercased name with capitalized `DISALLOWED_NAMES` entries

`is_valid_name` returns `name.lower() not in DISALLOWED_NAMES`. The list's Attorney General
surnames are capitalized (`"Akerman"`, `"Johnson"`, `"Miller"`, ...), so a lowercased name never
equals one of them and none is excluded. Only the lowercase entries at the top work, and one of
those doesn't: there is no comma after `"commissioner"`, so Python joins it with the next string
into `"commissionerAkerman"`.

```python
from eyecite.utils import is_valid_name

print([is_valid_name(n) for n in ["Johnson", "Miller", "Garland", "Commissioner", "State"]])
```

```text
[True, True, True, True, False]
```

Fixing the case comparison as written would make things worse. Surnames like Johnson, Miller,
Smith, Rogers, Williams and Jackson are the names courts most often use to refer back to a case
(`In Rogers, ...`, `Under the <i>Miller</i> test`), and over 7,632 CourtListener opinions (the
benchmark's ten-percent sample) the references eyecite finds today by those names almost always
name the one case of that name the opinion cites. The false positives the list was meant to stop
come from a name that several cited cases share, which a fixed list can't tell from one that only
one case has.

**Suggested fix:** add the missing comma and keep only the generic party words (`state`, `united
states`, `people`, `commonwealth`, `mass`, `commissioner`), and treat a reference name as
ambiguous when two or more different decisions cited in full before it have that party name. citare
does this; its write-up, with the measurement, is
<https://github.com/codexpublicus/citare/blob/main/docs/divergences.md#reference-names>.
