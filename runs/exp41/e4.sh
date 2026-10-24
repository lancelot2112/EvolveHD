#!/bin/bash
# E4 (forum round six, step 4): a learned hop-2 fan-out (--sit-chain-learn) against the full walk, with "the" never a
# boundary (SIT_BND_DENY=2, the diagnostic) so the chains have facts to follow; small qa2 then the full qa2 stream
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41g
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096 --sit-path-forget 2"
S="--sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-chain --sit-lt-budget 131072 --sit-sketch-decay"
run() { d=$1; T=$2; tag=$3; shift 3; SIT_BND_DENY=2 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_$d.ids --vocab runs/exp40/bl_$d.txt $S "$@" --sit-freeze-from $T --out runs/exp41/R_${d}_$tag 2> runs/exp41/R_${d}_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_$d runs/exp41/R_${d}_$tag > runs/exp41/R_${d}_$tag.score; }
run qa2s 102303 dn & run qa2s 102303 dncl --sit-chain-learn; wait
run qa2 220618 dn & run qa2 220618 dncl --sit-chain-learn; wait
for r in qa2s_dn qa2s_dncl qa2_dn qa2_dncl; do echo "== $r"; cat runs/exp41/R_$r.score; grep -E "words,|cost per" runs/exp41/R_$r.log | cut -c1-130; grep -E "^TIMING|^HEADS" runs/exp41/R_$r.log | tail -2 | cut -c1-230; done
