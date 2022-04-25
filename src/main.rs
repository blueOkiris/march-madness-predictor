/*
 * Author: Dylan Turner
 * Description: Interface to march madness prediction tool
 */

mod neuron;
mod network;
mod genetic;
mod data;

use std::{
    sync::Arc,
    time::Instant
};
use clap::{
    Arg, Command, crate_version, ArgMatches
};
use crate::{
    data::{
        GameInfo, RawGameInfo, DataSet, TableEntry
    }, genetic::{
        gen_pop, test_and_sort, reproduce
    }, network::Network
};

const DATA_FILE_NAME: &'static str = "NCAA Mens March Madness Historical Results.csv";
const MODEL_FILE_NAME: &'static str = "model.mmp";
const NUM_GENS: usize = 2;

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
    let games: Vec<Arc<RawGameInfo>> = games.iter().map(|game| { // Redefines games
        Arc::new(RawGameInfo {
            input_bits: game.to_input_bits().to_vec(),
            output_bits: game.to_output_bits().to_vec()
        })
    }).collect();
    let data_set = Arc::new(DataSet {
        games
    });

    println!("Generating randomized population");
    let now = Instant::now();
    let mut pop = gen_pop().await;
    let elapsed = now.elapsed();
    println!("Generation took {}s", elapsed.as_secs_f64());

    println!("Starting training");
    for i in 0..NUM_GENS {
        println!("Generation {} / {}", i, NUM_GENS);

        let now = Instant::now();
        test_and_sort(&mut pop, data_set.clone()).await;
        let elapsed = now.elapsed();
        println!("Test and sort took {}s", elapsed.as_secs_f64());

        let now = Instant::now();
        reproduce(&mut pop).await;
        let elapsed = now.elapsed();
        println!("Reproduction took {}s", elapsed.as_secs_f64());
    }

    // Save algorithm
    println!("Saving model to {}", MODEL_FILE_NAME);

    // One last test and sort after reproducing
    test_and_sort(&mut pop, data_set.clone()).await;
    pop[0].lock().await.save_model(MODEL_FILE_NAME).await;
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
    let raw_game = Arc::new(RawGameInfo {
        input_bits: game.to_input_bits().to_vec(),
        output_bits: game.to_output_bits().to_vec()
    });

    println!("Predicting!");
    let predictor = Network::from_file(MODEL_FILE_NAME);
    let result = predictor.result(raw_game).await;

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
