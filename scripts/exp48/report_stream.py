"""Act path, stage 4 (goal-directed acts): turn every other "where is NP ? ANS ." into a report request.
The world says "report NP"; the reader says "NP is in the ANS ." as self slots (each id carries bit 31 and the
teacher's word); then the world says the same sentence (the teacher). The other questions stay questions.
info["report"]: [first self slot, length, offset of ANS in the act]. Answer positions and splits are shifted.
usage: report_stream.py SRC DST"""
import sys, json
import numpy as np
src, dst = sys.argv[1], sys.argv[2]
ids = np.fromfile(src + ".ids", dtype=np.uint32).tolist()
voc = open(src + ".txt").read().split("\n")
while voc and voc[-1] == "":
    voc.pop()
def wid(w):
    if w not in voc:
        voc.append(w)
    return voc.index(w)
REPORT, IN = wid("report"), wid("in")
WHERE, IS, Q, DOT, THE = voc.index("where"), voc.index("is"), voc.index("?"), voc.index("."), voc.index("the")
info = json.load(open(src + ".info.json"))
story = "names" in info
ans_at = {a[0]: a for a in info["answers"]} if not story else {p: (p, k) for p, k in info["answers"]}
out, newpos, reports, kept = [], [None] * (len(ids) + 1), [], []
converted = set()
i, nq = 0, 0
while i < len(ids):
    # a where-question: where is NP ? ANS (.)
    if ids[i] == WHERE and i + 1 < len(ids) and ids[i + 1] == IS:
        j = i + 2
        while j < len(ids) and ids[j] not in (Q, DOT) and j - i < 8:
            j += 1
        if j < len(ids) and ids[j] == Q and j + 1 in ans_at:
            nq += 1
            if nq % 2 == 0:
                np_ = ids[i + 2:j]; ans = ids[j + 1]; converted.add(j + 1)
                sent = np_ + [IS, IN, THE, ans, DOT]
                for k in range(i, j + 2):
                    newpos[k] = len(out)
                out.extend([REPORT] + np_)
                reports.append([len(out), len(sent), len(np_) + 3, ans_at[j + 1][1] if story else ans_at[j + 1][1]])
                out.extend(int(x) | (1 << 31) for x in sent)
                out.extend(sent)
                i = j + 2
                if i < len(ids) and ids[i] == DOT:
                    newpos[i] = len(out) - 1
                    i += 1
                continue
    newpos[i] = len(out); out.append(ids[i]); i += 1
newpos[len(ids)] = len(out)
for k in range(len(newpos) - 1, -1, -1):
    if newpos[k] is None:
        newpos[k] = newpos[k + 1]
inf2 = dict(info)
keep = []
for a in info["answers"]:
    p = a[0]
    # an answer converted into a report is no longer a question
    if p in converted:
        continue
    keep.append([newpos[p]] + list(a[1:]))
inf2["answers"] = [tuple(a) if story else a for a in keep]
if story:
    inf2["answers"] = [[a[0], a[1]] for a in keep]
for k in ("newq", "newfact"):
    if k in info:
        inf2[k] = [newpos[p] for p in info[k]]
inf2["report"] = reports
inf2["shift"] = {str(p): newpos[p] for p in (102303, 156952, 220618, len(ids) // 2) if p <= len(ids)}
np.array(out, dtype=np.uint32).tofile(dst + ".ids")
open(dst + ".txt", "w").write("\n".join(voc) + "\n")
json.dump(inf2, open(dst + ".info.json", "w"))
print(f"{len(ids)} -> {len(out)} words, {len(reports)} reports, {len(keep)} questions kept, shifts {inf2['shift']}")
