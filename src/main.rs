#![allow(unused)]

use std::collections::HashMap;

use csv::Reader;
use lib::{
    Csv, CsvString, DivinationCard, DivinationCardPrice, DivinationCardRecord,
    DivinationCardsSample, FixedCardName, CARDS, CARDS_N, LEGACY_CARDS,
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let s = std::fs::read_to_string("example-3.csv").unwrap();
    let prices = DivinationCardPrice::fetch().await.unwrap();
    let mut sample = DivinationCardsSample::from_prices(prices);
    sample.csv(Csv::CsvString(CsvString(s))).weight().polished();

    dbg!(sample);
}
