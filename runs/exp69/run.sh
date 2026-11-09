#!/bin/bash
# Exp 69: the token view alone, told which segments are descriptions, with words cut into units by a cut learned
# from the descriptions' own character statistics (env CUT=2: branching-entropy rise above 2 bits, substrings of up
# to 3 characters, see Cutter in src/acts.rs). Binary sit-69a. Streams: exp 68's practice C (libmagic spelling,
# runs/exp68/Cprereal*) and exp 66's original practice (runs/exp66/prereal*, style O), each then exp 66's real part.
# usage: run.sh ["style rotation,..."]
cd /home/claude/evolvehd
one() {
  sty=$1; s=$2; st=${sty}prereal$s; d=runs/exp69/${st}_cut
  src=runs/exp68; [ $sty = O ] && src=runs/exp66 && st=prereal$s
  [ -s $d/report.txt ] && return
  rm -rf $d
  env CUT=2 runs/exp36/sit-69a stack --data $src/$st.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts $src/$st.starts --desc-expert --desc-oracle $src/$st.descs --out-dir $d > $d.out 2> $d.log
}
export -f one
if [ $# -gt 0 ]; then echo "$@" | tr ',' '\n' | xargs -P 2 -L 1 bash -c 'one $0 $1'; exit; fi
for s in 0 1 2 3; do for sty in C O; do echo "$sty $s"; done; done | xargs -P 2 -L 1 bash -c 'one $0 $1'
