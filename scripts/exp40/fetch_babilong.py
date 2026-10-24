"""Fetch BABILong samples from the Hugging Face datasets server into runs/exp38/babilong/ (the format
babilong_stream.py reads: a list of {input, question, target}).
usage: fetch_babilong.py TASK:LENGTH:N[:train] ...   (train: from RMT-team/babilong-train-5k-samples)"""
import sys, json, urllib.request, os, time
for spec in sys.argv[1:]:
    p = spec.split(":"); task, length, n = p[0], p[1], int(p[2]); train = len(p) > 3
    ds = "RMT-team%2Fbabilong-train-5k-samples" if train else "RMT-team%2Fbabilong"
    out = f"runs/exp38/babilong/{'train_' if train else ''}{task}_{length}.json"
    if os.path.exists(out): print("have", out); continue
    rows = []
    while len(rows) < n:
        url = f"https://datasets-server.huggingface.co/rows?dataset={ds}&config={length}&split={task}&offset={len(rows)}&length={min(100, n - len(rows))}"
        for attempt in range(6):
            try:
                got = json.load(urllib.request.urlopen(url, timeout=120))["rows"]; break
            except Exception as e:
                print("retry", attempt, e, file=sys.stderr); time.sleep(5 * (attempt + 1))
        else:
            raise SystemExit("gave up on " + url)
        if not got: break
        rows += [r["row"] for r in got]
    json.dump([{"input": r["input"], "question": r["question"], "target": r["target"]} for r in rows], open(out, "w"))
    print(out, len(rows))
