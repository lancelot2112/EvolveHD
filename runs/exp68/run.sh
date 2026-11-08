#!/bin/bash
# Exp 68: the spelling view told which segments are descriptions (exp 67's soracle arm), after the made-up practice
# stream with its magic spelled three ways (A: escapes after "magic"; B: spaced hex after "0 string"; C: escapes after
# "0 string"), then exp 66's real files. Binary sit-67b, unchanged. Optional args: "style rotation arm,...".
# Arm tsplit (binary sit-68a): the token view alone, told the descriptions, with tokens cut into escape units
# (env TOKSPLIT, an oracle cut). Style O: exp 66's prereal streams (the original hex-note practice).
cd /home/claude/evolvehd
export B=runs/exp36/sit-67b
one() {
  sty=$1; s=$2; arm=$3; st=${sty}prereal$s; d=runs/exp68/${st}_$arm
  src=runs/exp68; [ $sty = O ] && src=runs/exp66 && st=prereal$s
  bin=$B; envs=""
  [ -s $d/report.txt ] && return
  case $arm in
    soracle) x="--desc-expert --desc-spell --desc-oracle $src/$st.descs" ;;
    spell) x="--desc-expert --desc-spell" ;;
    oracle) x="--desc-expert --desc-oracle $src/$st.descs" ;;
    tsplit) x="--desc-expert --desc-oracle $src/$st.descs"; bin=runs/exp36/sit-68a; envs="TOKSPLIT=1" ;;
  esac
  rm -rf $d
  env $envs $bin stack --data $src/$st.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts $src/$st.starts $x --out-dir $d > $d.out 2> $d.log
}
export -f one
if [ $# -gt 0 ]; then echo "$@" | tr ',' '\n' | xargs -P 2 -L 1 bash -c 'one $0 $1 $2'; exit; fi
for s in 0 1 2 3; do for sty in A B C; do echo "$sty $s soracle"; done; done | xargs -P 2 -L 1 bash -c 'one $0 $1 $2'
