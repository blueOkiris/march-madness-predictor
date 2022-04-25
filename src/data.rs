/*
 * Author: Dylan Turner
 * Description: Convert data from CSV to representation to raw "bits"
 */

use csv::{
    Error, Reader
};
use serde::Deserialize;

pub const NUM_INPUTS: usize = 68 * 8; // In bits
pub const NUM_OUTPUTS: usize = 24; // In bits
const NAME_LEN: usize = 32; // Max name length

/* This is how we'll load in data from the CSV */

#[derive(Debug, Deserialize, Clone)]
pub struct TableEntry {
    pub date: String,
    pub round: String,
    pub region: String,
    pub win_seed: String,
    pub winner: String,
    pub win_score: String,
    pub lose_seed: String,
    pub loser: String,
    pub lose_score: String,
    pub overtime: String
}

impl TableEntry {
    pub fn table_from_file(fname: &str) -> Result<Vec<Self>, Error> {
        let mut data = Vec::new();

        let mut reader = Reader::from_path(fname)?;
        for result in reader.deserialize() {
            let record: TableEntry = result?;
            data.push(record);
        }

        Ok(data)
    }
}

/*
 * And this is how we represent it internally
 * Note:
 * - Date: 00000 000.0 000000 0 - Day Month Year Left over bit
 */

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Round {
    Opening,
    R64,
    R32,
    Sweet16,
    Elite8,
    Semis,
    Championship,
    None
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Region {
    West,
    East,
    Midwest,
    South,
    Southeast,
    Southwest,
    None
}

#[derive(Debug, Clone, Copy)]
pub struct GameInfo {
    pub date: u16,
    pub round: Round,
    pub region: Region,
    pub win_seed: u8,
    pub winner: [u8; NAME_LEN],
    pub win_score: u8,
    pub lose_seed: u8,
    pub loser: [u8; NAME_LEN],
    pub lose_score: u8,
    pub overtime: u8
}

impl GameInfo {
    pub fn from_table_entry(entry: &TableEntry) -> Self {
        // Date is 0000 0000.0 0000000
        let date_str_pieces = entry.date.split('/');
        let mut date_pieces = Vec::new();
        for section in date_str_pieces {
            date_pieces.push(
                String::from(section).parse::<u8>().expect("Failed to parse date!")
            );
        }
        let date =
            (((date_pieces[0] & 0x0F) as u16) << 12)
            + (((date_pieces[1] & 0x1F) as u16) << 7)
            + date_pieces[2] as u16;

        // Round is a set of strings that reocurr
        let round = match entry.round.as_str() {
            "Opening Round" => Round::Opening,
            "Round of 64" => Round::R64,
            "Round of 32" => Round::R32,
            "Sweet Sixteen" => Round::Sweet16,
            "Elite Eight" => Round::Elite8,
            "National Semifinals" => Round::Semis,
            "National Championship" => Round::Championship,
            _ => Round::None
        };

        // Same with region
        let region = match entry.region.as_str() {
            "East" => Region::East,
            "West" => Region::West,
            "Midwest" => Region::Midwest,
            "South" => Region::South,
            "Southeast" => Region::Southeast,
            "Southwest" => Region::Southwest,
            _ => Region::None
        };

        let win_seed = entry.win_seed.parse::<u8>()
            .expect("Failed to parse win seed!");
        let lose_seed = entry.lose_seed.parse::<u8>()
            .expect("Failed to parse lose seed!");

        // Note: Update 32 in format str with NAME_LEN when changing NAME_LEN!
        let winner = format!("{: >32}", entry.winner).as_bytes()[0..NAME_LEN].try_into()
            .expect("Failed to parse winner name!");
        let loser = format!("{: >32}", entry.loser).as_bytes()[0..NAME_LEN].try_into()
            .expect("Failed to parse loser name!");

        let win_score = entry.win_score.parse::<u8>()
            .expect("Failed to parse win score!");
        let lose_score = entry.lose_score.parse::<u8>()
            .expect("Failed to parse lose score!");

        let overtime = if entry.overtime.ends_with(" OT") {
            entry.overtime.clone().replace(" OT", "").parse::<u8>()
                .expect("Failed to parse overtime!")
        } else {
            0
        };

        Self {
            date,
            round,
            region,
            win_seed,
            winner,
            win_score,
            lose_seed,
            loser,
            lose_score,
            overtime
        }
    }

    pub fn collection_from_file(fname: &str) -> Vec<Self> {
        let mut games = Vec::new();

        let table = TableEntry::table_from_file(fname).expect("Failed to open data file!");
        for game in table {
            games.push(GameInfo::from_table_entry(&game));

            /*
             * De-bias. We don't care who's the winner. We care about team scores
             * If the positive score is always in front, it bias' the algorithm
             * making it less correct
             */
            let mut rev_game = game.clone();
            let old_winner = rev_game.winner.clone();
            let old_win_seed = rev_game.win_seed.clone();
            let old_win_score = rev_game.win_score.clone();
            rev_game.winner = rev_game.loser;
            rev_game.win_seed = rev_game.lose_seed;
            rev_game.win_score = rev_game.lose_score;
            rev_game.loser = old_winner;
            rev_game.lose_seed = old_win_seed;
            rev_game.lose_score = old_win_score;
            games.push(GameInfo::from_table_entry(&rev_game));
        }

        games
    }

    // "Output bits are the scores" we can just store it as two bytes
    pub fn to_output_bits(self) -> [u8; NUM_OUTPUTS / 8] {
        [ self.win_score, self.lose_score, self.overtime ]
    }

    /*
    * And finally we convert it to "bits" for the algorithm
    * Note:
    * - Round & Region (& left over): 00 000 000
    * - Win seed, lose seed: 0000 0000
    * In total:
    * R&R (1), WS&LS (1), Winner Name (32), Loser Name (32), Overtime (1) = 69 u8s
    */
    pub fn to_input_bits(self) -> [u8; NUM_INPUTS / 8] {
        let mut bits = [0; NUM_INPUTS / 8];

        bits[0] = (self.date >> 8) as u8;
        bits[1] = (self.date & 0x000F) as u8;
        bits[2] = ((self.round as u8 & 0x07) << 3) + (self.region as u8 & 0x07);
        bits[3] = ((self.win_seed & 0x0F) << 4) + (self.lose_seed & 0x0F);

        let mut i = 0;
        for c in self.winner {
            bits[4 + i] = c as u8;
            i += 1;
        }
        for c in self.loser {
            bits[4 + i] = c as u8;
            i += 1;
        }

        bits
    }
}

/*
 * Just a helpful tool for dealing with the bits raw
 */

#[derive(Clone)]
pub struct RawGameInfo {
    pub input_bits: Vec<u8>, // Bit arrays. Could use Vec<bool>, but SHOULD be more efficient
    pub output_bits: Vec<u8>
}

#[derive(Clone)]
pub struct DataSet {
    pub games: Vec<RawGameInfo>
}
