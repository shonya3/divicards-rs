#![allow(unused)]

use std::collections::HashMap;

use csv::Reader;
use lib::{
    Csv, CsvString, DivinationCard, DivinationCardPrice, DivinationCardRecord,
    DivinationCardsSample, FixedCardName, CARDS, LEGACY_CARDS,
};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    #[derive(Deserialize, Debug, Serialize)]
    struct PriceData {
        lines: Vec<DivinationCardPrice>,
    }

    let actual = DivinationCardPrice::actual().await.unwrap();
    let price_data: PriceData = serde_json::from_str(&actual).unwrap();
    let prices = price_data.lines;

    let s = std::fs::read_to_string("example-2.csv").unwrap();
    let mut sample = DivinationCardsSample::default();
    sample
        .csv(Csv::CsvString(CsvString(s)))
        .price(prices.clone())
        .polished();

    // dbg!(sample);

    // dbg!(sample.polished.clone());

    std::fs::write("polished.csv", sample.polished.0);

    let s = std::fs::read_to_string("polished.csv").unwrap();
    let mut sample = DivinationCardsSample::default();
    sample
        .csv(Csv::CsvString(CsvString(s)))
        .price(prices)
        .polished();

    dbg!(sample.cards);
    std::fs::write("amount.csv", sample.polished.0);
}
