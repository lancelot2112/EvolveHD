"""Two books one after the other, scored on the second only: a reader trained on book A then reads book B.
usage: concat_books.py A_PREFIX B_PREFIX OUT_PREFIX (the two must share a vocabulary)"""
import sys, json, shutil
import numpy as np
a, b, out = sys.argv[1:4]
assert open(a + ".txt").read() == open(b + ".txt").read(), "the books must share a vocabulary"
ia, ib = np.fromfile(a + ".ids", dtype=np.uint32), np.fromfile(b + ".ids", dtype=np.uint32)
np.concatenate([ia, ib]).tofile(out + ".ids")
shutil.copy(a + ".txt", out + ".txt")
info = json.load(open(b + ".info.json"))
off = len(ia)
info["answers"] = [[p + off, k] for p, k in info["answers"]]
info["words"] = len(ia) + len(ib)
info["book_a"], info["book_a_words"] = a, off
json.dump(info, open(out + ".info.json", "w"))
print(f"{len(ia)} + {len(ib)} words; {len(info['answers'])} questions, all in the second book (from word {off})")
