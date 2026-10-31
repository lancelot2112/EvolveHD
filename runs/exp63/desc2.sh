#!/bin/bash
# Exp 64: the description expert in the stack, told which segments are descriptions (oracle), and with its trust
# conditioned on the regime before each segment (learned). Formats stream fmt2, position sense on, binary sit-64a.
cd /home/claude/evolvehd
export B=runs/exp36/sit-64a
fm() { t=$1; shift; EVOLVEHD_PROGRESS=100000 $B stack --data runs/exp60/fmt2.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp60/fmt2.starts "$@" --out-dir runs/exp63/fmt_$t > runs/exp63/fmt_$t.out 2>&1; }
export -f fm
printf '%s\n' "oracle|--desc-expert --desc-oracle runs/exp60/fmt2.descs" "regbase|--regime 128,16 --regime-no-node" "regdesc|--regime 128,16 --regime-no-node --desc-expert" "regctx|--regime 128,16 --regime-no-node --desc-expert --desc-ctx-regime" "desc64|--desc-expert" | xargs -P 2 -I{} bash -c 'IFS="|" read t x <<< "{}"; fm $t $x'
# The two learning-by-trust runs (binary sit-64b):
#   B=runs/exp36/sit-64b fm lt --regime 128,16 --regime-no-node --desc-expert --desc-ctx-regime --desc-learn-trust
#   B=runs/exp36/sit-64b fm lt0 --desc-expert --desc-learn-trust
# Exp 65, the role gate (binary sit-65a):
#   B=runs/exp36/sit-65a fm role --desc-expert --desc-role
