#!/usr/bin/env perl
# Rewrites a patch or file from upstream dsntk to rheon's crate names: every crate in
# @crates is renamed both as a package name (dsntk-feel-number) and as a Rust path
# (dsntk_feel_number). Crates not listed pass through unchanged. Usage:
#   git format-patch -1 --stdout <sha> | perl scripts/upstream-rename.pl | git am -3
# Extend @crates when another crate is renamed (see docs/FORK.md, "Rules").
use strict; use warnings;
my @crates = qw(feel-number feel-evaluator feel-parser feel-temporal feel-regex model-evaluator feel);
local $/; my $t = <STDIN>;
for my $c (@crates) {
  (my $u = $c) =~ s/-/_/g;
  $t =~ s/\bdsntk-\Q$c\E(?![-\w])/rheon-$c/g;
  $t =~ s/\bdsntk_$u\b/rheon_$u/g;
}
print $t;
