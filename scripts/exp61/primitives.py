"""Pretraining streams of primitive encodings, as files (each file one primitive, one parameter choice), laid end to
end with their start offsets written for the position sense. Kinds:
  int    arrays of integers: width 1, 2, 4 or 8 bytes, little or big endian, signed or unsigned; sequences that are
         counters (random start and step), random walks, small values, or sorted values
  float  arrays of IEEE floats (4 or 8 bytes, little endian) sampled from smooth signals (sums of sines), ramps or
         noise
  varint integers in LEB128 (as in DWARF and protobuf), unsigned or zigzag signed
  utf8   text in made-up languages written in other scripts (Greek, Cyrillic, Armenian, Hebrew, Devanagari, CJK,
         Hangul), so the multi-byte forms of UTF-8 appear with word structure
  ascii  English prose (slices of the Gutenberg books)
  strtab string tables: zero-terminated names, and length-prefixed names (1- or 2-byte lengths)
  image  raw pixel rows (RGB or grey, widths 16-256), from ImageMagick's built-in images and gradients
  rosetta  the same numbers side by side: "0x04d2 " then its bytes as 2- and 4-byte little endian, then "\n"
Writes OUT.bin, OUT.starts (one start offset per line) and OUT.files.json (kind, parameters, start, length).
usage: primitives.py OUT [FILE_BYTES] [N_PER_KIND] [SEED] [KINDS]"""
import sys, json, random, struct, math, subprocess, os, tempfile
out = sys.argv[1]; FB = int(sys.argv[2]) if len(sys.argv) > 2 else 8000; NK = int(sys.argv[3]) if len(sys.argv) > 3 else 12
rng = random.Random(int(sys.argv[4]) if len(sys.argv) > 4 else 0)
kinds = sys.argv[5].split(",") if len(sys.argv) > 5 else ["int", "float", "varint", "utf8", "ascii", "strtab", "image", "rosetta"]
WORDS = open("/usr/share/dict/words").read().split() if os.path.exists("/usr/share/dict/words") else None

def f_int():
    w = rng.choice([1, 2, 4, 8]); order = rng.choice(["little", "big"]); signed = rng.random() < 0.3
    lo, hi = (-(1 << (8 * w - 1)), (1 << (8 * w - 1)) - 1) if signed else (0, (1 << (8 * w)) - 1)
    kind = rng.choice(["counter", "walk", "small", "sorted"])
    n = FB // w; v = []
    if kind == "counter":
        x = rng.randint(0, min(hi, 1 << 20)); st = rng.choice([1, 1, 2, 4, 8, 16, rng.randint(1, 1000)])
        v = [x + i * st for i in range(n)]
    elif kind == "walk":
        x = rng.randint(0, min(hi, 1 << 16)); sc = rng.choice([1, 4, 64, 1000])
        for _ in range(n):
            x += rng.randint(-sc, sc); v.append(x)
    elif kind == "small":
        top = rng.choice([10, 100, 1000, 60000]); v = [rng.randint(0, top) for _ in range(n)]
    else:
        v = sorted(rng.randint(0, min(hi, 1 << 24)) for _ in range(n))
    b = bytearray()
    for x in v:
        x = max(lo, min(hi, x)); b += x.to_bytes(w, order, signed=signed)
    return bytes(b), {"width": w, "order": order, "signed": signed, "seq": kind}

def f_float():
    w = rng.choice([4, 8]); kind = rng.choice(["sines", "ramp", "noise"]); n = FB // w
    fr = [rng.uniform(0.001, 0.1) for _ in range(3)]; am = [rng.uniform(0.1, 100) for _ in range(3)]; off = rng.uniform(-50, 50)
    v = []
    for i in range(n):
        if kind == "sines": x = off + sum(a * math.sin(f * i + a) for f, a in zip(fr, am))
        elif kind == "ramp": x = off + i * fr[0] * am[0]
        else: x = rng.gauss(off, am[0])
        v.append(x)
    return struct.pack("<" + ("f" if w == 4 else "d") * n, *v), {"width": w, "seq": kind}

def leb(x):
    b = bytearray()
    while True:
        c = x & 0x7F; x >>= 7
        if x: b.append(c | 0x80)
        else: b.append(c); return bytes(b)
def f_varint():
    zz = rng.random() < 0.5; top = rng.choice([100, 10000, 1 << 20, 1 << 32]); b = bytearray()
    while len(b) < FB:
        x = rng.randint(-top, top) if zz else int(rng.paretovariate(1.2)) % top
        b += leb((x << 1) ^ (x >> 63) if zz else x)
    return bytes(b[:FB]), {"zigzag": zz, "top": top}

SCRIPTS = {"greek": (0x3B1, 25), "cyrillic": (0x430, 32), "armenian": (0x561, 38), "hebrew": (0x5D0, 27),
           "devanagari": (0x915, 37), "cjk": (0x4E00, 3000), "hangul": (0xAC00, 2000)}
def f_utf8():
    sc = rng.choice(sorted(SCRIPTS)); base, n = SCRIPTS[sc]
    lex = ["".join(chr(base + rng.randrange(n)) for _ in range(rng.randint(1, 3 if n > 100 else 7))) for _ in range(300)]
    s = ""
    while len(s.encode()) < FB:
        sent = " ".join(rng.choice(lex) for _ in range(rng.randint(3, 12)))
        s += sent + rng.choice([". ", ".\n", ", "])
    return s.encode()[:FB], {"script": sc}

BOOKS = [os.path.join("corpus/raw/gutenberg", f) for f in sorted(os.listdir("corpus/raw/gutenberg")) if f.endswith(".txt")]
def f_ascii():
    p = rng.choice(BOOKS); raw = open(p, "rb").read(); a = rng.randrange(2000, len(raw) - FB)
    return raw[a:a + FB], {"book": os.path.basename(p)}

NAMES = "main init exit read write open close malloc free memcpy strlen printf fopen fclose socket bind listen accept connect send recv poll select mmap munmap getpid fork exec wait signal kill time clock sleep abort qsort bsearch strcmp strcpy strcat atoi strtol".split()
def f_strtab():
    kind = rng.choice(["zero", "len1", "len2"]); b = bytearray()
    while len(b) < FB:
        nm = rng.choice(NAMES) + rng.choice(["", "", "_r", "64", "_chk", "@GLIBC_2.2.5", "_" + rng.choice(NAMES)])
        e = nm.encode()
        b += e + b"\0" if kind == "zero" else bytes([len(e)]) + e if kind == "len1" else len(e).to_bytes(2, "little") + e
    return bytes(b[:FB]), {"form": kind}

IMGS = ["rose:", "logo:", "wizard:", "granite:", "netscape:", "gradient:red-blue", "gradient:white-black", "plasma:fractal", "radial-gradient:yellow-navy"]
def f_image():
    src = rng.choice(IMGS); w = rng.choice([16, 32, 48, 64, 100, 128, 200, 256]); grey = rng.random() < 0.3
    ch = 1 if grey else 3; h = max(1, FB // (w * ch))
    with tempfile.TemporaryDirectory() as d:
        fn = os.path.join(d, "x.raw")
        args = ["convert"] + (["-size", f"{w}x{h}"] if src.split(":")[0] in ("gradient", "plasma", "radial-gradient") else []) + [src, "-resize", f"{w}x{h}!", "-depth", "8", ("gray:" if grey else "rgb:") + fn]
        subprocess.run(args, check=True, capture_output=True)
        b = open(fn, "rb").read()
    return b[:FB], {"src": src, "width": w, "channels": ch}

def f_rosetta():
    b = bytearray(); top = rng.choice([256, 65536])
    while len(b) < FB:
        x = rng.randrange(top)
        b += f"0x{x:04x} ".encode() + x.to_bytes(2, "little") + x.to_bytes(4, "little") + b"\n"
    return bytes(b[:FB]), {"top": top}

GEN = {"int": f_int, "float": f_float, "varint": f_varint, "utf8": f_utf8, "ascii": f_ascii, "strtab": f_strtab, "image": f_image, "rosetta": f_rosetta}
plan = [k for k in kinds for _ in range(NK)]; rng.shuffle(plan)
data, meta = bytearray(), []
for k in plan:
    b, prm = GEN[k]()
    meta.append({"kind": k, "params": prm, "start": len(data), "len": len(b)}); data += b
open(out + ".bin", "wb").write(bytes(data))
open(out + ".starts", "w").write("\n".join(str(m["start"]) for m in meta) + "\n")
json.dump({"files": meta, "bytes": len(data)}, open(out + ".files.json", "w"), indent=0)
print(f"{out}: {len(data)} bytes, {len(meta)} files: " + ", ".join(f"{k} {sum(m['kind'] == k for m in meta)}" for k in kinds), file=sys.stderr)
