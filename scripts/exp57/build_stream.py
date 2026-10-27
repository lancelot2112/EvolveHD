"""A byte stream of real files, two of each type, each cut to its first N bytes (default 24,000) and laid end to
end with nothing between them: round A (one file of each type), then round B (the second file of each type, same
order). Writes OUT.bin (raw bytes), OUT.ids (u32 per byte, for the word reader), OUT.txt (a name per byte value),
OUT.files.json (name, type, round, start, length).
usage: build_stream.py OUT [N]"""
import sys, json, os
import numpy as np
out = sys.argv[1]; N = int(sys.argv[2]) if len(sys.argv) > 2 else 24000
H = os.path.expanduser("~")
C = "/home/claude/.cache/uv/archive-v0"
files = [
    # (type, round A file, round B file, skip bytes at the start)
    ("text", "corpus/raw/gutenberg/austen-persuasion.txt", "corpus/raw/gutenberg/chesterton-thursday.txt", 2000),
    ("json", f"{C}/mtoHCAj7J6yIIPGp/playwright/driver/package/lib/server/deviceDescriptorsSource.json", f"{C}/mtoHCAj7J6yIIPGp/playwright/driver/package/api.json", 0),
    ("csv", f"{C}/LAZSNjXI6jZquv-B/sklearn/datasets/data/breast_cancer.csv", f"{C}/Fl4NnZ62Fbv-oW7x/matplotlib/mpl-data/sample_data/Stocks.csv", 0),
    ("rust", "src/brain.rs", "src/situation.rs", 0),
    ("xml", "/etc/ImageMagick-6/mime.xml", f"{C}/vSzU4QKPTk8j3sMa/skimage/data/lbpcascade_frontalface_opencv.xml", 0),
    ("elf", "/usr/bin/xz", "/usr/bin/gzip", 0),
    ("bmp", "runs/exp57/logo.bmp", "runs/exp57/wizard.bmp", 0),
    ("gzip", "/usr/share/doc/gcc-13-base/changelog.gz", "/usr/share/doc/libllvm18/changelog.Debian.gz", 0),
]
data, meta = bytearray(), []
for rnd in (0, 1):
    for typ, a, b, skip in files:
        p = (a, b)[rnd]
        raw = open(p, "rb").read()[skip:skip + N]
        assert len(raw) == N, (p, len(raw))
        meta.append({"name": p, "type": typ, "round": "AB"[rnd], "start": len(data), "len": N})
        data += raw
open(out + ".bin", "wb").write(bytes(data))
np.frombuffer(bytes(data), dtype=np.uint8).astype(np.uint32).tofile(out + ".ids")
def nm(b):
    c = chr(b)
    if c == " ": return "SP"
    if c == "\n": return "NL"
    if c == "\t": return "TAB"
    if 33 <= b < 127: return c
    return f"x{b:02x}"
open(out + ".txt", "w").write("\n".join(nm(b) for b in range(256)) + "\n")
json.dump({"files": meta, "bytes": len(data)}, open(out + ".files.json", "w"), indent=1)
print(f"{out}: {len(data)} bytes, {len(meta)} files", file=sys.stderr)
