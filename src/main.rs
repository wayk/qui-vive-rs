extern crate hyper;
extern crate futures;
extern crate regex;
extern crate rand;
extern crate mouscache;
#[macro_use]
extern crate mouscache_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

#[macro_use]
extern crate lazy_static;

use hyper::{Body, StatusCode, mime};
use hyper::Method::{Get, Post};
use hyper::header::{ContentType};
use hyper::server::{Request, Response, Service};

use futures::Future;
use futures::stream::{Stream};

use mouscache::{MemoryCache, RedisCache};

use rand::{thread_rng, Rng};
use regex::Regex;

use std::sync::{Arc, Mutex};

#[derive(Cacheable, Clone, Debug)]
struct QuiViveKey {
    key: String,
    value: String,
}

struct QuiVive {
    pub cache: Arc<Mutex<mouscache::Cache>>,
}

fn gen_id() -> Option<String> {
    const CHARSET: &[u8] = b"23456789\
    abcdefghjkimnpqrstuvwxyz\
    ABCDEFGHJKLMNPQRSTUVWXYZ";

    let mut rng = thread_rng();
    let id: Option<String> = (0..9)
        .map(|_| Some(*rng.choose(CHARSET)? as char))
        .collect();
    id
}

impl Service for QuiVive {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response<Body>, Error=hyper::Error>>;

    fn call(&self, request: Request) -> Self::Future {

        let url_prefix = "http://127.0.0.1:8080";

        lazy_static! {
            static ref RE_KEY: Regex = Regex::new(r"^/key$").unwrap();
            static ref RE_KEY_ID: Regex = Regex::new(r"^/key/(\w+)$").unwrap();
        }

        let method = request.method().clone();
        let path = request.path().clone().to_owned();

        match (method, path.as_str()) {
            (Post, ref x) if RE_KEY.is_match(x) => {
                let key = gen_id().unwrap();
                let cache = self.cache.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let entry = QuiViveKey { key: key.clone(), value: value };
                    let result = format!("{}/key/{}\n", url_prefix, key);

                    let mut cache_guard = cache.lock().unwrap();
                    if let Ok(_) = cache_guard.insert(key.clone(), entry.clone()) {
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                            .with_body(result)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                }))
            }
            (Post, ref x) if RE_KEY_ID.is_match(x) => {
                let cap = RE_KEY_ID.captures(x).unwrap();
                let key = cap[1].to_string();
                let cache = self.cache.clone();

                Box::new(request.body().concat2().map(move|body| {
                    let mut value = String::from_utf8(body.to_vec()).unwrap();

                    if !value.ends_with('\n') {
                        value.push('\n');
                    }

                    let entry = QuiViveKey { key: key.clone(), value: value };
                    let result = format!("{}/key/{}\n", url_prefix, key);

                    let mut cache_guard = cache.lock().unwrap();
                    if let Ok(_) = cache_guard.insert(key.clone(), entry.clone()) {
                        Response::new()
                            .with_status(StatusCode::Ok)
                            .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                            .with_body(result)
                    } else {
                        Response::new()
                            .with_status(StatusCode::NotFound)
                    }
                }))
            }
            (Get, ref x) if RE_KEY_ID.is_match(x) => {
                let cap = RE_KEY_ID.captures(x).unwrap();
                let key = cap[1].to_string();

                if let Some(entry) = self.cache.lock().unwrap().get::<String, QuiViveKey>(key.clone()) {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::Ok)
                        .with_header(ContentType(mime::TEXT_PLAIN_UTF_8))
                        .with_body(entry.value)
                    ))
                } else {
                    Box::new(futures::future::ok(Response::new()
                        .with_status(StatusCode::NotFound)))
                }
            }
            (Get, "/health") => {
                Box::new(futures::future::ok(Response::new()
                    .with_status(StatusCode::Ok)))
            }
            _ => {
                Box::new(futures::future::ok(Response::new()
                    .with_status(StatusCode::NotFound)))
            }
        }
    }
}

fn main() {
    env_logger::init();
    let address = "127.0.0.1:8080".parse().unwrap();
    let new_service = || {
        let cache = match RedisCache::new("localhost", None) {
            Ok(cache) => cache,
            Err(_) => MemoryCache::new()
        };
        Ok(QuiVive{ cache: Arc::new(Mutex::new(cache)) } )
    };
    let server = hyper::server::Http::new()
        .bind(&address, new_service)
        .unwrap();
    info!("running qui-vive at {}", address);
    server.run().unwrap();
}
