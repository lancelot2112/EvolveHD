"""BABILong (RMT-team/babilong on Hugging Face) as one word stream for the reader (experiment 38, step 6).
Each sample is a document: book text (PG-19) with bAbI facts hidden in it, then its question and answer. The stream
is the documents one after another, each as: its words, then the question, then the answer word, then ".".
Words: lower-cased; letters, digits and apostrophes make a word; "." "?" "!" "," ";" ":" are words of their own;
other characters (quotes, dashes, brackets) are dropped. Answers are scored at the answer's position.
usage: babilong_stream.py OUT_PREFIX TASK:LENGTH:N ... (e.g. qa1:0k:100 qa1:4k:100; files read from runs/exp38/babilong)"""
import sys, json, re
import numpy as np
out = sys.argv[1]
tok = re.compile(r"[a-z0-9']+|[.?!,;:]")
vocab, ix, ids, answers, meta = [], {}, [], [], []
def wid(w):
    if w not in ix: ix[w] = len(vocab); vocab.append(w)
    return ix[w]
for spec in sys.argv[2:]:
    task, length, n = spec.split(":")
    data = json.load(open(f"runs/exp38/babilong/{task}_{length}.json"))
    for k, s in enumerate(data[:int(n)]):
        start = len(ids)
        ids.extend(wid(w) for w in tok.findall(s["input"].lower()))
        ids.extend(wid(w) for w in tok.findall(s["question"].lower()))
        if not ids or vocab[ids[-1]] != "?": ids.append(wid("?"))
        ans = tok.findall(str(s["target"]).lower())
        answers.append([len(ids), task, length, k, len(ids) - start])
        ids.extend(wid(w) for w in ans); ids.append(wid("."))
np.array(ids, dtype=np.uint32).tofile(out + ".ids")
open(out + ".txt", "w").write("\n".join(vocab))
json.dump({"answers": answers, "words": len(ids), "specs": sys.argv[2:]}, open(out + ".info.json", "w"))
print(f"{len(ids)} words, vocabulary {len(vocab)}, {len(answers)} questions")
