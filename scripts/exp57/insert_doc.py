"""Insert N bytes of a document (from its first ".SH DESCRIPTION" on) into the byte stream just before a given file,
to test whether reading a format's documentation first helps read the format. Writes OUT.bin and OUT.files.json.
usage: insert_doc.py STREAM OUT DOC BEFORE_INDEX [N]"""
import sys, json
s, out, doc, at = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]); N = int(sys.argv[5]) if len(sys.argv) > 5 else 24000
data = open(s + ".bin", "rb").read(); meta = json.load(open(s + ".files.json"))["files"]
raw = open(doc, "rb").read(); i = raw.find(b".SH DESCRIPTION"); raw = raw[max(i, 0):max(i, 0) + N]
assert len(raw) == N, len(raw)
a = meta[at]["start"]
new = data[:a] + raw + data[a:]
m2 = []
for j, m in enumerate(meta):
    if j == at:
        m2.append({"name": doc, "type": "doc", "round": m["round"], "start": a, "len": N})
    mm = dict(m)
    if j >= at:
        mm["start"] += N
    m2.append(mm)
open(out + ".bin", "wb").write(new); json.dump({"files": m2, "bytes": len(new)}, open(out + ".files.json", "w"), indent=1)
print(out, len(new), file=sys.stderr)
