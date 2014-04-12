# The Computer Language Benchmarks Game
# http://shootout.alioth.debian.org/
# contributed by Dominique Wahli
# 2to3
# mp by Ahmad Syukri
# modified by Justin Peel

from sys import stdin
from re import sub, findall
from multiprocessing import Pool

def init(arg):
    global seq
    seq = arg

def var_find(f):
    return len(findall(f, seq))

def main():
    seq = stdin.read()
    ilen = len(seq)

    seq = sub('>.*\n|\n', '', seq)
    clen = len(seq)

    pool = Pool(initializer = init, initargs = (seq,))

    variants = (
          'agggtaaa|tttaccct',
          '[cgt]gggtaaa|tttaccc[acg]',
          'a[act]ggtaaa|tttacc[agt]t',
          'ag[act]gtaaa|tttac[agt]ct',
          'agg[act]taaa|ttta[agt]cct',
          'aggg[acg]aaa|ttt[cgt]ccct',
          'agggt[cgt]aa|tt[acg]accct',
          'agggta[cgt]a|t[acg]taccct',
          'agggtaa[cgt]|[acg]ttaccct')
    for f in zip(variants, pool.imap(var_find, variants)):
        print(f[0], f[1])

    subst = {
          'B' : '(c|g|t)', 'D' : '(a|g|t)',   'H' : '(a|c|t)', 'K' : '(g|t)',
          'M' : '(a|c)',   'N' : '(a|c|g|t)', 'R' : '(a|g)',   'S' : '(c|g)',
          'V' : '(a|c|g)', 'W' : '(a|t)',     'Y' : '(c|t)'}
    for f, r in list(subst.items()):
        seq = sub(f, r, seq)

    print()
    print(ilen)
    print(clen)
    print(len(seq))

if __name__=="__main__":
    main()
