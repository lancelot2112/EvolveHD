#!/bin/sh
# Fetch the NLTK Gutenberg corpus and cut the two books used by the text experiments.
# Book A: Austen, Persuasion (300,000 bytes); book B: Chesterton, The Man Who Was Thursday (100,000 bytes).
# Both skip the first 2,000 bytes (header) and normalise CRLF to LF.
# Also writes corpus/gutenberg_all.bin: all 18 texts concatenated, with offsets in gutenberg_all.idx.
set -e
cd "$(dirname "$0")/.."
mkdir -p corpus/raw
[ -f corpus/raw/gutenberg.zip ] || curl -sSL -o corpus/raw/gutenberg.zip \
  https://raw.githubusercontent.com/nltk/nltk_data/gh-pages/packages/corpora/gutenberg.zip
unzip -o -q corpus/raw/gutenberg.zip -d corpus/raw
python3 - <<'PY'
def cut(src, n, skip=2000):
    b = open(src, 'rb').read().replace(b'\r\n', b'\n')
    return b[skip:skip + n]
open('corpus/bookA.bin', 'wb').write(cut('corpus/raw/gutenberg/austen-persuasion.txt', 300000))
open('corpus/bookB.bin', 'wb').write(cut('corpus/raw/gutenberg/chesterton-thursday.txt', 100000))
print('corpus/bookA.bin and corpus/bookB.bin written')
# the whole corpus as one stream (18 texts, alphabetical, CRLF -> LF), with an index of offsets
import os
names = sorted(f for f in os.listdir('corpus/raw/gutenberg') if f.endswith('.txt'))
off, out, idx = 0, open('corpus/gutenberg_all.bin', 'wb'), open('corpus/gutenberg_all.idx', 'w')
for n in names:
    b = open('corpus/raw/gutenberg/' + n, 'rb').read().replace(b'\r\n', b'\n')
    out.write(b)
    idx.write(f'{n[:-4]}\t{off}\t{len(b)}\n')
    off += len(b)
print(f'corpus/gutenberg_all.bin written: {len(names)} texts, {off} bytes')
PY
