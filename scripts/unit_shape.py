"""How word-like are the units a stack level hands up? Reads levelN.units.txt (count<TAB>text, \\n escaped).
Reports, weighted by occurrences: mean length in bytes, share ending at a word end (last byte not a letter),
share shaped like one whole word (letters, then at least one non-letter), and share shaped like several words."""
import re, sys
for path in sys.argv[1:]:
    tot = end = one = multi = ln = 0
    for line in open(path, encoding='utf-8', errors='replace'):
        c, _, t = line.rstrip('\n').partition('\t')
        t = t.replace('\\n', '\n'); c = int(c)
        tot += c; ln += c * len(t)
        if t and not t[-1].isalpha():
            end += c
        if re.fullmatch(r"[A-Za-z']+[^A-Za-z']+", t):
            one += c
        elif re.fullmatch(r"([A-Za-z']+[^A-Za-z']+)+", t) and len(re.findall(r"[A-Za-z']+", t)) > 1:
            multi += c
    print(f"{path}: units {tot}, mean length {ln / tot:.1f} bytes, end at a word end {end / tot:.2f}, "
          f"one whole word {one / tot:.2f}, several whole words {multi / tot:.2f}")
