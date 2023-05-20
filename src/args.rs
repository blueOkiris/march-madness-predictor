// Author(s): Dylan Turner <dylan.turner@tutanota.com>
//! Checks for CLI arguments and help messages

use clap::{
    Parser, Subcommand
};
use crate::data::{
    Round, Region
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: PredictorCommands
}

/// What you can do with the program
#[derive(Subcommand)]
pub enum PredictorCommands {
    /// Train on the data set .csv
    Train,

    /// Use a model to predict wins
    Predict {
        year: String,
        round: Round,
        region: Option<Region>,
        high_seed: u8,
        high_seed_team: String,
        low_seed: u8,
        low_seed_team: String
    }
}

