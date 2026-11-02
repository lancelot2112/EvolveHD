"""Experiment 66: real files after real manuals. A stream of real files from this machine (the first L bytes of each),
of 28 formats, each file preceded by one of: its format's entry in file(1)'s magic database, verbatim ("Rm"); a note
that translates that entry's test into the hex notation of experiment 60's stream ("Rn"); another format's magic entry
("W"); or nothing ("N"). Formats appear progressively. Four rotations of the same files and order: rotation s gives the
k-th file of format f condition (f + k + s) mod 4, so across the four every file has every condition.
Also writes each rotation after experiment 60's synthetic formats stream (pre*), for runs that learn the notation and
the role first.
usage: build_real.py TYPES_TSV MAGDIR OUTDIR [L]"""
import sys, json, re, random, hashlib, os
types_tsv, magdir, outdir = sys.argv[1:4]
L = int(sys.argv[4]) if len(sys.argv) > 4 else 1024
K = 6  # files per format
os.makedirs(outdir, exist_ok=True)

# name, file(1) output pattern, magic file, line of the entry (1-based)
FORMATS = [
    ("png", r"PNG image data", "images", 522),
    ("tzif", r"timezone data", "timezone", 31),
    ("tfm", r"TeX font metric data", "tex", 42),
    ("vf", r"TeX virtual font data", "tex", 21),
    ("mo", r"GNU message catalog \(little", "gnu", 15),
    ("lz4", r"LZ4 compressed data \(v1\.4", "compress", 298),
    ("gzip", r"gzip compressed data", "compress", 35),
    ("bzip2", r"bzip2 compressed data", "compress", 155),
    ("xcursor", r"X11 cursor", "images", 2779),
    ("gvariant", r"GVariant Database", "gnome", 46),
    ("llvmbc", r"LLVM IR bitcode", "llvm", 22),
    ("ar", r"current ar archive", "archive", 555),
    ("woff2", r"Web Open Font Format \(Version 2", "fonts", 446),
    ("otf", r"OpenType font data", "fonts", 410),
    ("ttf", r"TrueType Font data", "fonts", 269),
    ("pyc", r"Byte-compiled Python module for CPython 3\.12", "python", 205),
    ("elf", r"ELF 64-bit LSB (shared object|pie executable|relocatable)", "elf", 352),
    ("jpeg", r"JPEG image data", "jpeg", 17),
    ("gif", r"GIF image data", "images", 568),
    ("wav", r"RIFF \(little-endian\) data, WAVE", "riff", 244),
    ("zip", r"Zip archive data", "zip", 129),
    ("pdf", r"PDF document", "pdf", 12),
    ("tar", r"POSIX tar archive", "archive", 98),
    ("jmod", r"Java jmod module", "java", 32),
    ("ole2", r"Composite Document File V2", "ole2compounddocs", 9),
    ("svg", r"SVG Scalable Vector Graphics", "sgml", 17),
    ("xml", r"XML 1\.0 document", "sgml", 21),
    ("ps", r"PostScript document text", "printer", 8),
]


def unesc(s):
    """libmagic string escapes to bytes."""
    out, i = bytearray(), 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(ord(c)); i += 1; continue
        i += 1
        if i >= len(s):
            break
        c = s[i]
        if c == "x":
            h = re.match(r"[0-9a-fA-F]{1,2}", s[i + 1:]).group(0)
            out.append(int(h, 16)); i += 1 + len(h)
        elif c in "01234567":
            o = re.match(r"[0-7]{1,3}", s[i:]).group(0)
            out.append(int(o, 8) & 255); i += len(o)
        else:
            out.append({"n": 10, "r": 13, "t": 9, "a": 7, "b": 8, "f": 12, "v": 11}.get(c, ord(c))); i += 1
    return bytes(out)


def test_bytes(line):
    """(offset, bytes) of a magic line's test, or None if it is not a fixed test at a fixed offset."""
    f = re.split(r"\s+", line.strip(), maxsplit=3)
    if len(f) < 3:
        return None
    off, typ, val = f[0].lstrip(">"), f[1], f[2]
    if not re.fullmatch(r"\d+|0x[0-9a-fA-F]+", off):
        return None
    off = int(off, 0)
    typ, _, mask = typ.partition("&")
    typ = typ.split("/")[0]
    if typ == "string":
        v = unesc(val.lstrip("=")) if not val.startswith((">", "<", "!")) else None
        return (off, v) if v else None
    sizes = {"byte": 1, "short": 2, "long": 4, "quad": 8}
    m = re.fullmatch(r"u?(be|le)?(byte|short|long|quad)", typ)
    if not m:
        return None
    n = sizes[m.group(2)]; end = "big" if m.group(1) == "be" else "little"
    val = val.lstrip("=")
    if not re.fullmatch(r"0x[0-9a-fA-F]+|\d+", val):
        return None
    b = int(val, 0).to_bytes(n, end)
    if mask:
        mb = int(mask, 0).to_bytes(n, end)
        keep = [i for i in range(n) if mb[i]]
        if not keep:
            return None
        b = b[:keep[-1] + 1]
    return off, b


def entry(path, ln):
    """The magic entry around line ln: up to 4 comment lines above its top-level line, that line, and up to 10
    continuation lines (at most 600 bytes); plus the test to translate (the anchor line's, else the top line's) and
    the message."""
    lines = open(path, encoding="latin-1").read().split("\n")
    a = ln - 1
    top = a
    while top > 0 and (lines[top].startswith(">") or lines[top].startswith("!:")):
        top -= 1
    com = []
    j = top - 1
    while j >= 0 and lines[j].startswith("#") and len(com) < 4:
        if not lines[j].startswith(("# $File", "#---", "#===")) and lines[j].strip("# "):
            com.insert(0, lines[j])
        j -= 1
    body = [lines[top]]
    j = top + 1
    while j < len(lines) and (lines[j].startswith(">") or lines[j].startswith("!:")) and len(body) < 11:
        body.append(lines[j]); j += 1
    if a > top and lines[a] not in body:
        body.append(lines[a])
    text = "\n".join(com + body)
    while len(text) > 600:
        if len(body) > 1:
            body.pop()
        else:
            com.pop(0)
        text = "\n".join(com + body)
    t = test_bytes(lines[a]) or test_bytes(lines[top])
    msg = re.split(r"\t+|\s{2,}", lines[a].strip(), maxsplit=3)
    msg = msg[3] if len(msg) > 3 else ""
    return text + "\n", t, msg


types = {}
for l in open(types_tsv, encoding="utf-8", errors="replace"):
    p, _, t = l.rstrip("\n").partition("\t")
    types.setdefault(t.strip(), []).append(p)
rng = random.Random(66)
fmts = []
for name, pat, mf, ln in FORMATS:
    text, t, msg = entry(os.path.join(magdir, mf), ln)
    cands = sorted(p for ty, ps in types.items() if re.match(pat, ty) for p in ps)
    rng.shuffle(cands)
    seen, picked = set(), []
    for p in cands:
        try:
            raw = open(p, "rb").read(L)
        except OSError:
            continue
        h = hashlib.sha1(raw).hexdigest()
        if len(raw) < L or h in seen:
            continue
        seen.add(h); picked.append((p, raw))
        if len(picked) == K:
            break
    if len(picked) < 4:
        print(f"skip {name}: {len(picked)} files", file=sys.stderr); continue
    note = f"format {name}. " + (msg.lower().strip(" ,") + ". " if msg else "")
    if t:
        note += ("magic " if t[0] == 0 else f"magic at offset {t[0]}: ") + " ".join(f"{b:02x}" for b in t[1]) + "."
    note += "\n"
    match = sum(raw[t[0]:t[0] + len(t[1])] == t[1] for _, raw in picked) if t else 0
    fmts.append({"name": name, "magic": text, "note": note, "test": t, "files": picked, "match": match})
    print(f"{name:9} {len(picked)} files, test {t[0] if t else '-'}:{t[1].hex() if t else '-'} matches {match}, "
          f"magic {len(text)} bytes", file=sys.stderr)

# progressive order: a new format with probability 0.35 while any is left, else a further file of a format seen
order, nxt, left = [], 0, {}
while nxt < len(fmts) or any(left.values()):
    live = [f for f, n in left.items() if n]
    if nxt < len(fmts) and (not live or rng.random() < 0.35):
        order.append((nxt, 0)); left[nxt] = len(fmts[nxt]["files"]) - 1; nxt += 1
    else:
        f = rng.choice(live); k = len(fmts[f]["files"]) - left[f]; left[f] -= 1; order.append((f, k))
wrong = {(f, k): rng.choice([g for g in range(len(fmts)) if g != f]) for f, k in order}
CONDS = ["Rm", "Rn", "W", "N"]
fmt2 = open("runs/exp60/fmt2.bin", "rb").read()
f2s = [int(x) for x in open("runs/exp60/fmt2.starts")]
f2d = [int(x) for x in open("runs/exp60/fmt2.descs")]
for s in range(4):
    data, starts, descs, items = bytearray(), [], [], []
    for f, k in order:
        c = CONDS[(f + k + s) % 4]
        d = {"Rm": fmts[f]["magic"], "Rn": fmts[f]["note"], "W": fmts[wrong[(f, k)]]["magic"], "N": ""}[c].encode("latin-1")
        it = {"format": f, "name": fmts[f]["name"], "k": k, "cond": c, "file": fmts[f]["files"][k][0],
              "desc_start": len(data), "desc_len": len(d)}
        if d:
            starts.append(len(data)); descs.append(len(data)); data += d
        it["start"] = len(data); it["len"] = L
        t = fmts[f]["test"]
        it["magic"] = [t[0], len(t[1])] if t and fmts[f]["files"][k][1][t[0]:t[0] + len(t[1])] == t[1] else None
        starts.append(len(data)); data += fmts[f]["files"][k][1]; items.append(it)
    for pre in ("", "pre"):
        off = len(fmt2) if pre else 0
        nm = f"{outdir}/{pre}real{s}"
        open(nm + ".bin", "wb").write((fmt2 if pre else b"") + bytes(data))
        open(nm + ".starts", "w").write("".join(f"{x}\n" for x in (f2s if pre else []) + [x + off for x in starts]))
        open(nm + ".descs", "w").write("".join(f"{x}\n" for x in (f2d if pre else []) + [x + off for x in descs]))
        json.dump({"offset": off, "L": L, "formats": [x["name"] for x in fmts],
                   "items": [{**it, "desc_start": it["desc_start"] + off, "start": it["start"] + off} for it in items]},
                  open(nm + ".items.json", "w"), indent=0)
    print(f"rotation {s}: {len(data)} bytes, {len(items)} files", file=sys.stderr)
json.dump([{k: v for k, v in x.items() if k != "files"} | {"files": [p for p, _ in x["files"]], "test": [x["test"][0], x["test"][1].hex()] if x["test"] else None} for x in fmts],
          open(f"{outdir}/formats.json", "w"), indent=1)
