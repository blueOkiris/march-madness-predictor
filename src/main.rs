/*
 * Author: Dylan Turner
 * Description: Interface to march madness prediction tool
 */

mod data;
mod args;

use std::{
    time::Instant,
    error::Error
};
use clap::Parser;
use scratch_genetic::genetic::{
    gen_pop, test_and_sort, reproduce, load_and_predict, export_model
};
use crate::{
    args::{
        CliArgs, PredictorCommands
    }, data::{
        Game, NUM_INPUTS, NUM_OUTPUTS, Round, Region, NAME_LEN
    }
};

// Neuron connection settings
pub const NEURON_ACTIVATION_THRESH: f64 = 0.60;
pub const TRAIT_SWAP_CHANCE: f64 = 0.80;
pub const WEIGHT_MUTATE_CHANCE: f64 = 0.65;
pub const WEIGHT_MUTATE_AMOUNT: f64 = 0.5;
pub const OFFSET_MUTATE_CHANCE: f64 = 0.25;
pub const OFFSET_MUTATE_AMOUNT: f64 = 0.05;

// Neural network settings
pub const LAYER_SIZES: [usize; 4] = [ 16, 32, 32, 2 ];

// Algortithm settings
const POP_SIZE: usize = 2000;

const DATA_FILE_NAME: &'static str = "march_madness_historical_data.csv";
const MODEL_FILE_NAME: &'static str = "model.mmp";
const NUM_GENS: usize = 1000;

// Entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match CliArgs::parse().command {
        PredictorCommands::Train => train().await,
        PredictorCommands::Predict {
            year, round, region,
            high_seed, high_seed_team,
            low_seed, low_seed_team
        } => predict(
            year.as_str(), round, region,
            high_seed, high_seed_team.as_str(),
            low_seed, low_seed_team.as_str()
        ).await
    }
}

// Train on march madness legacy data
pub async fn train() -> Result<(), Box<dyn Error>> {
    println!("Training new March Madness Predictor Model");

    println!("Loading training data from {}", DATA_FILE_NAME);
    let games = Game::vec_from_file(DATA_FILE_NAME)?;
    let games: Vec<(Vec<u8>, Vec<u8>)> = games.iter().map(|game| {( // Redefines games
        game.clone().to_input_bits().expect("Failed to convert to bits.").to_vec(),
        game.clone().to_output_bits().to_vec()
    )}).collect();

    println!("Generating randomized population");
    let now = Instant::now();
    let mut pop = gen_pop(
        POP_SIZE,
        LAYER_SIZES.to_vec(), NUM_INPUTS, NUM_OUTPUTS,
        NEURON_ACTIVATION_THRESH, TRAIT_SWAP_CHANCE,
        WEIGHT_MUTATE_CHANCE, WEIGHT_MUTATE_AMOUNT,
        OFFSET_MUTATE_CHANCE, OFFSET_MUTATE_AMOUNT
    ).await;
    let elapsed = now.elapsed();
    println!("Generation took {}s", elapsed.as_secs_f64());

    println!("Starting training");
    for i in 0..NUM_GENS {
        println!("Generation {} / {}", i, NUM_GENS);
        test_and_sort(&mut pop, &games).await;
        reproduce(&mut pop).await;
    }

    // Save algorithm
    println!("Saving model to {}", MODEL_FILE_NAME);
    export_model(MODEL_FILE_NAME, &pop[0]).await;

    Ok(())
}

// Load in a model and make a prediction
pub async fn predict(
        year: &str, round: Round, region: Option<Region>,
        high_seed: u8, high_seed_team: &str,
        low_seed: u8, low_seed_team: &str) -> Result<(), Box<dyn Error>> {
    let _ = year.parse::<u8>()?;
    if year.len() < 2 {
        Err("Invalid year given!")?;
    }
    if high_seed_team.len() > NAME_LEN {
        Err(format!("Team name {} is longer than {} characters.", high_seed_team, NAME_LEN))?;
    }
    if low_seed_team.len() > NAME_LEN {
        Err(format!("Team name {} is longer than {} characters.", low_seed_team, NAME_LEN))?;
    }

    println!("Converting input into data...");
    let game = Game {
        year: [
            year.chars().collect::<Vec<char>>()[0],
            year.chars().collect::<Vec<char>>()[1]
        ], round,
        region,
        winner_seed: high_seed, // I promise this isn't adding bias. They get sorted. Look at bits
        winner_name: name_to_chars(high_seed_team),
        winner_score: 0,
        loser_seed: low_seed,
        loser_name: name_to_chars(low_seed_team),
        loser_score: 0,
        overtime: 0
    };

    println!("Predicting!");
    let result = load_and_predict(MODEL_FILE_NAME, &game.to_input_bits()?.to_vec()).await;

    println!("Predicted score for {}: {}", high_seed_team, result[0]);
    println!("Predicted score for {}: {}", low_seed_team, result[1]);
    println!("Expected overtimes: {}", result[2]);

    Ok(())
}

/// Convert an &str team name into a char array of fixed size
fn name_to_chars(name: &str) -> [char; NAME_LEN] {
    let mut list = ['\0'; NAME_LEN];
    let mut i = 0;
    for c in name.chars() {
        list[i] = c;
        i += 1;
        if i >= NAME_LEN {
            break;
        }
    }
    list
}

