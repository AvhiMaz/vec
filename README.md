# raw vec / append vec

reimplementing `Vec<T>` and `AppendVec<T>` from scratch in unsafe rust.

`Vec<T>` is a growable heap-allocated array with push, pop, insert, remove<br>
and amortized O(1) growth via pointer doubling.

`AppendVec<T>` is a fixed-capacity, append-only, single-writer multi-reader<br>
vector that uses `Release`/`Acquire` atomics to safely publish elements to<br>
concurrent readers without locks, following the same pattern used in<br>
[agave's accounts-db](https://github.com/anza-xyz/agave/blob/master/accounts-db/src/append_vec.rs).
