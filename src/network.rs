/*
 * Author: Dylan Turner
 * Description: Neural Network that can be used to predict a game
 */

use std::{
    fs::{
        File, remove_file
    }, io::{
        Write, Read
    }, path::Path
};
use futures::future::try_join_all;
use tokio::{
    spawn,
    task::JoinHandle
};
use crate::{
    data::{
        RawGameInfo, NUM_INPUTS, NUM_OUTPUTS
    }, neuron::{
        NeuronConnectionMap, NeuronConnection, NeuronConnectionSet
    }
};

pub const NUM_LAYERS: usize = 4;
pub const LAYER_SIZES: [usize; NUM_LAYERS] = [ 8, 32, 32, 16 ];

#[derive(Debug, Clone)]
pub struct Network {
    pub maps: Vec<NeuronConnectionMap>
}

impl Network {
    /*
     * Generate all connections randomly
     * Unlike underlying functions these ARE faster when multithreaded
     */
    pub async fn new_random() -> Self {
        let mut handles: Vec<JoinHandle<NeuronConnectionMap>> = Vec::new();
        for i in 0..=NUM_LAYERS {
            handles.push(spawn(match i {
                0 => NeuronConnectionMap::new_random(LAYER_SIZES[i], NUM_INPUTS),
                NUM_LAYERS => NeuronConnectionMap::new_random(NUM_OUTPUTS, LAYER_SIZES[i - 1]),
                _ => NeuronConnectionMap::new_random(LAYER_SIZES[i], LAYER_SIZES[i - 1])
            }));
        }
        Self {
            maps: try_join_all(handles).await.unwrap()
        }
    }

    // Cannot be parallelized.
    pub async fn result(&self, game: &RawGameInfo) -> Vec<u8> {
        let mut last_bits = game.clone();
        for map in self.maps.iter() {
            let activations = map.layer_activations(&last_bits).await;
            last_bits = RawGameInfo {
                input_bits: activations,
                output_bits: Vec::new()
            };
        }
        last_bits.input_bits.clone()
    }

    // Can't be parallelized bc mutation
    pub async fn random_trade(&mut self, other: &mut Self) {
        for i in 0..self.maps.len() {
            self.maps[i].trade_with(&mut other.maps[i]).await;
        }
    }

    // Can't be parallelized bc mutation
    pub async fn mutate(&mut self) {
        for map in self.maps.iter_mut() {
            map.mutate_all().await;
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
        let mut maps = Vec::new();
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

            let mut map = Vec::new();
            for _ in 0..out_layer_size {
                let mut conns = Vec::new();
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
                    conns.push(
                        NeuronConnection {
                            weight: f64::from_be_bytes(weight_data),
                            offset: f64::from_be_bytes(offset_data)
                        }
                    );
                }
                map.push(NeuronConnectionSet {
                    conns
                });
            }

            maps.push(NeuronConnectionMap {
                map
            });
        }

        Self {
            maps
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
        for map in self.maps.iter() {
            for conns in map.map.iter() {
                for conn in conns.conns.iter() {
                    let weight_data = conn.weight.to_be_bytes();
                    for k in 0..8 {
                        big_arr[x] = weight_data[k];
                        x += 1;
                    }
                    let offset_data = conn.offset.to_be_bytes();
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
