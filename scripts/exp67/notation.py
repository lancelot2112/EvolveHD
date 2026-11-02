"""Experiment 67: the magic bytes of each format's first file, by how its manual entry writes each byte: a hex escape
(\\x89), an octal escape (\\037), a plain character (P), another escape (\\n, \\<), or a hex number (0x4a4d0100: "be"
in the order written, "le" reversed). Per notation class: the mean cost per byte under each condition (Rm: its entry,
Rn: the note, W: a wrong entry, N: nothing), pooled over the four rotations; and paired per format and byte (the
same byte under Rm and W), Rm minus W with its standard error and how many bytes were cheaper after the right entry.
usage: notation.py ITEMS_PATTERN NAME=BITS_PATTERN ...   (patterns contain {s})"""
import sys, json, re, os
import numpy as np

items_pat = sys.argv[1]
runs = [(a.split("=")[0], a.split("=")[1]) for a in sys.argv[2:]]
here = os.path.dirname(os.path.abspath(items_pat.format(s=0)))
fmts = json.load(open(os.path.join(here, "formats.json")))


def classes(entry, test):
    """The notation class of each byte of the test, from the entry's test line."""
    off, hexb = test
    n = len(hexb) // 2
    for line in entry.split("\n"):
        f = re.split(r"\s+", line.strip(), maxsplit=3)
        if len(f) < 3 or not re.fullmatch(r">?\d+|>?0x[0-9a-fA-F]+", f[0]) or int(f[0].lstrip(">"), 0) != off:
            continue
        typ, val = f[1].split("&")[0].split("/")[0], f[2]
        if typ == "string":
            out, i = [], 0
            v = val.lstrip("=")
            while i < len(v) and len(out) < n:
                if v[i] != "\\":
                    out.append("char"); i += 1
                elif i + 1 < len(v) and v[i + 1] == "x":
                    m = re.match(r"[0-9a-fA-F]{1,2}", v[i + 2:]).group(0); out.append("hex"); i += 2 + len(m)
                elif i + 1 < len(v) and v[i + 1] in "01234567":
                    m = re.match(r"[0-7]{1,3}", v[i + 1:]).group(0); out.append("octal"); i += 1 + len(m)
                else:
                    out.append("escape"); i += 2
            if len(out) == n:
                return out
        elif re.fullmatch(r"u?(be|le)?(byte|short|long|quad)", typ):
            return ["hexnum-le" if typ.lstrip("u").startswith("le") else "hexnum"] * n
    return None


cls = {}
for x in fmts:
    c = classes(x["magic"], x["test"]) if x["test"] else None
    cls[x["name"]] = c
    print(f"{x['name']:9} {' '.join(c) if c else '?'}", file=sys.stderr)
C = ["Rm", "Rn", "W", "N"]
acc = {}
for s in range(4):
    meta = json.load(open(items_pat.format(s=s)))["items"]
    bits = {nm: np.fromfile(p.format(s=s), dtype="<f4").astype(float) for nm, p in runs}
    for it in meta:
        if it["k"] or not it["magic"] or not cls[it["name"]]:
            continue
        x = it["start"] + it["magic"][0]
        for j, c in enumerate(cls[it["name"]]):
            for nm, b in bits.items():
                acc.setdefault((nm, c, it["name"], j), {})[it["cond"]] = b[x + j]
order = ["hex", "hexnum", "hexnum-le", "octal", "char", "escape"]
print(f"{'run':<10}{'notation':<11}{'bytes':>6}{'formats':>8} | " + " ".join(f"{c:>5}" for c in C) + " | Rm-W (se, cheaper)      | Rn-W (se, cheaper)")
for nm, _ in runs:
    for c in order:
        ks = [k for k in acc if k[0] == nm and k[1] == c and len(acc[k]) == 4]
        if not ks:
            continue
        v = {q: np.array([acc[k][q] for k in ks]) for q in C}
        d = v["Rm"] - v["W"]; e = v["Rn"] - v["W"]
        nf = len(set(k[2] for k in ks))
        print(f"{nm:<10}{c:<11}{len(ks):>6}{nf:>8} | " + " ".join(f"{v[q].mean():5.2f}" for q in C)
              + f" | {d.mean():+5.2f} ({d.std(ddof=1) / np.sqrt(len(d)):4.2f}, {np.sum(d < 0):>3}) | {e.mean():+5.2f} ({e.std(ddof=1) / np.sqrt(len(e)):4.2f}, {np.sum(e < 0):>3})")
