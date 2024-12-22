Repurpose https://docs.rs/csv/latest/csv/tutorial/index.html#reading-with-serde
to process the mteam dashboard actions csv file using rust.

The goal is to build a simple microservice that will read csv file over https, process and produce the data plotly needs.
```shell
cargo build  
./target/debug/mteam-dashboard-action-processor timeline-multiplayer-09182024.csv
#if you need tests to print to the console using println then use
cargo test -- --nocapture
```

To troubleshoot particular row:
```shell
head -n 682 timeline-multiplayer-09182024.csv | tail -n1
```

## Code Coverage

rustup component add llvm-tools-preview # needed for grcov
cargo install grcov
CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html --llvm-path /Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/bin