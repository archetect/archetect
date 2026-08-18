-- The unit-test leg is one opt-in class: conducting the deputy COMPILES the
-- workspace, so it must never fire because someone typed `prova`. One
-- declaration gates the directory; the `ut` profile (and `-s ut`) throws it.
suite.config { switch = "ut" }
