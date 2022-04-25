/*
 * Author: Dylan Turner
 * Description: Helper functions for performing the genetic algorithm
 */

use std::sync::Arc;
use futures::future::try_join_all;
use tokio::{
    spawn,
    sync::Mutex, task::JoinHandle
};
use crate::{
    data::{
        RawGameInfo, DataSet, NUM_OUTPUTS
    }, network::Network
};

const POP_SIZE: usize = 5000;

/*
 * Generate starting batch
 * Non-parallel version tested and slower
 */
pub async fn gen_pop() -> Vec<Arc<Mutex<Network>>> {
    let mut pop_funcs = Vec::new();
    for _ in 0..POP_SIZE {
        pop_funcs.push(spawn(Network::new_random()));
    }
    try_join_all(pop_funcs).await.unwrap()
}

/*
 * Test the population on the data and sort
 * Parallel is slower
 */
pub async fn test_and_sort(pop: &mut Vec<Arc<Mutex<Network>>>, data_set: Arc<DataSet>) {
    let handles: Vec<JoinHandle<u64>> = pop.iter().map(|pred| {
        spawn(test_all(pred.clone(), data_set.clone()))
    }).collect();
    let results: Vec<u64> = try_join_all(handles).await.unwrap();

    // Attach results to the population and sort together
    let pop_copy = pop.clone(); // Bc rust stuff
    let mut pop_and_res: Vec<(&Arc<Mutex<Network>>, &u64)> =
        pop_copy.iter().zip(results.iter()).collect();
    pop_and_res.sort_by(|(_, res_a), (_, res_b)| {
        res_b.partial_cmp(res_a).unwrap()
    });

    // Extract new population
    for i in 0..pop_and_res.len() {
        pop[i] = pop_and_res[i].0.clone(); // Prefer to do an unzip and set, but not working
    }

    let best = *pop_and_res[0].1;
    let max = data_set.clone().games.len() * NUM_OUTPUTS;
    println!("Gen best: {} / {} = {}", best, max, (best as f64) / (max as f64));
}

/*
 * Load input and output data and test performance (# output bits right)
 * Faster to do sequential here ~5s
 * Slowest function
 */
async fn test_all(pred: Arc<Mutex<Network>>, data_set: Arc<DataSet>) -> u64 {
    let mut sum = 0;
    for game in &data_set.games {
        sum += single_test(pred.clone(), game.clone()).await;
    }
    sum
}

// The fitness function
async fn single_test(pred: Arc<Mutex<Network>>, game: Arc<RawGameInfo>) -> u64 {
    let pred = &pred.lock().await;
    let res = pred.result(game.clone()).await;
    res.iter().zip(game.output_bits.iter()).map(|(res_bit, expected)| {
        let mut bits_correct = 0;
        for i in 0..8 {
            if ((res_bit >> i) & 0x01) == ((expected >> i) & 0x01) {
                bits_correct += 1;
            }
        }
        bits_correct
    }).sum()
}

// Take the top half of population and reproduce to make a better population
pub async fn reproduce(pop: &mut Vec<Arc<Mutex<Network>>>) {
    for i in 0..POP_SIZE / 2 {
        if i % 2 == 0 {
            // Make a copy of parents
            pop.push(pop[0].clone());
            pop.push(pop[1].clone());

            // Trade (in braces to unborrow; yeah it's weird)
            {
                let fst = &mut pop[i + POP_SIZE / 2].lock().await;
                let snd = &mut pop[i + POP_SIZE / 2 + 1].lock().await;
                fst.random_trade(snd).await;

                // Let children mutate
                fst.mutate().await;
                snd.mutate().await;
            }

            pop.remove(0);
            pop.remove(0);
        }
    }
}
