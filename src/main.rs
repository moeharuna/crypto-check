use clap::{Parser, Subcommand};
use core::fmt;
use reqwest::blocking as req;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display};
#[derive(Parser)]
#[clap(version, about)]
struct Args {
    #[clap(default_value = "bitcoin")]
    crypto: String,
    #[clap(default_value = "usd")]
    target_currency: String,
    #[clap(subcommand)]
    command: Option<SubCommands>,
}
#[derive(Subcommand)]
enum SubCommands {
    CryptoList,
    TargetList,
}

const URL: &str = "https://api.coingecko.com/api/v3";

//Types for json derserilizing
#[derive(Deserialize, Debug, PartialEq, PartialOrd, Clone)]
struct Crypto {
    id: String,
}

#[derive(Deserialize, Debug, PartialEq, PartialOrd, Clone)]
struct Currency(String);

struct ResultType {
    current_price: f64,
    vol_24h: f64,
    change_24h: f64,
}

impl ResultType {
    fn new(current_price: f64, vol_24h: f64, change_24h: f64) -> ResultType {
        ResultType {
            current_price,
            vol_24h,
            change_24h,
        }
    }
}

#[derive(Debug)]
enum PriceError {
    NoSuchId(Crypto),
    NoSuchTargetCurrency(Currency),
}

impl Display for PriceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for PriceError {}

fn get_crypto_ids(client: &req::Client) -> Result<Vec<Crypto>, Box<dyn Error>> {
    let endpoint = URL.to_string() + "/coins/list";
    Ok(client.get(endpoint).send()?.json()?)
}

fn get_target_currencies(client: &req::Client) -> Result<Vec<Currency>, Box<dyn Error>> {
    let endpoint = URL.to_string() + "/simple/supported_vs_currencies";
    Ok(client.get(endpoint).send()?.json()?)
}

fn binary_search<T>(array: &[T], value: &T) -> bool
where
    T: PartialEq + PartialOrd,
{
    if array.len() == 0 {
        return false;
    }
    let middle = array.len() / 2;
    if &array[middle] == value {
        return true;
    } else if value <= &array[middle] {
        return binary_search(&array[..middle - 1], value);
    } else {
        return binary_search(&array[middle + 1..], value);
    }
}

fn check_id(client: &req::Client, crypto: &str) -> Result<(), Box<dyn Error>> {
    let check_list = get_crypto_ids(client)?;
    let crypto = Crypto {
        id: crypto.to_string(),
    };
    if !binary_search(&check_list, &crypto) {
        return Err(Box::new(PriceError::NoSuchId(crypto)));
    }
    return Ok(());
}

fn check_target(client: &req::Client, target: &str) -> Result<(), Box<dyn Error>> {
    let check_list = get_target_currencies(client)?;
    let target = Currency(target.to_string());

    //input not sorted so not binary search
    if !check_list.iter().any(|curr| &target == curr) {
        return Err(Box::new(PriceError::NoSuchTargetCurrency(target)));
    }
    return Ok(());
}

fn get_price(client: &req::Client, id: &str, target: &str) -> Result<ResultType, Box<dyn Error>> {
    let endpoint = URL.to_string() + "/simple/price";
    let json: HashMap<String, HashMap<String, f64>> = client
        .get(endpoint)
        .query(&[
            ("ids", id),
            ("vs_currencies", target),
            ("include_24hr_vol", "true"),
            ("include_24hr_change", "true"),
        ])
        .send()?
        .json()?;

    //ugly json parsing ahead
    let id = id.to_string();
    let price_str = target.to_string();
    let vol_str = target.to_string() + "_24h_vol";
    let change_str = target.to_string() + "_24h_change";

    let price = json[&id][&price_str];
    let vol = json[&id][&vol_str];
    let change = json[&id][&change_str];

    Ok(ResultType::new(price, vol, change))
}

fn main() {
    let args = Args::parse();
    let http_client = reqwest::blocking::Client::new();

    if let Some(c) = args.command {
        match c {
            SubCommands::CryptoList => {
                let cryptos = get_crypto_ids(&http_client).unwrap();
                for c in cryptos {
                    println!("{}", c.id);
                }
            }
            SubCommands::TargetList => {
                let targets = get_target_currencies(&http_client).unwrap();
                for t in targets {
                    println!("{}", t.0);
                }
            }
        }
        return ();
    }
    let crypto = args.crypto;
    let currency = args.target_currency;

    check_id(&http_client, &crypto).unwrap(); //TODO: Error handlig
    check_target(&http_client, &currency).unwrap();

    let result = get_price(&http_client, &crypto, &currency).unwrap();

    let price_str = format!("{}", result.current_price);
    let vol_str = format!("{:.0}", result.vol_24h);
    let change_str = format!("{:.2}", result.change_24h);

    println!(
        "{:p_w$} | {:vol_w$} | {:c_w$}",
        "Price",
        "24h_vol",
        "24h_change",
        p_w = price_str.len(),
        vol_w = vol_str.len(),
        c_w = change_str.len()
    );
    println!("{} | {} | {}", price_str, vol_str, change_str);
}
