# March Madness Predictor

## Description

A from scratch implementation of a genetic algorithm neural network used to predict scores for March Madness so I can pwn my friends.

This is not a serious attempt at a march madness predictor but rather an exercise to gain a deeper understanding of machine learning and improve my skills at Rust async code.

Data collected [from here](https://data.world/sports/ncaa-mens-march-madness).

It's also a library. The neural network code is decoupled into its own `genetic/` folder, so it could be reused by other Rust projects!

## Build/Run

You just need the Rust build system, `cargo`

__Train:__

`cargo run --release --bin mmp`

The release is important because it adds a MAJOR performance boost

__Predict:__

`cargo run --release --bin mmp -- --predict=<team a name>,<team a seed>,<team b name>,<team b seed>,<date>,<round>,<region>`

Round can be (use backslashes to escape spaces):
- Opening Round
- Round of 64
- Round of 32
- Sweet Sixteen
- Elite Eight
- National Semifinals
- National Championship

Region can be (use backslashes to escape spaces):
- West
- East
- Midwest
- South
- Southeast
- Southwest

Example:
`cargo run --release --bin mmp -- --predict=Arkansas,9,Iowa,8,3/15/85,Round\ of\ 64,West`
