/*
 * Author: Dylan Turner
 * Description: Neural Network that can be used to predict a game
 */

use std::{
    fs::{
        File, remove_file
    }, io::{
        Write, Read
    }, sync::Arc,
    path::Path
};
use futures::future::try_join_all;
use tokio::{
    spawn,
    sync::Mutex,
    task::JoinHandle
};
use crate::{
    data::{
        RawGameInfo, NUM_INPUTS, NUM_OUTPUTS
    }, neuron::{
        random_gen_neurons, activations, neurons_trade, neurons_mutate
    }
};

pub const NUM_LAYERS: usize = 4;
pub const LAYER_SIZES: [usize; NUM_LAYERS] = [ 8, 32, 32, 16 ];

#[derive(Debug)]
pub struct Network {
    pub layer_conn_set: Vec<Vec<Arc<Mutex<Vec<(f64, f64)>>>>>
}

impl Network {
    /*
     * Generate all connections randomly
     * Unlike underlying functions these ARE faster when multithreaded
     */
    pub async fn new_random() -> Arc<Mutex<Self>> {
        let mut handles: Vec<JoinHandle<Vec<Arc<Mutex<Vec<(f64, f64)>>>>>> = Vec::new();
        for i in 0..=NUM_LAYERS {
            handles.push(spawn(match i {
                0 => random_gen_neurons(LAYER_SIZES[i], NUM_INPUTS),
                NUM_LAYERS => random_gen_neurons(NUM_OUTPUTS, LAYER_SIZES[i - 1]),
                _ => random_gen_neurons(LAYER_SIZES[i], LAYER_SIZES[i - 1])
            }));
        }
        let layer_conn_set = try_join_all(handles).await.unwrap();

        Arc::new(Mutex::new(Self {
            layer_conn_set
        }))
    }

    // Cannot be parallelized
    pub async fn result(&self, game: Arc<RawGameInfo>) -> Vec<u8> {
        let mut last_bits = game;
        for layer_conn in self.layer_conn_set.iter() {
            let layer = activations(layer_conn, last_bits).await;
            last_bits = Arc::new(RawGameInfo {
                input_bits: layer,
                output_bits: Vec::new()
            });
        }
        last_bits.input_bits.clone()
    }

    // Can't be parallelized bc mutation
    pub async fn random_trade(&mut self, other: &mut Self) {
        for i in 0..self.layer_conn_set.len() {
            neurons_trade(&mut self.layer_conn_set[i], &mut other.layer_conn_set[i]).await;
        }
    }

    // Can't be parallelized bc mutation
    pub async fn mutate(&mut self) {
        for layer_conn in self.layer_conn_set.iter_mut() {
            neurons_mutate(layer_conn).await;
        }
    }

    // Don't care to optimize. Performance doesn't really matter
    pub fn from_file(fname: &str) -> Self {
        let mut big_arr_size = NUM_INPUTS * LAYER_SIZES[0];
        for i in 0..NUM_LAYERS - 1 {
            big_arr_size += LAYER_SIZES[i] * LAYER_SIZES[i + 1];
        }
        big_arr_size += LAYER_SIZES[NUM_LAYERS - 1] * NUM_OUTPUTS;
        big_arr_size *= 16; // 8 bytes for weight and 8 for offset
        let mut big_arr = vec![0; big_arr_size];

        let mut file = File::open(fname).expect("Failed to open model file!");
        file.read_exact(&mut big_arr).expect("Failed to save model to file!");

        let mut x = 0;
        let mut layer_conn_set = Vec::new();
        for i in 0 as usize..=NUM_LAYERS {
            let in_layer_size = if i == 0 {
                NUM_INPUTS
            } else {
                LAYER_SIZES[i - 1]
            };
            let out_layer_size = if i == NUM_LAYERS {
                NUM_OUTPUTS
            } else {
                LAYER_SIZES[i]
            };

            let mut layer_conn = Vec::new();
            for _ in 0..out_layer_size {
                let mut neurons = Vec::new();
                for _ in 0..in_layer_size {
                    let mut weight_data = [0; 8];
                    for k in 0..8 {
                        weight_data[k] = big_arr[x];
                        x += 1;
                    }
                    let mut offset_data = [0; 8];
                    for k in 0..8 {
                        offset_data[k] = big_arr[x];
                        x += 1;
                    }
                    neurons.push(
                        (f64::from_be_bytes(weight_data), f64::from_be_bytes(offset_data))
                    );
                }
                layer_conn.push(Arc::new(Mutex::new(neurons)));
            }

            layer_conn_set.push(layer_conn);
        }

        Self {
            layer_conn_set
        }
    }

    // Don't care to optimize. Performance doesn't really matter
    pub async fn save_model(&self, fname: &str) {
        let mut big_arr_size = NUM_INPUTS * LAYER_SIZES[0];
        for i in 0..NUM_LAYERS - 1 {
            big_arr_size += LAYER_SIZES[i] * LAYER_SIZES[i + 1];
        }
        big_arr_size += LAYER_SIZES[NUM_LAYERS - 1] * NUM_OUTPUTS;
        big_arr_size *= 16; // 8 bytes for weight and 8 for offset
        let mut big_arr = vec![0; big_arr_size];

        let mut x = 0;
        for layer_conn in self.layer_conn_set.iter() {
            for neuron in layer_conn {
                for (weight, offset) in  neuron.lock().await.iter() {
                    let weight_data = weight.to_be_bytes();
                    for k in 0..8 {
                        big_arr[x] = weight_data[k];
                        x += 1;
                    }
                    let offset_data = offset.to_be_bytes();
                    for k in 0..8 {
                        big_arr[x] = offset_data[k];
                        x += 1;
                    }
                }
            }
        }

        if Path::new(fname).exists() {
            remove_file(fname).unwrap();
        }

        let mut file = File::create(fname).expect("Failed to open model file!");
        file.write_all(&big_arr).expect("Failed to save model to file!");
    }
}
