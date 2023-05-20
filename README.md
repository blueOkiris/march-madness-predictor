# March Madness Predictor

## Description

A from scratch implementation of a genetic algorithm neural network used to predict scores for March Madness so I can pwn my friends.

This is not a serious attempt at a march madness predictor but rather an exercise to gain a deeper understanding of machine learning and improve my skills at Rust async code.

Data collected [from here](https://data.world/sports/ncaa-mens-march-madness).

Library used is something I've separated for use in other AI projects. It is maintained [here](https://github.com/blueOkiris/scratch_genetic).

## Build/Run

You just need the Rust build system, `cargo`

__Train:__

`cargo run --release -- train`

The release is important because it adds a MAJOR performance boost

__Predict:__

`cargo run --release -- predict <year> <round> <optionally region> <higher seed> <higher seed team> <lower seed> <lower seed team>`

Round can be (use backslashes to escape spaces):
- OpeningRound
- RoundOf64
- RoundOf32
- Sweet16
- Elite8
- Semifinals
- Championship

Region can be (use backslashes to escape spaces):
- East
- Midwest
- South
- Southeast
- Southwest
- West

Example:
`cargo run --release -- predict 23 RoundOf64 West 9 Arkansas 8 Iowa`
