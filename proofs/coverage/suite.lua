-- The coverage leg is one opt-in class: conducting it rebuilds the workspace
-- instrumented, runs every unit test under it, AND re-runs the whole black-box
-- suite through an instrumented archetect. Minutes, not seconds — so it must
-- never fire because someone typed `prova`. The `coverage` profile throws the
-- switch; `-s coverage` is the ad-hoc door.
suite.config { switch = "coverage" }
