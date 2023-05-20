// Author: Dylan Turner <dylan.turner@tutanota.com>
//! Convert CSV game data into raw bits for IO with scratch_genetic

use std::error::Error;
use clap::ValueEnum;
use csv::Reader;
use serde::Deserialize;

/// Input bits
pub const NUM_INPUTS: usize = 67 * 8;

/// Output bits
pub const NUM_OUTPUTS: usize = 3 * 8;

/// Maximum length for a team name
pub const NAME_LEN: usize = 32;

/// What level of the tournament a game took place in
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Round {
    OpeningRound,
    RoundOf64,
    RoundOf32,
    Sweet16,
    Elite8,
    Semifinals,
    Championship
}

impl Round {
    /// For storing as bits. Can't use #[repr(u8)] as it could break serde
    pub fn to_u8(self) -> u8 {
        match self {
            Round::OpeningRound => 0,
            Round::RoundOf64 => 1,
            Round::RoundOf32 => 2,
            Round::Sweet16 => 3,
            Round::Elite8 => 4,
            Round::Semifinals => 5,
            Round::Championship => 6
        }
    }
}

/// What region were the teams from (use Option<Region> as none for Championship)
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Region {
    East,
    Midwest,
    South,
    Southeast,
    Southwest,
    West
}

impl Region {
    /// For storing as bits. Can't use #[repr(u8)] as it could break serde
    pub fn to_u8(self) -> u8 {
        match self {
            // Note: not 0 bc we could have no region
            Region::East => 1,
            Region::Midwest => 2,
            Region::South => 3,
            Region::Southeast => 4,
            Region::Southwest => 5,
            Region::West => 6
        }
    }
}

/// What a line of the CSV file looks like
#[derive(Deserialize, Clone, Debug)]
pub struct Game {
    /// In the format 85, 86... 12, ... 18
    pub year: [char; 2],
    pub round: Round,
    pub region: Option<Region>,
    pub winner_seed: u8,
    pub winner_name: [char; NAME_LEN],
    pub winner_score: u8,
    pub loser_seed: u8,
    pub loser_name: [char; NAME_LEN],
    pub loser_score: u8,
    pub overtime: u8
}

impl Game {
    /// Read in the columns of the table
    pub fn vec_from_file(fname: &str) -> Result<Vec<Self>, Box<dyn Error>> {
        let mut data = Vec::new();

        let mut reader = Reader::from_path(fname)?;
        for result in reader.deserialize() {
            data.push(result?);
        }

        Ok(data)
    }

    /// "Output bits are the scores" we can just store it as three bytes
    pub fn to_output_bits(self) -> [u8; NUM_OUTPUTS / 8] {
        if self.winner_seed >= self.loser_seed {
            [ self.winner_score, self.loser_score, self.overtime ]
        } else {
            [ self.loser_score, self.winner_score, self.overtime ]
        }
    }

    /// Convert the data to bits for the algorithm
    ///
    /// Note:
    ///
    /// - Data set is Winner/Loser, so convert to just high seed/low seed to reduce bias
    /// - Round & Region (& left over): 00 xxx yyy
    /// - Winner seed, loser seed => high seed and low seed: xxxx yyyy
    ///
    /// In total: Year, R&R (1), HSS&LSS (1), HS Name (32), LS Name (32) = 67 B
    pub fn to_input_bits(self) -> Result<[u8; NUM_INPUTS / 8], Box<dyn Error>> {
        let mut bits = [0; NUM_INPUTS / 8];

        // Years are 1985 through 2018, 2018-1985=33, so we can store that in a single u8
        bits[0] = self.year.iter().collect::<String>().parse::<u8>()?;
        bits[1] = (self.round.to_u8() << 3)
            + if self.region.is_some() {
                self.region.unwrap().to_u8()
            } else {
                0
            };
        if self.winner_seed >= self.loser_seed {
            bits[3] = ((self.winner_seed & 0x0F) << 4) + (self.loser_seed & 0x0F);
        } else {
            bits[3] = ((self.loser_seed & 0x0F) << 4) + (self.winner_seed & 0x0F);
        }

        if self.winner_seed >= self.loser_seed {
            let mut i = 0;
            for c in self.winner_name {
                bits[4 + i] = c as u8;
                i += 1;
            }
            for c in self.loser_name {
                bits[4 + i] = c as u8;
                i += 1;
            }
        } else {
            let mut i = 0;
            for c in self.loser_name {
                bits[4 + i] = c as u8;
                i += 1;
            }
            for c in self.winner_name {
                bits[4 + i] = c as u8;
                i += 1;
            }
        }

        Ok(bits)
    }
}

