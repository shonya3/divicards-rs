#![allow(unused)]
pub mod error;

use std::{collections::HashMap, path::Path, time::Instant};

use csv::{Reader, WriterBuilder};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CsvString(pub String);
pub enum Csv {
    // CsvFilePath(String),
    CsvString(CsvString),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivinationCardPrice {
    pub name: String,
    #[serde(rename(deserialize = "chaosValue"))]
    pub price: Option<f32>,
}

impl DivinationCardPrice {
    pub async fn fetch() -> Result<[Self; CARDS_N], reqwest::Error> {
        #[derive(Deserialize, Debug, Serialize)]
        struct PriceData {
            lines: Vec<DivinationCardPrice>,
        }

        let client = reqwest::Client::new();
        let url =
        "https://poe.ninja/api/data/itemoverview?league=Crucible&type=DivinationCard&language=en";
        let json = client.get(url).send().await?.text().await?;

        let price_data: PriceData = serde_json::from_str(&json).unwrap();
        let prices = price_data.lines;

        let prices_arr: [DivinationCardPrice; CARDS_N] = CARDS
            .into_iter()
            .map(|card| {
                let price = prices
                    .iter()
                    .find(|div_card_price| div_card_price.name == card)
                    .and_then(|v| v.price);
                DivinationCardPrice {
                    name: card.to_string(),
                    price,
                }
            })
            .collect::<Vec<Self>>()
            .try_into()
            .unwrap();

        Ok(prices_arr)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DivinationCardsSample {
    #[serde(with = "BigArray")]
    pub cards: [DivinationCardRecord; CARDS_N],
    pub not_cards: Vec<String>,
    pub fixed_names: Vec<FixedCardName>,
    pub polished: CsvString,
}

impl DivinationCardsSample {
    pub fn new(
        cards: [DivinationCardRecord; 438],
        not_cards: Vec<String>,
        fixed_names: Vec<FixedCardName>,
        polished: CsvString,
    ) -> DivinationCardsSample {
        DivinationCardsSample {
            cards,
            not_cards,
            fixed_names,
            polished,
        }
    }

    const CONDENSE_FACTOR: f32 = 2.0 / 3.0;

    pub fn price(&mut self, prices: [DivinationCardPrice; CARDS_N]) -> &mut Self {
        for card in &mut self.cards {
            let price = prices
                .iter()
                .find(|div_card_price| div_card_price.name == card.name)
                .and_then(|v| v.price);
            card.price = price;
        }
        self
    }

    pub fn polished(&mut self) -> &mut Self {
        let mut writer = csv::Writer::from_writer(vec![]);
        for card in self.cards.clone() {
            writer.serialize(card).unwrap();
        }
        let s = String::from_utf8(writer.into_inner().expect("Error with csv serialize"))
            .expect("Error");
        self.polished = CsvString(s);
        self
    }

    pub fn from_prices(prices: [DivinationCardPrice; CARDS_N]) -> Self {
        let cards: [DivinationCardRecord; CARDS_N] = prices
            .into_iter()
            .map(|DivinationCardPrice { name, price }| DivinationCardRecord {
                name,
                price,
                amount: Default::default(),
                weight: Default::default(),
            })
            .collect::<Vec<DivinationCardRecord>>()
            .try_into()
            .unwrap();

        DivinationCardsSample {
            cards,
            not_cards: Default::default(),
            fixed_names: Default::default(),
            polished: CsvString(String::from("")),
        }
    }

    pub fn csv(&mut self, csv: Csv) -> &mut Self {
        match csv {
            Csv::CsvString(s) => {
                let mut rdr = Reader::from_reader(s.0.as_bytes());
                for result in rdr.deserialize::<DivinationCardRecord>() {
                    if let Ok(mut record) = result {
                        match &record.is_card() {
                            true => {
                                // self.cards.get_mut(&record.name).unwrap().amount = record.amount
                                // self.cards.get_mut(index)
                                let mut card = self
                                    .cards
                                    .iter_mut()
                                    .find(|card| card.name == record.name)
                                    .unwrap();
                                card.amount = record.amount;
                            }
                            false => match record.fix_name() {
                                Some(fixed) => {
                                    let mut card = self
                                        .cards
                                        .iter_mut()
                                        .find(|card| card.name == record.name)
                                        .unwrap();
                                    card.amount = record.amount;
                                    self.fixed_names.push(fixed);
                                }
                                None => self.not_cards.push(record.name),
                            },
                        }
                    }
                }
                self
            }
        }
    }
}

impl Default for DivinationCardsSample {
    fn default() -> Self {
        let cards: [DivinationCardRecord; 438] = CARDS
            .clone()
            .into_iter()
            .map(|card| DivinationCardRecord {
                name: card.to_string(),
                price: Default::default(),
                amount: Default::default(),
                weight: Default::default(),
            })
            .collect::<Vec<DivinationCardRecord>>()
            .try_into()
            .unwrap();

        DivinationCardsSample {
            cards,
            fixed_names: vec![],
            not_cards: vec![],
            polished: CsvString(String::from("")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivinationCardRecord {
    pub name: String,
    #[serde(rename(deserialize = "calculated"))]
    pub price: Option<f32>,
    #[serde(alias = "stackSize")]
    pub amount: i32,
    pub weight: Option<f32>,
}

impl DivinationCardRecord {
    pub fn new(name: &str, price: Option<f32>, amount: Option<i32>) -> DivinationCardRecord {
        DivinationCardRecord {
            name: name.to_string(),
            price,
            amount: amount.unwrap_or_default(),
            weight: None,
        }
    }

    fn most_similar_card(name: &str) -> (String, f64) {
        let mut similarity_map = HashMap::<String, f64>::new();
        for card in CARDS {
            let similarity = strsim::normalized_damerau_levenshtein(&name, card);
            similarity_map.insert(card.to_string(), similarity);
        }

        let most_similar = similarity_map
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        (most_similar.0.to_owned(), most_similar.1.to_owned())
    }

    pub fn fix_name(&mut self) -> Option<FixedCardName> {
        match self.is_card() {
            true => None,
            false => self.fix_name_unchecked(),
        }
    }

    pub fn fix_name_unchecked(&mut self) -> Option<FixedCardName> {
        let (similar, score) = Self::most_similar_card(&self.name);
        match score >= 0.75 {
            true => {
                let fixed_name = FixedCardName::new(&self.name, &similar);
                self.name = similar;
                Some(fixed_name)
            }
            false => {
                let the_name = format!("The {}", &self.name);
                let (similar, score) = Self::most_similar_card(&the_name);
                match score >= 0.75 {
                    true => {
                        let fixed_name = FixedCardName::new(&self.name, &similar);
                        self.name = similar;
                        Some(fixed_name)
                    }
                    false => None,
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum IsACard {
    FixCardName(String, String),
    NotACard(String),
    Card,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FixedCardName {
    pub old: String,
    pub fixed: String,
}

impl FixedCardName {
    pub fn new(old: &str, fixed: &str) -> FixedCardName {
        FixedCardName {
            old: String::from(old),
            fixed: String::from(fixed),
        }
    }
}

impl Default for DivinationCardRecord {
    fn default() -> Self {
        Self {
            name: String::from("Rain Of Chaos"),
            price: Some(1.0),
            amount: 1,
            weight: None,
        }
    }
}

impl DivinationCard for DivinationCardRecord {
    fn is_card(&self) -> bool {
        CARDS.contains(&self.name.as_str())
    }

    fn is_legacy_card(&self) -> bool {
        LEGACY_CARDS.contains(&self.name.as_str())
    }
}

pub trait DivinationCard {
    fn is_card(&self) -> bool;
    fn is_legacy_card(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_card() {
        let record = DivinationCardRecord::new("Rain of Chaos", None, None);
        assert_eq!(record.is_card(), true);
    }

    #[test]
    fn is_legacy_card() {
        let record = DivinationCardRecord::new("Friendship", None, None);
        assert_eq!(record.is_legacy_card(), true);
    }

    #[tokio::test]
    async fn prices() {
        // let prices = DivinationCardsSample::fetch_prices().await.unwrap();
        // dbg!(prices);
    }
}

pub const LEGACY_CARDS: [&'static str; 12] = [
    "Friendship",
    "Squandered Prosperity",
    "Blessing of God",
    "The Devastator",
    "Luck of the Vaal",
    "The Valley of Steel Boxes",
    "Birth of the Three",
    "The Mayor",
    "Treasures of the Vaal",
    "The Bargain",
    "The Long Watch",
    "The Sustenance",
];

pub const CARDS_N: usize = 438;
pub const LEGACY_CARDS_N: usize = 12;
pub const CARDS: [&'static str; CARDS_N] = [
    "Brother's Gift",
    "Soul Quenched",
    "Poisoned Faith",
    "A Chilling Wind",
    "Matryoshka",
    "A Dusty Memory",
    "Alivia's Grace",
    "Auspicious Ambitions",
    "Divine Beauty",
    "Ever-Changing",
    "Man With Bear",
    "The Finishing Touch",
    "The Insane Cat",
    "The Return of the Rat",
    "The Wedding Gift",
    "A Dab of Ink",
    "A Familiar Call",
    "A Fate Worse Than Death",
    "A Modest Request",
    "A Mother's Parting Gift",
    "A Sea of Blue",
    "A Stone Perfected",
    "Abandoned Wealth",
    "Acclimatisation",
    "Alluring Bounty",
    "Alone in the Darkness",
    "Altered Perception",
    "Ambitious Obsession",
    "Anarchy's Price",
    "Arrogance of the Vaal",
    "Assassin's Favour",
    "Astral Protection",
    "Atziri's Arsenal",
    "Audacity",
    "Azure Rage",
    "Azyran's Reward",
    "Baited Expectations",
    "Beauty Through Death",
    "Bijoux",
    "Birth of the Three",
    "Blind Venture",
    "Boon of Justice",
    "Boon of the First Ones",
    "Boundless Realms",
    "Bowyer's Dream",
    "Broken Promises",
    "Broken Truce",
    "Brotherhood in Exile",
    "Brother's Stash",
    "Brush, Paint and Palette",
    "Buried Treasure",
    "Burning Blood",
    "Call to the First Ones",
    "Cameria's Cut",
    "Cartographer's Delight",
    "Chaotic Disposition",
    "Chasing Risk",
    "Checkmate",
    "Choking Guilt",
    "Costly Curio",
    "Council of Cats",
    "Coveted Possession",
    "Cursed Words",
    "Dark Dreams",
    "Dark Temptation",
    "Darker Half",
    "Deadly Joy",
    "Death",
    "Deathly Designs",
    "Dementophobia",
    "Demigod's Wager",
    "Desecrated Virtue",
    "Desperate Crusade",
    "Destined to Crumble",
    "Dialla's Subjugation",
    "Disdain",
    "Divine Justice",
    "Doedre's Madness",
    "Doryani's Epiphany",
    "Draped in Dreams",
    "Duality",
    "Dying Anguish",
    "Dying Light",
    "Earth Drinker",
    "Echoes of Love",
    "Emperor of Purity",
    "Emperor's Luck",
    "Endless Night",
    "Etched in Blood",
    "Eternal Bonds",
    "Fateful Meeting",
    "Forbidden Power",
    "From Bone to Ashes",
    "Further Invention",
    "Gemcutter's Mercy",
    "Gemcutter's Promise",
    "Gift of Asenath",
    "Gift of the Gemling Queen",
    "Glimmer of Hope",
    "Grave Knowledge",
    "Guardian's Challenge",
    "Harmony of Souls",
    "Haunting Shadows",
    "Her Mask",
    "Heterochromia",
    "Home",
    "Hope",
    "House of Mirrors",
    "Hubris",
    "Humility",
    "Hunter's Resolve",
    "Hunter's Reward",
    "Immortal Resolve",
    "Imperfect Memories",
    "Imperial Legacy",
    "Jack in the Box",
    "Judging Voices",
    "Justified Ambition",
    "Keeper's Corruption",
    "Lachrymal Necrosis",
    "Lantador's Lost Love",
    "Last Hope",
    "Left to Fate",
    "Lethean Temptation",
    "Light and Truth",
    "Lingering Remnants",
    "Lost Worlds",
    "Love Through Ice",
    "Loyalty",
    "Lucky Connections",
    "Lucky Deck",
    "Luminous Trove",
    "Lysah's Respite",
    "Magnum Opus",
    "Mawr Blaidd",
    "Merciless Armament",
    "Might is Right",
    "Misery in Darkness",
    "Mitts",
    "Monochrome",
    "More is Never Enough",
    "No Traces",
    "Nook's Crown",
    "Parasitic Passengers",
    "Peaceful Moments",
    "Perfection",
    "Prejudice",
    "Pride Before the Fall",
    "Pride of the First Ones",
    "Prometheus' Armoury",
    "Prosperity",
    "Rain of Chaos",
    "Rain Tempter",
    "Rats",
    "Rebirth",
    "Rebirth and Renewal",
    "Reckless Ambition",
    "Remembrance",
    "Sambodhi's Vow",
    "Sambodhi's Wisdom",
    "Scholar of the Seas",
    "Seven Years Bad Luck",
    "Shard of Fate",
    "Silence and Frost",
    "Society's Remorse",
    "Something Dark",
    "Struck by Lightning",
    "Succor of the Sinless",
    "Terrible Secret of Space",
    "The Academic",
    "The Admirer",
    "The Adventuring Spirit",
    "The Aesthete",
    "The Apothecary",
    "The Archmage's Right Hand",
    "The Arena Champion",
    "The Army of Blood",
    "The Artist",
    "The Aspirant",
    "The Astromancer",
    "The Avenger",
    "The Awakened",
    "The Bargain",
    "The Battle Born",
    "The Bear Woman",
    "The Beast",
    "The Betrayal",
    "The Bitter Blossom",
    "The Blazing Fire",
    "The Blessing of Moosh",
    "The Body",
    "The Bones",
    "The Brawny Battle Mage",
    "The Breach",
    "The Brittle Emperor",
    "The Cache",
    "The Cacophony",
    "The Calling",
    "The Card Sharp",
    "The Carrion Crow",
    "The Cartographer",
    "The Cataclysm",
    "The Catalyst",
    "The Catch",
    "The Celestial Justicar",
    "The Celestial Stone",
    "The Chains that Bind",
    "The Cheater",
    "The Chosen",
    "The Coming Storm",
    "The Conduit",
    "The Craving",
    "The Cursed King",
    "The Damned",
    "The Dapper Prodigy",
    "The Dark Mage",
    "The Darkest Dream",
    "The Deal",
    "The Deceiver",
    "The Deep Ones",
    "The Demon",
    "The Demoness",
    "The Destination",
    "The Doctor",
    "The Doppelganger",
    "The Dragon",
    "The Dragon's Heart",
    "The Dreamer",
    "The Dreamland",
    "The Drunken Aristocrat",
    "The Dungeon Master",
    "The Easy Stroll",
    "The Eldritch Decay",
    "The Emptiness",
    "The Encroaching Darkness",
    "The Endless Darkness",
    "The Endurance",
    "The Enforcer",
    "The Enlightened",
    "The Enthusiasts",
    "The Escape",
    "The Eternal War",
    "The Ethereal",
    "The Explorer",
    "The Eye of Terror",
    "The Eye of the Dragon",
    "The Fathomless Depths",
    "The Feast",
    "The Fiend",
    "The Fishmonger",
    "The Fletcher",
    "The Flora's Gift",
    "The Fool",
    "The Forgotten Treasure",
    "The Formless Sea",
    "The Forsaken",
    "The Forward Gaze",
    "The Fox",
    "The Fox in the Brambles",
    "The Gambler",
    "The Garish Power",
    "The Gemcutter",
    "The Gentleman",
    "The Gladiator",
    "The Golden Era",
    "The Greatest Intentions",
    "The Gulf",
    "The Hale Heart",
    "The Harvester",
    "The Hermit",
    "The Heroic Shot",
    "The Hive of Knowledge",
    "The Hoarder",
    "The Hook",
    "The Hunger",
    "The Immortal",
    "The Incantation",
    "The Innocent",
    "The Inoculated",
    "The Insatiable",
    "The Inventor",
    "The Jester",
    "The Jeweller's Boon",
    "The Journalist",
    "The Journey",
    "The King's Blade",
    "The King's Heart",
    "The Landing",
    "The Last One Standing",
    "The Last Supper",
    "The Leviathan",
    "The Lich",
    "The Life Thief",
    "The Lion",
    "The Long Con",
    "The Long Watch",
    "The Lord in Black",
    "The Lord of Celebration",
    "The Lover",
    "The Lunaris Priestess",
    "The Magma Crab",
    "The Master",
    "The Master Artisan",
    "The Mercenary",
    "The Messenger",
    "The Metalsmith's Gift",
    "The Mind's Eyes",
    "The Mountain",
    "The Nurse",
    "The Oath",
    "The Obscured",
    "The Offering",
    "The Offspring",
    "The Old Man",
    "The One That Got Away",
    "The One With All",
    "The Opulent",
    "The Pack Leader",
    "The Pact",
    "The Patient",
    "The Penitent",
    "The Poet",
    "The Polymath",
    "The Porcupine",
    "The Price of Devotion",
    "The Price of Loyalty",
    "The Price of Prescience",
    "The Price of Protection",
    "The Primordial",
    "The Prince of Darkness",
    "The Professor",
    "The Progeny of Lunaris",
    "The Puzzle",
    "The Queen",
    "The Rabbit's Foot",
    "The Rabid Rhoa",
    "The Realm",
    "The Risk",
    "The Rite of Elements",
    "The Road to Power",
    "The Ruthless Ceinture",
    "The Sacrifice",
    "The Saint's Treasure",
    "The Samurai's Eye",
    "The Scarred Meadow",
    "The Scavenger",
    "The Scholar",
    "The Scout",
    "The Seeker",
    "The Sephirot",
    "The Shepherd's Sandals",
    "The Shieldbearer",
    "The Shortcut",
    "The Side Quest",
    "The Sigil",
    "The Siren",
    "The Skeleton",
    "The Soul",
    "The Spark and the Flame",
    "The Spoiled Prince",
    "The Standoff",
    "The Stormcaller",
    "The Strategist",
    "The Summoner",
    "The Sun",
    "The Surgeon",
    "The Surveyor",
    "The Survivalist",
    "The Sustenance",
    "The Sword King's Salute",
    "The Thaumaturgist",
    "The Throne",
    "The Tinkerer's Table",
    "The Tireless Extractor",
    "The Tower",
    "The Traitor",
    "The Trial",
    "The Tumbleweed",
    "The Twilight Moon",
    "The Twins",
    "The Tyrant",
    "The Undaunted",
    "The Undisputed",
    "The Unexpected Prize",
    "The Union",
    "The Valkyrie",
    "The Vast",
    "The Visionary",
    "The Void",
    "The Warden",
    "The Warlord",
    "The Watcher",
    "The Web",
    "The White Knight",
    "The Whiteout",
    "The Wilted Rose",
    "The Wind",
    "The Witch",
    "The Wolf",
    "The Wolf's Legacy",
    "The Wolf's Shadow",
    "The Wolven King's Bite",
    "The Wolverine",
    "The World Eater",
    "The Wrath",
    "The Wretched",
    "Thirst for Knowledge",
    "Three Faces in the Dark",
    "Three Voices",
    "Thunderous Skies",
    "Time-Lost Relic",
    "Tranquillity",
    "Treasure Hunter",
    "Triskaidekaphobia",
    "Turn the Other Cheek",
    "Unchained",
    "Underground Forest",
    "Unrequited Love",
    "Vanity",
    "Vinia's Token",
    "Void of the Elements",
    "Volatile Power",
    "Wealth and Power",
    "Winter's Embrace",
    "Friendship",
    "Vile Power",
    "Squandered Prosperity",
    "Blessing of God",
    "The Devastator",
    "The Rusted Bard",
    "Luck of the Vaal",
    "A Note in the Wind",
    "The Valley of Steel Boxes",
    "Akil's Prophecy",
    "The Mayor",
    "The Transformation",
    "The Mad King",
    "Treasures of the Vaal",
];