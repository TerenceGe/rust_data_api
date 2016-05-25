extern crate iron;
extern crate redis;
extern crate time;
extern crate router;
extern crate rustc_serialize;
extern crate params;

use std::error::Error;
use rustc_serialize::json;
use iron::prelude::*;
use iron::{BeforeMiddleware, AfterMiddleware, typemap};
use iron::mime::Mime;
use redis::Commands;
use router::Router;
use time::precise_time_ns;
use params::{Params, Value};

struct ResponseTime;

impl typemap::Key for ResponseTime { type Value = u64; }

impl BeforeMiddleware for ResponseTime {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<ResponseTime>(precise_time_ns());
        Ok(())
    }
}

impl AfterMiddleware for ResponseTime {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = precise_time_ns() - *req.extensions.get::<ResponseTime>().unwrap();
        println!("Request took: {} ms", (delta as f64) / 1000000.0);
        Ok(res)
    }
}

fn get_ticker_from_redis(ticker : &mut String, market : &str) -> redis::RedisResult<()> {
    let client = try!(redis::Client::open("redis://127.0.0.1:6379"));
    let con = try!(client.get_connection());
    match market {
        "btccny" | "cnybtc" => {
            *ticker = try!(con.get("ticker_btccny"));
        },
        "ltccny" | "cnyltc" => {
            *ticker = try!(con.get("ticker_ltccny"));
        },
        "btcltc" | "ltcbtc" => {
            *ticker = try!(con.get("ticker_ltcbtc"));
        },
        "all" => {
            let ticker_btccny : String = try!(con.get("ticker_btccny"));
            let ticker_ltccny : String = try!(con.get("ticker_ltccny"));
            let ticker_ltcbtc : String = try!(con.get("ticker_ltcbtc"));
            *ticker = format!("{}{},{}{},{}{}{}",
                ticker_btccny.replace("ticker", "ticker_btccny").trim_right_matches('}'), "}",
                ticker_ltccny.replace("ticker", "ticker_ltccny").trim_right_matches('}').trim_left_matches('{'), "}",
                ticker_ltcbtc.replace("ticker", "ticker_ltcbtc").trim_right_matches('}').trim_left_matches('{'), "}", "}");
        },
        _ => println!("Invalid market!"),
    }
    Ok(())
}

fn fetch_ticker(req: &mut Request) -> IronResult<Response> {
    let map = req.get_ref::<Params>().unwrap();
    match map.find(&["market"]) {
        Some(&Value::String(ref market)) if market.len() > 0 => {
            let mut ticker = String::new();
            match get_ticker_from_redis(&mut ticker, &market.to_string()) {
                Err(err) => {
                    println!("Could not get ticker:");
                    println!("  {}: {}", err.category(), err.description());
                },
                Ok(()) => {
                    println!("Get ticker success: {}", ticker);
                },
            }
            if ticker.is_empty() {
                return Ok(Response::with(iron::status::Ok));
            }
            let json = json::Json::from_str(&ticker).unwrap();
            let payload = json::encode(&json).unwrap();
            let content_type = "application/json".parse::<Mime>().unwrap();
            Ok(Response::with((content_type, iron::status::Ok, payload)))
       },
       _ => Ok(Response::with(iron::status::Ok)),
   }
}

fn main() {
    let mut router = Router::new();
    router.get("/ticker", move |r: &mut Request| fetch_ticker(r));

    Iron::new(router).http("localhost:3001").unwrap();
}
