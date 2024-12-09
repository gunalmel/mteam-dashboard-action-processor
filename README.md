Repurpose https://docs.rs/csv/latest/csv/tutorial/index.html#reading-with-serde
to process the mteam dashboard actions csv file using rust.

The goal is to build a simple microservice that will read csv file over https, process and produce the data plotly needs.

cargo build
./target/debug/mteam-dashboard-action-processor timeline-multiplayer-09182024.csv

To troubleshoot particular row:
head -n 682 timeline-multiplayer-09182024.csv | tail -n1