# Data

`crates/citare-core/build.rs` compiles the files here, except `edition-slugs.json`, into
the data bundle that citare-core embeds by default
([docs/data-bundle.md](../docs/data-bundle.md)). `edition-slugs.json` is read only by a
test.

| Path | What it is | Licence |
|------|------------|---------|
| `reporters-local.json` | citare's supplement to reporters-db's reporters.json ([docs/reporters-local.md](../docs/reporters-local.md)) | [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/) |
| `laws-local.json` | citare's supplement to reporters-db's laws.json ([docs/laws-local.md](../docs/laws-local.md)) | [CC BY-NC-ND 4.0](https://creativecommons.org/licenses/by-nc-nd/4.0/) |
| `reporters-db/` | Free Law Project's [reporters-db](https://github.com/freelawproject/reporters-db), a git submodule | BSD 2-Clause ([THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)) |
| `courts-db/` | Free Law Project's [courts-db](https://github.com/freelawproject/courts-db), a git submodule | BSD 2-Clause ([THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)) |
| `harvard_reporters.json` | the Caselaw Access Project's reporter list, for CAP slugs | |
| `cap_volumes.json` | the first and last volume CAP has of each reporter, by CAP slug | |
| `edition-slugs.json` | every edition name's slugs, which a test holds stable | |

The two supplements are licensed under the Creative Commons
Attribution-NonCommercial-NoDerivatives 4.0 International licence (CC BY-NC-ND 4.0), the
terms stare.pub's source lists credit them under; credit citare
(https://github.com/codexpublicus/citare). citare's code is under the
[PolyForm Shield 1.0.0](../LICENSE.md) licence, which doesn't cover them.
