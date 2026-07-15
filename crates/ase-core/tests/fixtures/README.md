# Test fixtures

Three sources, all MIT-licensed:

- `aseprite-tests/` — 20 first-party test sprites from the Aseprite source
  repo's `tests/` subtree (github.com/aseprite/aseprite). The main app is
  source-available/EULA, but `tests/` carries its own MIT grant — see
  `aseprite-tests/LICENSE.txt` (Igara Studio S.A. / David Capello). Covers
  tilemaps, tag repeat counts, groups, slices, linked cels, indexed +
  background, empty frames, custom properties.
- `asefile/` — fixtures + Aseprite-rendered golden PNGs from
  github.com/alpine-alpaca/asefile (`tests/data/`), MIT — see
  `asefile/LICENSE`. Covers old-format palette chunks, grayscale/indexed
  tilemaps, raw cels, cel overflow, user data, slices. The large `blend_*`
  files were deliberately not vendored (~10 MB); we generate our own small
  equivalents instead.
- `generated/` — produced by `tools/corpus/generate.sh` driving a real
  Aseprite headlessly (dev machines only; outputs are committed so CI needs
  no Aseprite). Each `.aseprite` has a matching `.png` golden flattened by
  Aseprite itself — the reference implementation renders our expected output.
  `GENERATED_BY.txt` records the Aseprite version used.

When adding fixtures: prefer extending `tools/corpus/gen_corpus.lua` (small,
systematic, regenerable). Hand-vendored files need a license note here.
