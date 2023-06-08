# rusty-pivx-parser
A RPC based block parser that also stores the information into RocksDB

## Pre-Requisites

It relies on `pivx-rpc-rs` which in the current repo, the Cargo.toml refers to as "rusty_piv" you can rename it however you like but as of now there is no crate to install.

RocksDB is the database, given its speed and rust's there is little to no DB bottlenecks. We are also batch writing to the database for ease, so there could be some points if it stops abruptly, you will need to reindex the database, working on potentials for restarting from last point.

Currently, this still has debugging and benchmarking code within it. Output from running looks like below
```Block: "a70eb2005bc4ba5734021a12883b02e53680b53b80cf502c92928df99f0764f8", 890386
Elapsed getrawtransaction time: 104.048877ms
Transaction Length: 447
Retrieved Transaction Length: 447
Elapsed getrawtransaction time: 101.297982ms
Transaction Length: 1784
Retrieved Transaction Length: 1784
Elapsed getrawtransaction time: 104.602913ms
Transaction Length: 1385
Retrieved Transaction Length: 1385
Elapsed getrawtransaction time: 100.823004ms
Transaction Length: 3349
Retrieved Transaction Length: 3349
Elapsed loop time: 513.441314ms
Elapsed getblock time: 101.008802ms
```
