"""Made-up binary formats, each with a short description in plain controlled wording, and a byte stream of files
read three ways: after their own format's description (R), after a different format's description (W), or after
nothing (N). A description and its file are one "item"; items are laid end to end.

A format: a magic (2-4 bytes), a version byte, a byte order, a count width (1 or 2 bytes), and a record layout of
2-4 fields from: a kind byte (0-7), a value (2 or 4 bytes, small numbers), a name (fixed width, padded with spaces
or zero bytes), a constant byte. A file: magic, version, count n (3-12), then n records.

Description example: "format 12. magic 4b 51 9e. version 3. little endian. count 2 bytes. record: kind 1 byte,
value 4 bytes, name 8 bytes padded with 00, byte 7f. end.\n"

Writes OUT.bin and OUT.items.json: per item, the format, the condition, the description's start and length (0 for
N), the file's start and length, and the file's field map (offset, length, field type) for scoring.
usage: formats.py OUT [N_FORMATS] [FILES_PER_FORMAT] [SEED] [SPREAD]
(SPREAD > 0: formats arrive through the stream; a format's later files come within SPREAD formats of its first)"""
import sys, json, random
out = sys.argv[1]; NF = int(sys.argv[2]) if len(sys.argv) > 2 else 300; FPF = int(sys.argv[3]) if len(sys.argv) > 3 else 8
rng = random.Random(int(sys.argv[4]) if len(sys.argv) > 4 else 0)
WORDS = "alpha bravo cedar delta ember frost grove haven iris jade kilo lumen mesa nova onyx pine quill raven sage tide umber vale willow yarrow zinc".split()
def hx(b): return f"{b:02x}"
formats = []
for f in range(NF):
    fields = []
    for _ in range(rng.randint(2, 4)):
        t = rng.choice(["kind", "value", "name", "const"])
        if t == "value": fields.append(("value", rng.choice([2, 4]), None))
        elif t == "name": fields.append(("name", rng.randint(4, 12), rng.choice([0x20, 0x00])))
        elif t == "const": fields.append(("const", 1, rng.randrange(256)))
        else: fields.append(("kind", 1, None))
    fm = {"id": f, "magic": [rng.randrange(256) for _ in range(rng.randint(2, 4))], "version": rng.randint(0, 9),
          "order": rng.choice(["little", "big"]), "cw": rng.choice([1, 2]), "fields": fields}
    parts = [f"format {f}.", "magic " + " ".join(hx(b) for b in fm["magic"]) + ".", f"version {fm['version']}.",
             f"{fm['order']} endian.", f"count {fm['cw']} byte" + ("s" if fm["cw"] > 1 else "") + "."]
    fd = []
    for t, w, x in fields:
        if t == "kind": fd.append("kind 1 byte")
        elif t == "value": fd.append(f"value {w} bytes")
        elif t == "name": fd.append(f"name {w} bytes padded with {hx(x)}")
        else: fd.append(f"byte {hx(x)}")
    parts.append("record: " + ", ".join(fd) + ".")
    fm["desc"] = (" ".join(parts) + " end.\n").encode()
    formats.append(fm)
def make_file(fm):
    b = bytearray(fm["magic"]); fmap = [(0, len(fm["magic"]), "magic"), (len(b), 1, "version")]
    b.append(fm["version"]); n = rng.randint(3, 12)
    fmap.append((len(b), fm["cw"], "count")); b += n.to_bytes(fm["cw"], fm["order"])
    for _ in range(n):
        for t, w, x in fm["fields"]:
            o = len(b)
            if t == "kind": b.append(rng.randrange(8))
            elif t == "value": b += rng.randrange(1000).to_bytes(w, fm["order"])
            elif t == "name":
                s = rng.choice(WORDS).encode()[:w]; b += s + bytes([x]) * (w - len(s))
            else: b.append(x)
            fmap.append((o, w, t))
    return bytes(b), fmap
# formats arrive one by one through the stream: format f is introduced at time f, and its files fall at times
# f + a random delay (the first at f), so files of formats never seen before keep coming to the end
SPREAD = int(sys.argv[5]) if len(sys.argv) > 5 else 0
if SPREAD > 0:
    ev = [(f + (0 if i == 0 else rng.uniform(0, SPREAD)), f, i) for f in range(NF) for i in range(FPF)]
    items = [(f, i) for _, f, i in sorted(ev)]
else:
    items = [(f, i) for f in range(NF) for i in range(FPF)]
    rng.shuffle(items)
data, meta = bytearray(), []
for f, i in items:
    fm = formats[f]; cond = rng.choice("RWN")
    d = fm["desc"] if cond == "R" else formats[rng.choice([g for g in range(NF) if g != f])]["desc"] if cond == "W" else b""
    ds = len(data); data += d
    fb, fmap = make_file(fm)
    meta.append({"format": f, "cond": cond, "desc_start": ds, "desc_len": len(d), "start": len(data), "len": len(fb), "fields": fmap})
    data += fb
open(out + ".bin", "wb").write(bytes(data))
json.dump({"items": meta, "formats": [{k: v for k, v in fm.items() if k != "desc"} | {"desc": fm["desc"].decode()} for fm in formats]}, open(out + ".items.json", "w"))
print(f"{out}: {len(data)} bytes, {len(meta)} items ({sum(m['cond'] == c for m in meta for c in 'R')} R), "
      f"file bytes {sum(m['len'] for m in meta)}", file=sys.stderr)
print(formats[0]["desc"].decode(), file=sys.stderr)
