"""Write the job list for the standard set: old vs new stack, seeds 0-2 (one line per job: name<TAB>command)."""
F = open("runs/exp37/F.flags").read().split()
X = "--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096".split()
NEW = "--sit-levels --sit-area-kinds --sit-area-anti 2 --sit-area-sticky".split()
B = "runs/exp36/sit-40a"
D = "--sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool".split()
def f(seed): return " ".join(x if x != "0" or F[i - 1] != "--sit-seed" else str(seed) for i, x in enumerate(F))
jobs = []
for s in [0, 1, 2]:
    for st, extra in [("o", []), ("n", NEW)]:
        e = " ".join(extra); fs = f(s)
        # skip seed-0 runs that already exist (same binary behaviour): vv_base, vv_aks, c2_base, bl_long, bl_long_ak2s (no levels: none of its questions nest)
        books = [("pq", "runs/exp37/ld2m_pq", ""), ("pr", "runs/exp38/ld2m_pr", "--sit-answer-only 1009264 --sit-freeze-from 1009264"),
                 ("ps", "runs/exp38/ld2m_ps", "--sit-answer-only 1009264 --sit-freeze-from 1009264"),
                 ("vv", "runs/exp39/ld2m_vv", "--sit-answer-only 1004771 --sit-freeze-from 1004771"), ("c2", "runs/exp37/ld200k", "")]
        for tag, doc, a in books:
            name = f"S{s}{st}_{tag}"
            jobs.append((name, f"B={B} bash runs/exp37/rld.sh {doc} {name} {fs} --sit-nest --sit-scope --sit-epoch {e} {a}"))
        name = f"S{s}{st}_story"
        jobs.append((name, f"B={B} bash runs/exp36/rq30.sh \"{name} {fs} {' '.join(X)} {e}\""))
        name = f"S{s}{st}_bl"
        jobs.append((name, f"SEED={s} BIN={B} TAG=_{name} EXTRA=\"{e}\" bash runs/exp38/bl_split.sh bl_long"))
# biggest first
order = {"pq": 0, "vv": 1, "pr": 2, "ps": 3, "bl": 4, "c2": 5, "story": 6}
jobs.sort(key=lambda j: (order[j[0].split("_")[1]], j[0]))
open("runs/exp40/jobs.txt", "w").write("".join(f"{n}\t{c}\n" for n, c in jobs))
print(len(jobs), "jobs")
