use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use chrono::{serde::ts_seconds_option, DateTime, Datelike, NaiveTime, Utc, Weekday};
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};

static FLIGHT_URL: &str = "";

#[derive(Debug, Deserialize)]
pub struct FlightNumberData {
    default: String,
    alternative: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Identification {
    number: FlightNumberData,
    callsign: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Model {
    code: String,
    text: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Aircraft {
    model: Model,
    registration: Option<String>,
    hex: Option<String>,
    #[serde(rename = "serialNo")]
    serial_no: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Code {
    iata: String,
    icao: String,
}
#[derive(Debug, Deserialize)]
pub struct Airline {
    name: String,
    code: Code,
}

#[derive(Debug, Deserialize)]
pub struct Airport {
    name: String,
    code: Code,
}
#[derive(Debug, Deserialize)]
pub struct Airports {
    origin: Airport,
    destination: Airport,
}

#[derive(Debug, Deserialize)]
pub struct Times {
    #[serde(with = "ts_seconds_option")]
    departure: Option<DateTime<Utc>>,
    #[serde(with = "ts_seconds_option")]
    arrival: Option<DateTime<Utc>>,
}
#[derive(Debug, Deserialize)]
pub struct TimeData {
    scheduled: Times,
    real: Times,
    estimated: Times,
}

#[derive(Debug, Deserialize)]
pub struct Flight {
    aircraft: Aircraft,
    airline: Option<Airline>,
    airport: Airports,
    identification: Identification,
    time: TimeData,
}

#[derive(Debug, Deserialize)]
pub struct PageData {
    current: u64,
}
#[derive(Debug, Deserialize)]
pub struct FlightApiResponse {
    data: Option<Vec<Flight>>,
    page: PageData,
}
#[derive(Debug, Deserialize)]
pub struct FlightApiResult {
    response: FlightApiResponse,
}
#[derive(Debug, Deserialize)]
pub struct Fr24FlightApiResponse {
    result: FlightApiResult,
}

pub async fn history_by_flight_number(token: &str, flight_number: &str) -> Result<Vec<Flight>> {
    reqwest::Client::new()
        .get(format!(
            "https://api.flightradar24.com/common/v1/flight/list.json?query={}&fetchBy=flight&page={}&limit=25&token={}",
             flight_number, 1, token
        ))
        .header("Origin", "https://www.flightradar24.com")
        .header("Referer", "https://www.flightradar24.com")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 6.1; WOW64; rv:28.0) Gecko/20100101 Firefox/28.0",
        )
        .send()
        .map_err(|e| anyhow!(e))
        .await?
        .json::<Fr24FlightApiResponse>()
        .await.map(|r|
            r.result.response.data.unwrap_or(vec![])
        )
        .map_err(|e| anyhow!(e))
}

type FlightNumber = String;
type OriginDestPairMap = HashMap<String, FlightInfo>;
pub type FlightNoMap = HashMap<FlightNumber, OriginDestPairMap>;
#[derive(Debug, Serialize)]
pub struct FlightInfo {
    origin: String,
    dest: String,
    scheduled_departure: HashSet<NaiveTime>,
    scheduled_arrival: HashSet<NaiveTime>,
    weekday: HashSet<Weekday>,
    model: HashSet<String>,
    callsign: HashSet<String>,
}
impl From<Flight> for FlightInfo {
    fn from(f: Flight) -> Self {
        FlightInfo {
            origin: f.airport.origin.code.icao,
            dest: f.airport.destination.code.icao,
            weekday: match f.time.scheduled.departure {
                Some(dt) => HashSet::from([dt.weekday()]),
                None => HashSet::new(),
            },
            scheduled_departure: match f.time.scheduled.departure {
                Some(dt) => HashSet::from([dt.time()]),
                None => HashSet::new(),
            },
            scheduled_arrival: match f.time.scheduled.arrival {
                Some(dt) => HashSet::from([dt.time()]),
                None => HashSet::new(),
            },
            model: HashSet::from([f.aircraft.model.code]),
            callsign: match f.identification.callsign {
                Some(cs) => HashSet::from([cs]),
                None => HashSet::new(),
            },
        }
    }
}
pub fn consolidate_flight_info(flights: Vec<Flight>) -> FlightNoMap {
    flights.into_iter().fold(HashMap::new(), |mut acc, f| {
        let flight_no = f.identification.number.default.clone();
        match acc.get_mut(&flight_no) {
            Some(odpm) => {
                let origin = &f.airport.origin.code.icao;
                let destination = &f.airport.destination.code.icao;
                match odpm.get_mut(&format!("{origin}-{destination}")) {
                    Some(fi) => {
                        fi.model.insert(f.aircraft.model.code);
                        f.time.scheduled.departure.map(|dt| {
                            fi.weekday.insert(dt.weekday());
                            fi.scheduled_departure.insert(dt.time())
                        });
                        f.time
                            .scheduled
                            .arrival
                            .map(|dt| fi.scheduled_arrival.insert(dt.time()));
                        f.identification.callsign.map(|cs| fi.callsign.insert(cs));
                    }
                    None => {
                        odpm.insert(format!("{origin}-{destination}"), FlightInfo::from(f));
                    }
                }
            }
            None => {
                let origin = &f.airport.origin.code.icao;
                let destination = &f.airport.destination.code.icao;
                let odpm =
                    HashMap::from([(format!("{origin}-{destination}"), FlightInfo::from(f))]);
                acc.insert(flight_no.clone(), odpm);
            }
        }
        acc
    })
}
