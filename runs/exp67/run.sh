#!/bin/bash
# Exp 67: the spelling view (--desc-spell) on exp 66's streams. Arms: spell (guessing), srole (exp 65's best role
# gate), soracle (told the description segments). Exp 66's base/desc/role/oracle runs are the comparison (the binary
# reproduces them exactly without --desc-spell). Binary sit-67b (each view learns as if alone; sit-67a split the credit, piloted on prereal0 only).
cd /home/claude/evolvehd
export B=runs/exp36/sit-67b
one() {
  st=$1; arm=$2; d=runs/exp67/${st}_$arm
  [ -s $d/report.txt ] && return
  case $arm in
    spell) x="--desc-expert --desc-spell" ;;
    srole) x="--desc-expert --desc-spell --desc-role --desc-role-contrast --desc-role-null 2" ;;
    soracle) x="--desc-expert --desc-spell --desc-oracle runs/exp66/$st.descs" ;;
  esac
  ROLE_DUMP=1 $B stack --data runs/exp66/$st.bin --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act --file-starts runs/exp66/$st.starts $x --out-dir $d > $d.out 2> $d.err
  grep "^ROLE" $d.err > $d.dump; grep -v "^ROLE" $d.err > $d.log; rm $d.err
}
export -f one
if [ $# -gt 0 ]; then echo "$@" | tr ',' '\n' | xargs -P 2 -L 1 bash -c 'one $0 $1'; exit; fi
for p in pre ""; do for s in 0 1 2 3; do for a in soracle spell srole; do echo "${p}real$s $a"; done; done; done | xargs -P 2 -L 1 bash -c 'one $0 $1'
