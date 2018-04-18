#![feature(try_from)]

extern crate tera;

#[macro_use]
extern crate serde_derive;

extern crate iron;
extern crate iron_tera;
extern crate router;
extern crate serde;
extern crate serde_json;

use iron::prelude::*;
use iron::status;
use router::Router;

use iron_tera::{Template, TemplateMode, TeraEngine};
use std::convert::TryInto;
use std::error::Error;
use std::fmt::{self, Debug, Display};
use tera::Context;

#[derive(Serialize)]
struct User<'a> {
    username: &'a str,
    my_var: &'a str,
    numbers: &'a [u32],
    bio: &'a str,
}

fn main() {
    let mut router = Router::new();
    router.get("/user", user_handler, "user");
    router.get("/usertest", produce_handler, "usertest");

    let mut chain = Chain::new(router);
    let teng = TeraEngine::new("src/examples/templates/**/*");
    chain.link_after(teng);

    Iron::new(chain).http("localhost:5000").unwrap();
}

fn user_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();

    let mut context = Context::new();
    context.add("username", &"Bob");
    context.add("my_var", &"Thing"); // comment out to see alternate thing
    context.add("numbers", &vec![1, 2, 3]);
    context.add("bio", &"<script>alert('pwnd');</script>");

    resp.set_mut(Template::new("users/profile.html", context))
        .set_mut(status::Ok);
    Ok(resp)
}

// this uses the unstable feature on nightly
fn produce_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();

    let user = User {
        username: "Bob",
        my_var: "Thing",
        numbers: &vec![1, 2, 3],
        bio: "<script>alert('pwnd');</script>",
    };
    match user.try_into() {
        Ok(u) => {
            resp.set_mut(Template::new("users/profile.html", u))
                .set_mut(status::Ok);
            Ok(resp)
        }
        Err(_) => Err(IronError::new(
            StringError("Serialization error".to_string()),
            status::BadRequest,
        )),
    }
}

#[derive(Debug)]
struct StringError(String);

impl Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for StringError {
    fn description(&self) -> &str {
        &*self.0
    }
}
