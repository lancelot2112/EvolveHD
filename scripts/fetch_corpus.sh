#!/bin/sh
# Fetch the NLTK Gutenberg corpus and cut the two books used by the text experiments.
# Book A: Austen, Persuasion (300,000 bytes); book B: Chesterton, The Man Who Was Thursday (100,000 bytes).
# Both skip the first 2,000 bytes (header) and normalise CRLF to LF.
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
PY
