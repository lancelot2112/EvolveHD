"""Build an echo stream (act path, stage 3) from a word stream: after every Nth fact sentence S, insert
"echo", then S again as self slots (the reader says it and hears its own words; each id carries bit 31 and the
teacher's word in the low bits), then S once more as ordinary input (the world repeats it). Answer and other
positions in the info file are shifted; info["echo"] lists each trial as [first self slot, length].
usage: echo_stream.py SRC DST [N] [--no-repeat]"""
import sys, json
import numpy as np
src, dst = sys.argv[1], sys.argv[2]
N = int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3].isdigit() else 3
repeat = "--no-repeat" not in sys.argv
ids = np.fromfile(src + ".ids", dtype=np.uint32).tolist()
voc = open(src + ".txt").read().split("\n")
while voc and voc[-1] == "":
    voc.pop()
ECHO = len(voc); voc.append("echo")
dot, q = voc.index("."), voc.index("?")
out, newpos, trials = [], [], []
sent, nfact = [], 0
for i, w in enumerate(ids):
    newpos.append(len(out)); out.append(w); sent.append(w)
    if w == dot:
        # a fact sentence: ends with ".", no "?", longer than an answer line ("kitchen .")
        if q not in sent and len(sent) >= 4 and voc[sent[0]] != "new":
            nfact += 1
            if nfact % N == 0:
                out.append(ECHO)
                trials.append([len(out), len(sent)])
                out.extend(int(x) | (1 << 31) for x in sent)
                if repeat:
                    out.extend(sent)
        sent = []
    elif w == q:
        sent = [] if False else sent
newpos.append(len(out))
info = json.load(open(src + ".info.json"))
def shift(p): return newpos[p]
for k, v in list(info.items()):
    if k == "answers":
        info[k] = [[shift(a[0])] + list(a[1:]) for a in v]
    elif k in ("newq", "newfact"):
        info[k] = [shift(p) for p in v]
info["echo"] = trials
info["pos_map_note"] = "positions shifted by echo trials; use info['shift'] for other source positions"
info["shift"] = {str(p): shift(p) for p in (102303, 156952, 220618, len(ids) // 2) if p <= len(ids)}
np.array(out, dtype=np.uint32).tofile(dst + ".ids")
open(dst + ".txt", "w").write("\n".join(voc) + "\n")
json.dump(info, open(dst + ".info.json", "w"))
print(f"{len(ids)} -> {len(out)} words, {len(trials)} echo trials, shifts {info['shift']}")
