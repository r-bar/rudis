#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate redis;
extern crate r2d2_redis;
extern crate r2d2;

use r2d2_redis::RedisConnectionManager;
use rocket::{http, request, Outcome, State};
use redis::Commands;
use std::ops::Deref;

pub struct RedisConnectionPool(r2d2::PooledConnection<RedisConnectionManager>);

impl<'a, 'r> request::FromRequest<'a, 'r> for RedisConnectionPool {
    type Error = ();

    fn from_request(request: &'a request::Request<'r>) -> request::Outcome<RedisConnectionPool, ()> {
        let pool = request.guard::<State<r2d2::Pool<RedisConnectionManager>>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(RedisConnectionPool(conn)),
            Err(_) => Outcome::Failure((http::Status::ServiceUnavailable, ())),
        }
    }
}

impl Deref for RedisConnectionPool {
    type Target = redis::Connection;

    fn deref(&self) -> &redis::Connection {
        &self.0.deref()
    }
}

fn init_redis_pool(url: &str) -> r2d2::Pool<RedisConnectionManager> {
    let manager = RedisConnectionManager::new(url).expect("Could not connect to Redis.");
    r2d2::Pool::new(manager).expect("Could not initialize connection pool.")
}

#[get("/")]
fn index(conn: RedisConnectionPool) -> Result<String, ()> {
    let _: () = redis::cmd("INCR").arg("counter").query(&*conn).map_err(|_| ())?;
    let counter: i32 = (*conn).get("counter").map_err(|_| ())?;
    Ok(format!("Counter: {}", counter))
}

fn main() {
    let redis_pool = init_redis_pool("redis://localhost:6379");
    rocket::ignite()
        .manage(redis_pool)
        .mount("/", routes![index]).launch();
}
