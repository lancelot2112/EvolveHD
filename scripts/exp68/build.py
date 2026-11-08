"""Experiment 68: experiment 60's made-up practice stream (fmt2) with the magic spelled three other ways, then
experiment 66's real files and manual entries, to separate two reasons experiment 67's spelling view didn't read the
real entries: where in the word a byte is spelled, and which word comes before the magic.
  A: "magic \\xf1\\302\\x6b."   escapes inside one token, after "magic"   (place in the word changes)
  B: "0 string f1 c2 6b."       spaced hex after "0 string"             (word before changes)
  C: "0 string \\xf1\\302\\x6b."  escapes inside one token, after "string" (both, as libmagic writes it)
Escapes are hex (\\xNN) for even-numbered formats and octal (\\NNN) for odd ones, as libmagic uses both.
Only the descriptions change: every file byte is the same, and the real part is experiment 66's prereal stream after
the practice. Writes runs/exp68/{A,B,C}prereal{0-3}.{bin,starts,descs,items.json}.
usage: build.py"""
import json, re
O = 593485
f2 = open("runs/exp60/fmt2.bin", "rb").read()
assert len(f2) == O
meta = json.load(open("runs/exp60/fmt2.items.json"))["items"]
pat = re.compile(rb"format (\d+)\. magic ((?:[0-9a-f]{2} )*[0-9a-f]{2})\.")


def esc(f, hx):
    bs = bytes.fromhex(hx.decode().replace(" ", ""))
    return "".join(f"\\x{b:02x}" if f % 2 == 0 else f"\\{b:03o}" for b in bs).encode()


def spell(style, d):
    m = pat.search(d)
    f, hx = int(m.group(1)), m.group(2)
    new = {"A": b"magic " + esc(f, hx), "B": b"0 string " + hx, "C": b"0 string " + esc(f, hx)}[style]
    return d[:m.start(2) - len(b"magic ")] + new + d[m.end(2):]


for style in "ABC":
    data, starts, descs, items = bytearray(), [], [], []
    for it in meta:
        d = f2[it["desc_start"]:it["desc_start"] + it["desc_len"]]
        it2 = dict(it, desc_start=len(data))
        if d:
            d = spell(style, d)
            starts.append(len(data)); descs.append(len(data)); data += d
        it2["desc_len"] = len(d); it2["start"] = len(data)
        starts.append(len(data)); data += f2[it["start"]:it["start"] + it["len"]]
        items.append(it2)
    # the starts and descs of fmt2 must be exactly these segment starts, shifted
    f2s = [int(x) for x in open("runs/exp60/fmt2.starts")]
    f2d = [int(x) for x in open("runs/exp60/fmt2.descs")]
    assert len(f2s) == len(starts) and len(f2d) == len(descs), (len(f2s), len(starts), len(f2d), len(descs))
    off = len(data)
    for s in range(4):
        src = f"runs/exp66/prereal{s}"
        b = open(src + ".bin", "rb").read()
        rs = [int(x) - O for x in open(src + ".starts") if int(x) >= O]
        rd = [int(x) - O for x in open(src + ".descs") if int(x) >= O]
        im = json.load(open(src + ".items.json"))
        nm = f"runs/exp68/{style}prereal{s}"
        open(nm + ".bin", "wb").write(bytes(data) + b[O:])
        open(nm + ".starts", "w").write("".join(f"{x}\n" for x in starts + [x + off for x in rs]))
        open(nm + ".descs", "w").write("".join(f"{x}\n" for x in descs + [x + off for x in rd]))
        im["offset"] = off
        im["items"] = [{**x, "desc_start": x["desc_start"] - O + off, "start": x["start"] - O + off} for x in im["items"]]
        json.dump(im, open(nm + ".items.json", "w"), indent=0)
    json.dump({"items": items}, open(f"runs/exp68/fmt2{style}.items.json", "w"))
    print(style, off, "bytes of practice; first description:", data[:items[0]["desc_len"]].decode().strip())
