"""Score generated samples: share of generated words that occur in the training text.

usage: python3 scripts/gen_report.py corpus_file samples.gen.txt [more.gen.txt ...]
A sample file holds blocks: '=== prompt: ...', '--- grown', text, '--- witten-bell order 4', text.
Only the generated continuation is scored (the prompt is removed).
"""
import re
import sys


def words(text):
    return re.findall(r"[A-Za-z]+", text)


def main():
    vocab = {w.lower() for w in words(open(sys.argv[1], 'rb').read().decode('latin-1'))}
    for path in sys.argv[2:]:
        blocks = open(path).read().split('=== prompt: ')[1:]
        tot = {}
        for b in blocks:
            prompt = eval(b.split('\n', 1)[0])
            parts = re.split(r'^--- (grown|witten-bell order 4)\n', b, flags=re.M)
            for name, body in zip(parts[1::2], parts[2::2]):
                gen = body.rstrip('\n')[len(prompt):]
                ws = words(gen)
                ok = sum(w.lower() in vocab for w in ws)
                n, k = tot.get(name, (0, 0))
                tot[name] = (n + len(ws), k + ok)
        for name, (n, k) in tot.items():
            print(f'{path}: {name:<22} {k}/{n} words in the training vocabulary ({100 * k / max(n, 1):.1f}%)')


if __name__ == '__main__':
    main()
