/*
 * Author: Dylan Turner
 * Description: Helper functions for performing the genetic algorithm
 */

use std::time::Instant;
use futures::future::try_join_all;
use tokio::{
    spawn, task::JoinHandle
};
use crate::network::Network;

/*
 * Generate starting batch
 * Non-parallel version tested and slower
 */
pub async fn gen_pop(
        pop_size: usize,
        layer_sizes: Vec<usize>, num_inputs: usize, num_outputs: usize,
        activation_thresh: f64, trait_swap_chance: f64,
        weight_mutate_chance: f64, weight_mutate_amount: f64,
        offset_mutate_chance: f64, offset_mutate_amount: f64) -> Vec<Network> {
    let mut pop_funcs = Vec::new();
    for _ in 0..pop_size {
        pop_funcs.push(spawn(Network::new_random(
            layer_sizes.clone(), num_inputs, num_outputs,
            activation_thresh, trait_swap_chance,
            weight_mutate_chance, weight_mutate_amount,
            offset_mutate_chance, offset_mutate_amount
        )));
    }
    try_join_all(pop_funcs).await.unwrap()
}

/*
 * Test the population on the data and sort
 * Parallel is slower
 */
pub async fn test_and_sort(pop: &mut Vec<Network>, data_set: &Vec<(Vec<u8>, Vec<u8>)>) {
    let now = Instant::now();
    let handles: Vec<JoinHandle<u64>> = pop.iter().map(|pred| {
        spawn(test_all(pred.clone(), data_set.clone()))
    }).collect();
    let results: Vec<u64> = try_join_all(handles).await.unwrap();
    let elapsed = now.elapsed();
    println!("Test took {}s", elapsed.as_secs_f64());

    // Attach results to the population and sort together
    let now = Instant::now();
    let pop_copy = pop.clone(); // Bc rust stuff
    let mut pop_and_res: Vec<(&Network, &u64)> = pop_copy.iter().zip(results.iter()).collect();
    pop_and_res.sort_by(|(_, res_a), (_, res_b)| {
        res_b.partial_cmp(res_a).unwrap()
    });

    // Extract new population
    for i in 0..pop_and_res.len() {
        pop[i] = pop_and_res[i].0.clone(); // Prefer to do an unzip and set, but not working
    }
    let elapsed = now.elapsed();
    println!("Sort took {}s", elapsed.as_secs_f64());

    let best = *pop_and_res[0].1;
    let max = data_set.len() * data_set[0].1.len();
    println!("Gen best: {} / {} = {}", best, max, (best as f64) / (max as f64));
}

// Load input and output data and test performance (# output bits right)
async fn test_all(pred: Network, data_set: Vec<(Vec<u8>, Vec<u8>)>) -> u64 {
    let mut sum = 0;
    for game in data_set.iter() {
        sum += single_test(pred.clone(), game).await;
    }
    sum
}

// The fitness function
async fn single_test(pred: Network, game: &(Vec<u8>, Vec<u8>)) -> u64 {
    let res = pred.result(&game.0).await;
    res.iter().zip(game.1.iter()).map(|(res_bit, expected)| {
        let mut bits_correct = 0;
        for i in 0..8 {
            if ((res_bit >> i) & 0x01) == ((expected >> i) & 0x01) {
                bits_correct += 1;
            }
        }
        bits_correct
    }).sum()
}

// Take the top half of population and reproduce to make a better population (expects sorted)
pub async fn reproduce(pop: &mut Vec<Network>) {
    let now = Instant::now();
    let pop_size = pop.len();
    for i in 0..pop_size / 2 {
        if i % 2 == 0 { // We're doing every two parents in the first half
            let mut child_a = pop[i].clone();
            let mut child_b = pop[i + 1].clone();

            // Make a copy of parents at bottom of vector
            pop.push(child_a.clone());
            pop.push(child_b.clone());

            // Then trade and mutate to modify children from parents
            child_a.random_trade(&mut child_b).await;
            child_a.mutate().await;
            child_b.mutate().await;

            // Add the children
            pop.push(child_a);
            pop.push(child_b);
        }
    }

    // Remove the "bad" individuals and og parents (now at the top since we copied good to bottom)
    for _ in 0..pop_size {
        pop.remove(0);
    }
    let elapsed = now.elapsed();
    println!("Reproduction took {}s", elapsed.as_secs_f64());
}