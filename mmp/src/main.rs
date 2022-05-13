/*
 * Author: Dylan Turner
 * Description: Interface to march madness prediction tool
 */

mod data;

extern crate genetic;

use std::time::Instant;
use clap::{
    Arg, Command, crate_version, ArgMatches
};
use genetic::{
    genetic::{
        gen_pop, test_and_sort, reproduce
    }, network::Network
};
use crate::data::{
    GameInfo, TableEntry, NUM_INPUTS, NUM_OUTPUTS
};

// Neuron connection settings
pub const NEURON_ACTIVATION_THRESH: f64 = 0.60;
pub const TRAIT_SWAP_CHANCE: f64 = 0.80;
pub const WEIGHT_MUTATE_CHANCE: f64 = 0.65;
pub const WEIGHT_MUTATE_AMOUNT: f64 = 0.5;
pub const OFFSET_MUTATE_CHANCE: f64 = 0.25;
pub const OFFSET_MUTATE_AMOUNT: f64 = 0.05;

// Neural network settings
pub const LAYER_SIZES: [usize; 4] = [ 8, 32, 32, 16 ];

// Algortithm settings
const POP_SIZE: usize = 10;

const DATA_FILE_NAME: &'static str = "NCAA Mens March Madness Historical Results.csv";
const MODEL_FILE_NAME: &'static str = "model.mmp";
const NUM_GENS: usize = 10;

// Entry point
#[tokio::main]
async fn main() {
    let args = get_args();

    if !args.is_present("predict") {
        train().await;
    } else {
        predict(args.value_of("predict").unwrap()).await;
    }
}

// Train on march madness legacy data
pub async fn train() {
    println!("Training new March Madness Predictor Model");

    println!("Loading training data from {}", DATA_FILE_NAME);
    let games = GameInfo::collection_from_file(DATA_FILE_NAME);
    let games: Vec<(Vec<u8>, Vec<u8>)> = games.iter().map(|game| { // Redefines games
        (game.to_input_bits().to_vec(), game.to_output_bits().to_vec())
    }).collect();

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

    // One last test and sort after reproducing
    test_and_sort(&mut pop, &games).await;
    pop[0].save_model(MODEL_FILE_NAME).await;
}

// Load in a model and make a prediction
pub async fn predict(team_names: &str) {
    let table_data = team_names.split(",");
    let mut indexable_table_data = Vec::new();
    for item in table_data {
        indexable_table_data.push(item);
    }
    
    // A team, A seed, B team, B seed, date, round, region
    if indexable_table_data.len() != 7 {
        println!("Invalid input string!");
        return;
    }

    println!("Converting input into data...");
    let entry = TableEntry {
        winner: String::from(indexable_table_data[0]),
        win_seed: String::from(indexable_table_data[1]),
        loser: String::from(indexable_table_data[2]),
        lose_seed: String::from(indexable_table_data[3]),
        date: String::from(indexable_table_data[4]),
        round: String::from(indexable_table_data[5]),
        region: String::from(indexable_table_data[6]),

        win_score: String::from("0"),
        lose_score: String::from("0"),
        overtime: String::from("")
    };
    let game = GameInfo::from_table_entry(&entry);

    println!("Predicting!");
    let predictor = Network::from_file(MODEL_FILE_NAME);
    let result = predictor.result(&game.to_input_bits().to_vec()).await;

    println!("Predicted score for {}: {}", indexable_table_data[0], result[0]);
    println!("Predicted score for {}: {}", indexable_table_data[2], result[1]);
    println!("Expected overtimes: {}", result[2]);
}

// Note that data in Game prediction should be alphabetical team name
fn get_args() -> ArgMatches {
    Command::new("mmp")
        .version(crate_version!())
        .author("Dylan Turner <dylantdmt@gmail.com>")
        .about("March Madness Game Predictor")
        .arg(
            Arg::new("predict")
                .short('p')
                .long("predict")
                .takes_value(true)
                .help("Switches application to prediction mode")
        ).get_matches()
}
