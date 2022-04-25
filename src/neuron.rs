/*
 * Author: Dylan Turner
 * Description: Helper functions for neuron manipulation
 */

use rand::{
    Rng, thread_rng
};
use crate::data::RawGameInfo;

pub const NEURON_ACTIVATION_THRESH: f64 = 0.60;
pub const TRAIT_SWAP_CHANCE: f64 = 0.60;
pub const WEIGHT_MUTATE_CHANCE: f64 = 0.35;
pub const WEIGHT_MUTATE_AMOUNT: f64 = 0.015;
pub const OFFSET_MUTATE_CHANCE: f64 = 0.10;
pub const OFFSET_MUTATE_AMOUNT: f64 = 0.05;

// A neuron doesn't actually exist, only the connections between them
#[derive(Debug, Clone)]
pub struct NeuronConnection {
    pub weight: f64,
    pub offset: f64
}

impl NeuronConnection {
    pub async fn new_random() -> Self {
        let mut rng = thread_rng();
        Self {
            weight: rng.gen_range(-1.0..=1.0),
            offset: rng.gen_range(-0.5..=0.5)
        }
    }

    pub async fn mutate(&mut self) {
        let mut rng = thread_rng();
        if rng.gen_bool(WEIGHT_MUTATE_CHANCE) {
            self.weight = rng.gen_range(
                (self.weight - WEIGHT_MUTATE_AMOUNT)..(self.weight + WEIGHT_MUTATE_AMOUNT)
            );
        }
        let mut rng = thread_rng();
        if rng.gen_bool(OFFSET_MUTATE_CHANCE) {
            self.offset = rng.gen_range(
                (self.offset - OFFSET_MUTATE_AMOUNT)..(self.offset + OFFSET_MUTATE_AMOUNT)
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct NeuronConnectionSet {
    pub conns: Vec<NeuronConnection>
}

impl NeuronConnectionSet {
    /*
     * Generate data
     * I like the elegance of the spawn/map version, but it's slow
     * The activations and rand_gen_neurons could also use something similar, but again, it's slower
     */
    async fn new_random(size: usize) -> Self {
        let mut conns = Vec::new();
        for _ in 0..size {
            conns.push(NeuronConnection::new_random().await);
        }
        Self {
            conns
        }

        /*let handles: Vec<JoinHandle<NeuronConnection>> = vec![0.0; size].iter().map(|_| {
            spawn(NeuronConnection::new_random())
        }).collect();
        try_join_all(handles).await.unwrap())*/
    }

    // Get activated status of the actual neuron that falls in between the connections
    pub async fn activated(&self, game: &RawGameInfo) -> bool {
        let mut bit: u8 = 0; // Input bits are stored as, you guessed it, bits, so index by bit
        let mut byte_ind: usize = 0; // After bit goes over 8, we increase the byte
        let mut sum: f64 = 0.0;
        for conn in self.conns.iter() {
            let input = game.input_bits[byte_ind] >> (7 - bit) & 0x01;
            sum += conn.weight * input as f64 + conn.offset;
    
            // Move throught the input array
            bit += 1;
            if bit == 8 {
                byte_ind += 1;
                bit = 0;
            }
        }
        sum > NEURON_ACTIVATION_THRESH
    }

    // Trade with another connection set
    pub async fn trade_with(&mut self, other: &mut Self) {
        self.conns.iter_mut().zip(other.conns.iter_mut()).for_each(|(conn, other_conn)| {
            let mut rng = thread_rng();
            if rng.gen_bool(TRAIT_SWAP_CHANCE) {
                let old_conn = conn.clone();
                *conn = other_conn.clone();
                *other_conn = old_conn;
            }
        });
    }

    // Mutate all neuron connections
    pub async fn mutate_all(&mut self) {
        for conn in self.conns.iter_mut() {
            conn.mutate().await;
        };
    }
}

// This is essentially a mapping from one layer to another, so it's a connection
#[derive(Debug, Clone)]
pub struct NeuronConnectionMap {
    pub map: Vec<NeuronConnectionSet>
}

impl NeuronConnectionMap {
    /*
    * Generating collections of data
    * Note that doing it in parallel is significantly SLOWER than sequential due to overhead!
    */
    pub async fn new_random(size: usize, neuron_size: usize) -> Self {
        let mut map = Vec::new();
        for _ in 0..size {
            map.push(NeuronConnectionSet::new_random(neuron_size).await);
        }
        Self {
            map
        }
    }

    /*
    * Get neuron activations for layer between connection
    * Appears to be slower to use parallelism
    */
    pub async fn layer_activations(&self, game: &RawGameInfo) -> Vec<u8> {
        let mut activates = Vec::new();
        let mut curr_byte: u8 = 0; // Store results into packed bit arrays
        let mut bit: u8 = 0;
        for node in self.map.iter() {
            if node.activated(&game).await {
                curr_byte += 0x01 << (7 - bit);
            }
            bit += 1;

            if bit == 8 {
                activates.push(curr_byte);

                curr_byte = 0;
                bit = 0;
            }
        }
        activates
    }

    // Trade with another map
    pub async fn trade_with(&mut self, other: &mut Self) {
        for (set, other_set) in self.map.iter_mut().zip(other.map.iter_mut()) {
            set.trade_with(other_set).await;
        }
    }

    // Mutate all neuron connections
    pub async fn mutate_all(&mut self) {
        for set in self.map.iter_mut() {
            set.mutate_all().await;
        }
    }
}
