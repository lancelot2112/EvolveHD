#!/bin/bash
# E3: the contrast question detector (--sit-q-contrast), and the learned fan-out credited only at answers
# (--sit-chain-learn --sit-chain-learn-q). qa2 runs ban "the" by hand (the diagnostic) so chains have facts.
cd /home/claude/evolvehd
F=$(cat runs/exp37/F.flags); B=runs/exp36/sit-41l
X="--sit-skip --sit-lsh --sit-slotlog 16 --sit-recent 32 --sit-heads-nokind --sit-bnd-hyst --sit-lt-min 4 --sit-lt-credit 64 --sit-wm-horizon 4096"
S="--sit-kinds --sit-mood --sit-query --sit-pool 0.7 --sit-fpool $F --sit-nest --sit-scope $X --sit-path-forget 2 --sit-chain --sit-lt-budget 131072 --sit-sketch-decay"
run() { d=$1; T=$2; tag=$3; shift 3; SIT_BND_DENY=2 SIT_TIMING=1 $B assoc --sit --data runs/exp40/bl_$d.ids --vocab runs/exp40/bl_$d.txt $S "$@" --sit-freeze-from $T --out runs/exp41/R_${d}_$tag 2> runs/exp41/R_${d}_$tag.log; python3 scripts/exp38/score_babilong.py runs/exp40/bl_$d runs/exp41/R_${d}_$tag > runs/exp41/R_${d}_$tag.score; }
CL="--sit-chain-learn --sit-chain-learn-q"
run qa2s 102303 qc --sit-q-contrast & run qa2s 102303 qccl --sit-q-contrast $CL; wait
B=$B bash runs/exp36/rq30.sh "qc_story $F $X --sit-q-contrast" &
BIN=$B TAG=_qc EXTRA="--sit-q-contrast" bash runs/exp38/bl_split.sh bl_pilot > /dev/null; wait
run qa2 220618 efqc --sit-entity-fair --sit-q-contrast & run qa2 220618 efqccl --sit-entity-fair --sit-q-contrast $CL; wait
s() { grep -E "words,|question boundaries|cost per" $1 | cut -c1-160; }
for r in qa2s_qc qa2s_qccl qa2_efqc qa2_efqccl; do echo "== $r"; cat runs/exp41/R_$r.score; s runs/exp41/R_$r.log; done
echo "== story qc"; tail -1 runs/exp36/R_qc_story.score; s runs/exp36/R_qc_story.log
echo "== pilot qc"; cat runs/exp38/R_bl_pilot_qc.score; s runs/exp38/R_bl_pilot_qc.log
