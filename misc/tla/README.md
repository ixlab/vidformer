# TLA+ Model

A TLA+ specification of vidformer's rendering-engine concurrency: the decode pool, decoder scheduling and GOP abandonment, and the filter/encode output pipeline (`vidformer/src/pool.rs` + `vidformer/src/dve.rs`).

To check: `java -cp tla2tools.jar tlc2.TLC -workers auto -config MC.cfg MC.tla` (get [tla2tools.jar](https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar) from the TLA+ release page). The bundled model doubles as a regression test for the decoder deadlock fixed in `35f6fd2`.
