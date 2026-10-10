# Fonts

Where each font file came from, what was done to it and the terms it is
under. Every font here is under the SIL Open Font License, Version 1.1.

Built with fonttools 4.60.2. Building again from the same upstream files gives
the same bytes.

## The starter's fonts

The two fonts `statac new --cards` writes into a site as the fonts of its
sharing cards, each with its licence text beside it. They are in
`starter/site/templates/og/`, not in this folder.

### starter/site/templates/og/NotoSans-Regular.ttf

Noto Sans Regular, version 2.007, a static font. The upstream file,
unmodified. 560,092 bytes, 4,702 glyphs, 3,053 mapped characters. Its name
table gives the version as "Version 2.007" and its unique name as
"2.007;GOOG;NotoSans-Regular".

- Source: `ofl/notosans/NotoSans-Regular.ttf` in
  <https://github.com/google/fonts>, at commit
  `a970ecbe0bf1e6ae5cb22d6abdea672448655f0c`, "Noto Sans: 2.007 added".
- URL: <https://raw.githubusercontent.com/google/fonts/a970ecbe0bf1e6ae5cb22d6abdea672448655f0c/ofl/notosans/NotoSans-Regular.ttf>
- SHA-256, upstream and here:
  `a3c4b14201354e76f360300cccf688162280b90ea1639092f1e76706a040fab4`
- Copyright: "Copyright 2015-2021 Google LLC. All Rights Reserved."
- Licence: SIL OFL 1.1, as the font's name table says. The text is
  `starter/site/templates/og/OFL.txt`, which `statac new --cards` writes
  beside the font.
- Reserved Font Name: none declared. The licence file's only notice is that
  copyright line.

The font's own name table says "Noto is a trademark of Google LLC."

### starter/site/templates/og/OFL.txt

The licence text published beside Noto Sans 2.007, unmodified.

- Source: `ofl/notosans/OFL.txt` at the same commit of
  <https://github.com/google/fonts>.
- SHA-256, upstream and here:
  `3a579abd469a5b75b779f244e4cebcb74451e32154503fce2c8e004d15bc7e98`

### starter/site/templates/og/NotoEmoji.ttf

Noto Emoji, the monochrome emoji font, version 3.002. The upstream file,
unmodified: only the file name differs. It is a variable font with one axis,
weight, from 300 to 700 with 400 as the default. 1,891 glyphs, 1,489 mapped
characters.

- Source: `ofl/notoemoji/NotoEmoji[wght].ttf` in
  <https://github.com/google/fonts>, at commit
  `9710da1eacb3be272583c3224dcb70f9da6eadbb`. The file last changed in commit
  `b979dba422e445492b0eb9951ac52ee0b4d648c3`, "Update Noto Emoji to the 3.002
  release".
- URL: <https://raw.githubusercontent.com/google/fonts/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/notoemoji/NotoEmoji%5Bwght%5D.ttf>
- SHA-256, upstream and here:
  `de6c18832938afc99caf132b39d6a30a19bac7f2e812e28db2535b4608d27551`
- Copyright: "Copyright 2013 Google LLC"
- Licence: SIL OFL 1.1. The text is
  `starter/site/templates/og/NotoEmoji-OFL.txt`, which `statac new --cards`
  writes beside the font.
- Reserved Font Name: none declared. The licence file's only notice is
  "Copyright 2013 Google LLC".

The font's own name table says "Noto is a trademark of Google Inc."

### starter/site/templates/og/NotoEmoji-OFL.txt

The licence text published beside the emoji font, unmodified: only the file
name differs.

- Source: `ofl/notoemoji/OFL.txt` at the same commit of
  <https://github.com/google/fonts>.
- SHA-256, upstream and here:
  `500bb1ccf43df7bbb522112f9133a52b16e1c35e809632f5d8609b179152de5b`

## The test fonts

Small subsets of Noto fonts for automated tests. Each is a Modified Version
under the licence, so each has its own family, full and PostScript names. The
copyright, licence, trademark and credit strings in each name table are as
upstream wrote them, and the description says which font and version the
subset was cut from.

Every subset was made with `pyftsubset` and these options, with the layout
features and characters given under each font:

```
pyftsubset SOURCE --output-file=OUTPUT --layout-features=FEATURES \
  --no-hinting --name-IDs=0,1,2,3,4,5,6,7,8,9,10,11,12,13,14 \
  --name-languages='*' --notdef-outline --no-recalc-timestamp CHARACTERS
```

Afterwards the names were replaced, the `STAT` table was removed, and name
records no longer referred to by anything were dropped.

The Japanese and Korean sources are variable fonts. After subsetting, each was
fixed at weight 400 to make a static font:

```
fonttools varLib.instancer INPUT wght=400 --output=OUTPUT --no-recalc-timestamp
```

### test/test-latin.ttf

Family name "Statac Test Latin". 25,956 bytes, 256 glyphs.

- Source: `NotoSans/unhinted/ttf/NotoSans-Regular.ttf`, Noto Sans version
  2.015, from the release archive
  <https://github.com/notofonts/latin-greek-cyrillic/releases/download/NotoSans-v2.015/NotoSans-v2.015.zip>.
  Tag `NotoSans-v2.015`, commit `c4a321e123e4d4ff315f57f4e0adf294fe3a95be`.
- SHA-256 of the archive:
  `0c34df072a3fa7efbb7cbf34950e1f971a4447cffe365d3a359e2d4089b958f5`
- SHA-256 of the upstream font:
  `f3961a9cde016d41a4879aecda1474d3a36d6bf54fa0e4643de029cc2248b0e8`
- SHA-256 of this file:
  `fac2dcde2a1238ca571d5b9abeac6a3cad31be1704d830f21291114444662746`
- Copyright: "Copyright 2022 The Noto Project Authors
  (https://github.com/notofonts/latin-greek-cyrillic)"
- Licence: SIL OFL 1.1. The text is `test/OFL.txt`.
- Reserved Font Name: none declared. The archive's `OFL.txt` has that
  copyright line as its only notice.
- Layout features: `*`, which is all of them.
- Characters:
  `U+0020-007E,U+00A0,U+00E7,U+00E9,U+00F1,U+00FC,U+2013,U+2014,U+2018,U+2019,U+201C,U+201D,U+2026`.
  That is printable ASCII, the no-break space, ç, é, ñ, ü, the en and em
  dashes, the single and double curly quotes and the ellipsis.

### test/test-latin-letters-only.ttf

Family name "Statac Test Latin Letters Only". 19,528 bytes, 174 glyphs. The
same as `test-latin.ttf` without the digits `U+0030-0039`, so that text with
digits falls through to another font.

- Source, archive, copyright, licence and Reserved Font Name position: as for
  `test-latin.ttf`.
- SHA-256 of this file:
  `7cb03a3ec8681a89b014777857d4651d2dc76234f0056bee0eb451cd715e1dcc`
- Layout features: `*`.
- Characters:
  `U+0020-002F,U+003A-007E,U+00A0,U+00E7,U+00E9,U+00F1,U+00FC,U+2013,U+2014,U+2018,U+2019,U+201C,U+201D,U+2026`

### test/test-arabic.ttf

Family name "Statac Test Arabic". 22,920 bytes, 178 glyphs.

- Source: `NotoSansArabic/unhinted/ttf/NotoSansArabic-Regular.ttf`, Noto Sans
  Arabic version 2.013, from the release archive
  <https://github.com/notofonts/arabic/releases/download/NotoSansArabic-v2.013/NotoSansArabic-v2.013.zip>.
  Tag `NotoSansArabic-v2.013`, commit
  `1b2b7e5c6ce3ab4d50681c854892325530084c35`.
- SHA-256 of the archive:
  `1301aceaea84c501cf2e6dcfb3182e2328c8eae5725817fcb239672bda7154f1`
- SHA-256 of the upstream font:
  `bd86ca02f087d7f3c3788ba458fb6b73744c7639ed276b8d870dba6def6c40d0`
- SHA-256 of this file:
  `c891c838323fc3ab9dffdf2c45de4e2ee1e8c46048ca5d915fcff38af6f7f73e`
- Copyright: "Copyright 2022 The Noto Project Authors
  (https://github.com/notofonts/arabic)"
- Licence: SIL OFL 1.1. The text is `test/OFL.txt`.
- Reserved Font Name: none declared. The archive's `OFL.txt` has that
  copyright line as its only notice.
- Layout features: `*`. The subset keeps `ccmp`, `init`, `medi`, `fina`,
  `rlig`, `locl`, `dlig`, `aalt`, `pnum` and `tnum` substitutions and `kern`,
  `mark` and `mkmk` positioning.
- Characters:
  `U+0020,U+060C,U+061B,U+061F,U+0621-064A,U+0660-066D,U+067E,U+0686,U+0698,U+06A9,U+06AF,U+06CC,U+06D4,U+06F0-06F9,U+200C,U+200D`.
  That is the space; the Arabic comma, semicolon and question mark; the
  letters hamza to yeh, tatweel among them; the Arabic-Indic digits with the
  percent sign, decimal and thousands separators and five-pointed star; the
  Persian peh, tcheh, jeh, keheh, gaf and Farsi yeh; the Arabic full stop; the
  Extended Arabic-Indic digits; and the zero-width non-joiner and joiner.

The Arabic presentation forms (`U+FB50-FDFF`, `U+FE70-FEFF`) are not mapped.
The font reaches its joining forms through the layout features, splitting
most letters into a base and its dots in `ccmp` and then substituting the
base in `init`, `medi` and `fina`, so a shaper does not need them. The vowel
marks `U+064B-0652` are not in the subset.

### test/test-japanese.ttf

Family name "Statac Test Japanese". 56,900 bytes, 239 glyphs.

- Source: `ofl/notosansjp/NotoSansJP[wght].ttf`, Noto Sans JP version 2.004,
  in <https://github.com/google/fonts> at commit
  `9710da1eacb3be272583c3224dcb70f9da6eadbb`.
- URL: <https://raw.githubusercontent.com/google/fonts/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/notosansjp/NotoSansJP%5Bwght%5D.ttf>
- SHA-256 of the upstream font:
  `c2f3b4d463500a2ddcd3849cded1fceeb9fd6d1c32e6cbecd568453ba50fc68f`
- SHA-256 of this file:
  `f6fc4b7ed26ae68cb916f36a9edd72b3f70d1af78fa405f302b40e8f710bdae3`
- Copyright: "Copyright 2014-2021 Adobe (http://www.adobe.com/), with Reserved
  Font Name 'Source'"
- Licence: SIL OFL 1.1. The text is `test/OFL.txt`. The upstream text is
  `ofl/notosansjp/OFL.txt` at the same commit, SHA-256
  `1c05c68c34f9708415aada51f17e1b0092d2cea709bf4a94cd38114f9e73d7d9`.
- Reserved Font Name: "Source", declared by Adobe in the copyright line quoted
  above. The subset's names do not use it. The word remains only in the
  copyright and trademark strings, which the licence requires to be kept.
- Layout features: `kern,palt,ccmp,locl,liga,mark`. The subset keeps `ccmp`
  substitutions and `kern` and `palt` positioning.
- Characters: `U+0020,U+3000-3002,U+300C-300F,U+3041-3096,U+3099-309F,U+30A0-30FF`
  and the kanji `日本語東京文字時間人大小中新聞書写真`. That is the space; the
  ideographic space, comma and full stop; the single and double corner
  brackets; every hiragana, with the combining and spacing sound marks and
  iteration marks; the whole katakana block, the prolonged sound mark
  `U+30FC` among them; and eighteen kanji.

This file is over 40 KB because the outlines of the 189 hiragana and katakana
characters alone come to about 38 KB, and the whole file to about 45 KB with no layout features at
all. Keeping every feature brings in the vertical, half-width, ruby and other
alternates of each kana and gives 187 KB, so the features kept are those that
bear on horizontal text. The vertical features `vert`, `vrt2`, `vkrn`, `vpal`
and `vhal` are among those dropped.

### test/test-korean.ttf

Family name "Statac Test Korean". 25,844 bytes, 203 glyphs.

- Source: `ofl/notosanskr/NotoSansKR[wght].ttf`, Noto Sans KR version 2.004,
  in <https://github.com/google/fonts> at commit
  `9710da1eacb3be272583c3224dcb70f9da6eadbb`.
- URL: <https://raw.githubusercontent.com/google/fonts/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/notosanskr/NotoSansKR%5Bwght%5D.ttf>
- SHA-256 of the upstream font:
  `194018e6b2b293a7964f037b25c0249ce1418bc9ab3c971060a03aa57861e252`
- SHA-256 of this file:
  `dd67bc63fc486999789f535663a9439a5721706e3bc2b38402d57f226fda91fa`
- Copyright: "Copyright 2014-2021 Adobe (http://www.adobe.com/), with Reserved
  Font Name 'Source'"
- Licence: SIL OFL 1.1. The text is `test/OFL.txt`. The upstream text is
  `ofl/notosanskr/OFL.txt` at the same commit, identical to the Japanese one.
- Reserved Font Name: "Source", as for the Japanese font. The subset's names
  do not use it.
- Layout features: `*`. The subset keeps `ljmo`, `vjmo`, `tjmo`, `ccmp`,
  `calt`, `locl`, `aalt`, `fwid`, `hwid` and `pwid` substitutions.
- Characters: the text `한국어 번역 상위 활동 사용자 서울 글자`, which is the
  space and the fifteen syllables
  `U+AD6D,U+AE00,U+B3D9,U+BC88,U+C0AC,U+C0C1,U+C11C,U+C5B4,U+C5ED,U+C6A9,U+C6B8,U+C704,U+C790,U+D55C,U+D65C`,
  and the conjoining jamo those syllables decompose to:
  `U+1100,U+1103,U+1107,U+1109,U+110B,U+110C,U+1112,U+1161,U+1165,U+1167,U+1169,U+116A,U+116D,U+116E,U+1171,U+1173,U+11A8,U+11AB,U+11AF,U+11BC`

### test/OFL.txt

One licence file for the five test fonts. It is the first line of each of the
three upstream licence files, which are their copyright notices, followed by
the Noto Sans licence file from its second line on.

SHA-256: `af84472f3fd4e9846284fc10db42e03c2b79db740c38aca5daf02ebcf662a5e5`

The three upstream files are the same from the second line on, except that
the Noto Sans Arabic one gives the address of the FAQ as
`https://openfontlicense.org` where the others give
`https://scripts.sil.org/OFL`.

| Upstream licence file | SHA-256 |
| --- | --- |
| `OFL.txt` in `NotoSans-v2.015.zip` | `cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a` |
| `OFL.txt` in `NotoSansArabic-v2.013.zip` | `a7a5a25eb188bf1cd96982030d53e23c33485c69b1044a562254226857ee13af` |
| `ofl/notosansjp/OFL.txt` and `ofl/notosanskr/OFL.txt` | `1c05c68c34f9708415aada51f17e1b0092d2cea709bf4a94cd38114f9e73d7d9` |
