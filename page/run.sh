#!/bin/sh
# The page's four druid routes from the hand-written talent orders, into out/.
# STYLE picks the plays tried before the first talent (levels 1-9).
# Then: python3 page/page.py
set -e
cd "$(dirname "$0")/.."
cargo build --release --quiet
mkdir -p out
for r in balance moonkin_single feral cat_feral; do
  style=feral; kit=feral; case $r in balance|moonkin_single) style=single; kit=balance;; esac
  rm -f out/library_$kit.json   # the plays at 1-9 depend on STYLE: rebuilt for each route
  STYLE=$style target/release/forever druid-level hand $r out/$r.json | tail -1
done
