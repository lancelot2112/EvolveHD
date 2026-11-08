#!/bin/bash
# Exp 68: the spelling view told which segments are descriptions (exp 67's soracle arm), after the made-up practice
# stream with its magic spelled three ways (A: escapes after "magic"; B: spaced hex after "0 string"; C: escapes after
# "0 string"), then exp 66's real files. Binary sit-67b, unchanged. Optional args: "style rotation arm,...".
cd /home/claude/evolvehd
export B=runs/exp36/sit-67b
one() {
  sty=$1; s=$2; arm=$3; st=${sty}prereal$s; d=runs/exp68/${st}_$arm
  [ -s $d/report.txt ] && return
  case $arm in
    soracle) x="--desc-expert --desc-spell --desc-oracle runs/exp68/$st.descs" ;;
    spell) x="--desc-expert --desc-spell" ;;
    oracle) x="--desc-expert --desc-oracle runs/exp68/$st.descs" ;;
  esac
  rm -rf $d
  $B stack --data runs/exp68/$st.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp68/$st.starts $x --out-dir $d > $d.out 2> $d.log
}
export -f one
if [ $# -gt 0 ]; then echo "$@" | tr ',' '\n' | xargs -P 2 -L 1 bash -c 'one $0 $1 $2'; exit; fi
for s in 0 1 2 3; do for sty in A B C; do echo "$sty $s soracle"; done; done | xargs -P 2 -L 1 bash -c 'one $0 $1 $2'
