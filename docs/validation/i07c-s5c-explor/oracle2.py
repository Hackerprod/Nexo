#!/usr/bin/env python3
"""Extension minima del oraculo (sin tocarlo): acepta anotaciones Array<...> (solo para el barrido 2).
Uso: import oracle2 as O2; O2.derive(src). Se apoya en scripts/i07c_s5c_oraculo.py de la rama i07c-estructural-r2."""
import os, re, sys
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(here, '..', '..', '..', '.scratch', 'ref', 'scripts'))
import i07c_s5c_oraculo as O

O.TOK = re.compile(r"\s+|//[^\n]*|(?P<id>[A-Za-z_][A-Za-z0-9_]*)|(?P<int>[0-9]+)|(?P<sym>==|[=<>+\-()\[\]{},;:])")


def ty(self):
    tk = self.eat()
    if tk[1] == "Array":
        self.eat("<")
        inner = self.ty()
        end = self.eat(">")
        return O.N("tyname", tk[2], end[3], name="Array", el=inner)
    return O.N("tyname", tk[2], tk[3], name=tk[1])


O.Parser.ty = ty
_orig = O.Checker.tyname


def tyname(self, n):
    if n.name == "Array":
        return O.arr(self.tyname(n.el))
    return _orig(self, n)


O.Checker.tyname = tyname
derive = O.derive
