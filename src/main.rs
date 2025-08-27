use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use clap::Parser;

#[derive(Parser)]
struct Opt {
    cards: PathBuf,
    config: PathBuf,
    out: PathBuf,
    printings: Vec<PathBuf>,
}

#[derive(serde::Deserialize)]
struct SideConfig {
    side: String,
    excludes: BTreeSet<String>,
    rares: BTreeSet<String>,
}

// { id: 'c_rc_01', name: 'Sprint', image: 'https://picsum.photos/seed/sprint/400/560', props: { color: 'Red', type: 'Action', rarity: 'Common' } },
#[derive(serde::Serialize)]
struct DraftCard {
    id: String,
    name: String,
    image: String,
    props: BTreeMap<String, String>,
}

fn main() {
    let opt = Opt::parse();
    let sideconfigstr = std::fs::read_to_string(opt.config).unwrap();
    let sideconfig: SideConfig = toml::from_str(&sideconfigstr).unwrap();

    let mut cardpool = BTreeSet::new();
    let mut pcmap = BTreeMap::new();

    for printing in opt.printings {
        let printings: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(printing).unwrap()).unwrap();

        let printings = printings.as_array().unwrap();

        for printing in printings {
            let card_id = printing["card_id"].as_str().unwrap().to_string();
            let printing_id = printing["id"].as_str().unwrap().to_string();
            cardpool.insert(card_id.clone());
            pcmap.insert(card_id, printing_id);
        }
    }

    for exclude in &sideconfig.excludes {
        if !cardpool.contains(exclude) {
            println!("Warning: exclude {exclude} not in cardpool");
            return;
        }
    }
    for rare in &sideconfig.rares {
        if !cardpool.contains(rare) {
            println!("Warning: rare {rare} not in cardpool");
            return;
        }
    }

    let mut draft_cards = Vec::<DraftCard>::new();
    for card in cardpool {
        if sideconfig.excludes.contains(&card) {
            continue;
        }
        let fpath = opt.cards.join(format!("{card}.json"));
        let card_data: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(fpath).unwrap()).unwrap();
        let side_id = card_data["side_id"].as_str().unwrap();
        if side_id != sideconfig.side {
            continue;
        }
        let card_type = card_data["card_type_id"].as_str().unwrap().to_string();
        if card_type == "runner_identity" || card_type == "corp_identity" {
            continue;
        }
        let faction = card_data["faction_id"].as_str().unwrap().to_string();
        let name = card_data["stripped_title"].as_str().unwrap().to_string();
        let rarity = match &card_type[..] {
            "agenda" => "agenda".to_string(),
            _ => {
                if sideconfig.rares.contains(&card) {
                    "rare".to_string()
                } else {
                    "common".to_string()
                }
            }
        };
        let image = format!(
            "https://nro-public.s3.nl-ams.scw.cloud/nro/card-printings/v2/webp/english/card/{}.webp",
            pcmap[&card]
        );
        let card = DraftCard {
            id: card,
            name,
            image,
            props: BTreeMap::from([
                ("faction".to_string(), faction),
                ("type".to_string(), card_type),
                ("rarity".to_string(), rarity),
            ]),
        };
        draft_cards.push(card);
    }

    let out = serde_json::to_string(&draft_cards)
        .unwrap()
        .replace(r#"\""#, r#"\\""#);
    std::fs::write(opt.out, out).unwrap();
}
