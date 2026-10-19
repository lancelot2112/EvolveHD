"""Write a text as a stream of word ids (u32 LE): lower-cased words and punctuation, each id its word's
frequency rank (0 = commonest), plus the words one per line. usage: word_ids.py BIN NBYTES OUT.ids OUT.txt"""
import sys, re
import numpy as np
from collections import Counter
t = open(sys.argv[1], 'rb').read().decode('latin-1')[:int(sys.argv[2])]
toks = [w.lower() for w in re.findall(r"[A-Za-z']+|[^\sA-Za-z']", t)]
order = [w for w, _ in Counter(toks).most_common()]; ix = {w: i for i, w in enumerate(order)}
np.array([ix[w] for w in toks], dtype=np.uint32).tofile(sys.argv[3])
open(sys.argv[4], 'w', encoding='latin-1').write('\n'.join(w.replace('\n', ' ') for w in order) + '\n')
print(len(toks), 'words,', len(order), 'distinct')
