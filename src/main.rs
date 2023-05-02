#![allow(unused)]

use std::{collections::HashMap, time::Instant};

use csv::Reader;
use lib::{
    Csv, CsvString, DivinationCard, DivinationCardPrice, DivinationCardRecord,
    DivinationCardsSample, FixedCardName, Prices, CARDS, CARDS_N, LEGACY_CARDS,
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let csv1 = std::fs::read_to_string("example-1.csv").unwrap();
    let csv2 = std::fs::read_to_string("example-2.csv").unwrap();
    let csv3 = std::fs::read_to_string("example-3.csv").unwrap();

    let prices = Prices::fetch().await.unwrap();

    let s1 = DivinationCardsSample::create(Csv::CsvString(CsvString(csv1)), prices.clone());
    let s2 = DivinationCardsSample::create(Csv::CsvString(CsvString(csv2)), prices.clone());
    let s3 = DivinationCardsSample::create(Csv::CsvString(CsvString(csv3)), prices.clone());

    let s = DivinationCardsSample::merge(prices, &[s1, s2, s3]);
    dbg!(s.chaos(None));
    dbg!(s.chaos(Some(50.0)));
}
