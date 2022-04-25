/*
 * Author: Dylan Turner
 * Description: Helper functions for neuron manipulation
 */

use std::sync::Arc;
use rand::{
    Rng, thread_rng
};
use tokio::sync::Mutex;
use crate::data::RawGameInfo;

pub const NEURON_ACTIVATION_THRESH: f64 = 0.75;
pub const TRAIT_SWAP_CHANCE: f64 = 0.75;
pub const WEIGHT_MUTATE_CHANCE: f64 = 0.25;
pub const WEIGHT_MUTATE_AMOUNT: f64 = 0.0015;
pub const OFFSET_MUTATE_CHANCE: f64 = 0.05;
pub const OFFSET_MUTATE_AMOUNT: f64 = 0.1;

// Helper func for getting a random number
pub async fn gen_rand_weight_and_offset() -> (f64, f64) {
    let mut rng = thread_rng();
    (rng.gen_range(-1.0..=1.0), rng.gen_range(-0.5..=0.5))
}

/*
 * Helper func for generating data
 * I like the elegance of the spawn/map version, but it's slow
 * The activations and rand_gen_neurons could also use something similar, but again, it's slower
 */
pub async fn random_gen_weight_and_offset_vec(size: usize) -> Arc<Mutex<Vec<(f64, f64)>>> {
    let mut weight_offset_pairs = Vec::new();
    for _ in 0..size {
        weight_offset_pairs.push(gen_rand_weight_and_offset().await);
    }
    Arc::new(Mutex::new(weight_offset_pairs))

    /*let handles: Vec<JoinHandle<f64>> = vec![0.0; size].iter().map(|_| {
        spawn(gen_rand_weight_and_offset())
    }).collect();
    Arc::new(Mutex::new(try_join_all(handles).await.unwrap()))*/
}

/*
 * Helper func for generating collections of data
 * Note that doing it in parallel is significantly SLOWER than sequential due to overhead!
 */
pub async fn random_gen_neurons(
        size: usize, neuron_size: usize) -> Vec<Arc<Mutex<Vec<(f64, f64)>>>> {
    let mut neurons = Vec::new();
    for _ in 0..size {
        neurons.push(random_gen_weight_and_offset_vec(neuron_size).await);
    }
    neurons
}

/*
 * Helper function to get neuron activations
 * Slower to use parallelism like above
 */
pub async fn activations(
        neurons: &Vec<Arc<Mutex<Vec<(f64, f64)>>>>, game: Arc<RawGameInfo>) -> Vec<bool> {
    let mut activates = Vec::new();
    for neuron in neurons {
        activates.push(activated(neuron.clone(), game.clone()).await);
    }
    activates
}

// Helper function to get activated status of a Vec<f64>
pub async fn activated(neuron: Arc<Mutex<Vec<(f64, f64)>>>, game: Arc<RawGameInfo>) -> bool {
    neuron.lock().await.iter().zip(game.input_bits.iter()).map(|((weight, offset), input)| {
        weight * if *input {
            1.0
        } else {
            0.0
        } + offset
    }).sum::<f64>() > NEURON_ACTIVATION_THRESH
}

// Helper functions to trade between weights
pub async fn neuron_trade(
        a: &mut Arc<Mutex<Vec<(f64, f64)>>>, b: &mut Arc<Mutex<Vec<(f64, f64)>>>) {
    a.lock().await.iter_mut().zip(b.lock().await.iter_mut()).for_each(|(a_weight, b_weight)| {
        let mut rng = thread_rng();
        if rng.gen_bool(TRAIT_SWAP_CHANCE) {
            let old_a_weight = *a_weight;
            *a_weight = *b_weight;
            *b_weight = old_a_weight;
        }
    });
}

// Helper functions to trade between connections. Modifies a and b
pub async fn neurons_trade(
        a: &mut Vec<Arc<Mutex<Vec<(f64, f64)>>>>, b: &mut Vec<Arc<Mutex<Vec<(f64, f64)>>>>) {
    for (a_neuron, b_neuron) in a.iter_mut().zip(b.iter_mut()) {
        neuron_trade(a_neuron, b_neuron).await;
    }
}

// Helper function to randomly change a neuron. Modifies nueron
pub async fn neuron_mutate(neuron: &mut Arc<Mutex<Vec<(f64, f64)>>>) {
    neuron.lock().await.iter_mut().for_each(|(weight, offset)| {
        let mut rng = thread_rng();
        if rng.gen_bool(WEIGHT_MUTATE_CHANCE) {
            *weight = rng.gen_range(
                (*weight - WEIGHT_MUTATE_AMOUNT)..(*weight + WEIGHT_MUTATE_AMOUNT)
            );
        }
        let mut rng = thread_rng();
        if rng.gen_bool(OFFSET_MUTATE_CHANCE) {
            *offset = rng.gen_range(
                (*offset - OFFSET_MUTATE_AMOUNT)..(*offset + OFFSET_MUTATE_AMOUNT)
            );
        }
    });
}

// Helper functions to randomly mutate collection of neurons. Modifies them
pub async fn neurons_mutate(neurons: &mut Vec<Arc<Mutex<Vec<(f64, f64)>>>>) {
    for neuron in neurons {
        neuron_mutate(neuron).await;
    }
}
