#!/bin/bash
# Exp 57: the byte stream. The word reader (default stack with instant kinds) on byte ids, and the level-0 byte
# brain of experiment 20 (stack, one level), both reading the same 384,000 bytes from nothing.
cd /home/claude/evolvehd
B=${B:-runs/exp36/sit-56a}; SEED=${SEED:-0}; T=${T:-a}
F=$(sed "s/--sit-seed 0/--sit-seed $SEED/" runs/exp37/F.flags)
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
G="--sit-bnd-unit --sit-bnd-unit-len --sit-act --sit-act-hyst --sit-rel-merge --sit-kind-instant --sit-kind-instant-move $EXTRA"
reader() { SIT_DEBUG_BND=1 $B assoc --sit --data runs/exp57/bytes.ids --vocab runs/exp57/bytes.txt --sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F $X $G --out runs/exp57/R_$T 2> runs/exp57/R_$T.log; }
brain() { EVOLVEHD_PROGRESS=50000 $B stack --data runs/exp57/bytes.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions $BEXTRA --out-dir runs/exp57/brain_$T > runs/exp57/brain_$T.out 2>&1; }
for f in "$@"; do $f & done; wait
