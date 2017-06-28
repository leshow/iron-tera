//! Please contact me on github and file any issues if you find some, I'm also open to PRs or other suggestions.
//!
//! Updated to Tera 0.10 / Serde 1.0 / Iron 0.5 !
//! Serde 0.9.0 to_value returns a `Result`, this means you need to handle the possiblity of a serialization failure.
//! If you just want to `unwrap()` there is an implementation of From<Value> for TemplateMode so `Template::new(path, value)`
//! works also.
//!
//! **update iron-tera-0.4.0**: If you build this crate with feature = "unstable" on the nightly compiler,
//! I've included a TryFrom impl so that you can use try without having to unwrap. Instead of
//! `TemplateMode::from_serial` for `serde_json::Value`s use `TemplateMode::try_serialize`.
//! `try_serialize` returns a `Result`, allowing you to explicitly handle or propagate up.
//!
//!
//! ## Examples
//! The following is a complete working example that I've tested with serde 0.9.0, tera 0.7.0 and iron-tera 0.2.0
//!
//! ```ignore
//! extern crate tera;
//! extern crate iron;
//! extern crate router;
//! #[macro_use] extern crate serde_json;
//! extern crate iron_tera;
//! fn main() {
//!     use iron::prelude::*;
//!     use iron::status;
//!     use router::Router;
//!     use tera::Context;
//!
//!     use iron_tera::{Template, TeraEngine}; // import TemplateMode to explicitly handle serialization errors
//!
//!     let mut router = Router::new();
//!     router.get("/context", context_handler, "context");
//!     router.get("/json", json_handler, "json");
//!
//!     let mut chain = Chain::new(router);
//!     let teng = TeraEngine::new("src/examples/templates/**/*");
//!     chain.link_after(teng);
//!
//!     Iron::new(chain).http("localhost:5000").unwrap();
//!
//!
//!     fn context_handler(_: &mut Request) -> IronResult<Response> {
//!         let mut resp = Response::new();
//!
//!         let mut context = Context::new();
//!         context.add("username", &"Bob");
//!         context.add("my_var", &"Thing"); // comment out to see alternate thing
//!         context.add("numbers", &vec![1, 2, 3]);
//!         context.add("bio", &"<script>alert('pwnd');</script>");
//!
//!         // can use Template::new(path, TemplateMode::from_context(context)) or TemplateMode::from(context) also
//!         resp.set_mut(Template::new("users/profile.html", context))
//!             .set_mut(status::Ok);
//!         Ok(resp)
//!     }
//!     fn json_handler(_: &mut Request) -> IronResult<Response> {
//!         let mut resp = Response::new();
//!
//!         let blob = json!({
//!             "username": "John Doe",
//!             "my_var": "Thing",
//!             "numbers": [
//!                 "1",
//!                 "+44 2345678",
//!                 "3"
//!             ],
//!             "bio": "<script>alert('pwnd');</script>"
//!          });
//!         // you can use Template::new(path, TemplateMode::from_serial(serde_json::Value)) to handle
//!         // serialization error explicitly
//!         resp.set_mut(Template::new("users/profile.html", blob))
//!             .set_mut(status::Ok);
//!         Ok(resp)
//!     }
//! }
//! ```
//!
//! Creating a template from a struct
//!
//! ```ignore
//! // The following uses serde's Serialize
//! #[derive(Serialize)]
//! struct Product {
//!     name: String,
//!     value: i32,
//! }
//! // Rendering from a struct that implements Serialize
//! fn produce_handler(_: &mut Request) -> IronResult<Response> {
//!     let mut resp = Response::new();
//!
//!     // Using serialized values
//!     let product = Product {
//!         name: "Foo".into(),
//!         value: 42,
//!     };
//!     // You can use TemplateMode::from()
//!     resp.set_mut(Template::new("product.html", product))
//!         .set_mut(status::Ok);
//!     Ok(resp)
//! }
//! ```
#![cfg_attr(feature = "unstable", feature(try_from))]

#![allow(dead_code)]
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate tera;
extern crate iron;
extern crate serde;
extern crate plugin;

use iron::{AfterMiddleware, status, typemap};
use iron::headers::ContentType;
use iron::modifier::Modifier;
use iron::prelude::*;

use plugin::Plugin;

use serde::ser::Serialize;
use serde_json::{Value, to_value};
use std::convert::From;

#[cfg(feature = "unstable")]
use std::convert::TryFrom;

use tera::{Context, Tera};

/// There are 2 main ways to pass data to generate a template.
#[derive(Clone)]
pub enum TemplateMode {
    /// TeraContext constructor takes a `Context`
    TeraContext(Context),
    /// Serialized constructor takes a `Value` from `serde_json`
    Serialized(Value),
}

/// TemplateMode should only ever be created from these smart constructors,
/// not with the enums type constructors.
impl TemplateMode {
    pub fn from_context(context: Context) -> TemplateMode {
        TemplateMode::TeraContext(context)
    }

    #[cfg(feature = "unstable")]
    pub fn try_serialize<S: Serialize>(
        serializeable: S,
    ) -> Result<TemplateMode, serde_json::Error> {
        Ok(TemplateMode::try_from(serializeable)?)
    }

    #[cfg(not(feature = "unstable"))]
    pub fn from_serial<S: Serialize>(serializeable: S) -> Result<TemplateMode, serde_json::Error> {
        Ok(TemplateMode::Serialized(to_value(serializeable)?))
    }
}

impl From<Context> for TemplateMode {
    fn from(context: Context) -> Self {
        TemplateMode::from_context(context)
    }
}

#[cfg(not(feature = "unstable"))]
impl From<Value> for TemplateMode {
    fn from(serializeable: Value) -> Self {
        TemplateMode::from_serial(serializeable).unwrap()
    }
}

#[cfg(feature = "unstable")]
impl<S> TryFrom<S> for TemplateMode
where
    S: Serialize,
{
    type Error = serde_json::Error;
    fn try_from(value: S) -> Result<Self, Self::Error> {
        from_serde(value)
    }
}

#[cfg(feature = "unstable")]
fn from_serde<S: Serialize>(serializeable: S) -> Result<TemplateMode, serde_json::Error> {
    Ok(TemplateMode::Serialized(to_value(serializeable)?))
}

/// Our template holds a name (path to template) and a mode (constructed with `from_context` or `from_serial`)
#[derive(Clone)]
pub struct Template {
    mode: TemplateMode,
    name: String,
}

impl Template {
    pub fn new<T: Into<TemplateMode>, S: Into<String>>(name: S, mode: T) -> Template {
        Template {
            name: name.into(),
            mode: mode.into(),
        }
    }
}

/// TeraEngine holds the Tera struct so that it can be used by many handlers without explicitly passing
pub struct TeraEngine {
    pub tera: Tera,
}

/// `compile_templates!` is used to parse the contents of a dir for all templates.
impl TeraEngine {
    /// Take a `String` and convert to a slice
    pub fn new<S: AsRef<str>>(dir: S) -> TeraEngine {
        TeraEngine { tera: compile_templates!(dir.as_ref()) }
    }
}

impl typemap::Key for TeraEngine {
    type Value = Template;
}

impl Modifier<Response> for Template {
    fn modify(self, resp: &mut Response) {
        resp.extensions.insert::<TeraEngine>(self);
    }
}

/// The middleware implementation for TeraEngine
impl AfterMiddleware for TeraEngine {
    /// This is where all the magic happens. We extract `TeraEngine` from Iron's `Response`,
    /// determine what `TemplateMode` we should render in, and pass the appropriate values to
    /// tera's render methods.
    fn after(&self, _: &mut Request, mut resp: Response) -> IronResult<Response> {
        let wrapper = resp.extensions.remove::<TeraEngine>().and_then(
            |t| match t.mode {
                TemplateMode::TeraContext(ref context) => Some(self.tera.render(&t.name, context)),
                TemplateMode::Serialized(ref value) => Some(self.tera.render(&t.name, value)),
            },
        );
        match wrapper {
            Some(result) => {
                result
                    .map_err(|e| IronError::new(e, status::InternalServerError))
                    .and_then(|page| {
                        if !resp.headers.has::<ContentType>() {
                            resp.headers.set(ContentType::html());
                        }
                        resp.set_mut(page);
                        Ok(resp)
                    })
            }
            None => Ok(resp),
        }
    }

    fn catch(&self, req: &mut Request, mut err: IronError) -> IronResult<Response> {
        err.response = self.after(req, err.response)?;
        Err(err)
    }
}

impl Plugin<Response> for TeraEngine {
    type Error = ();

    fn eval(resp: &mut Response) -> Result<Template, ()> {
        match resp.extensions.get::<TeraEngine>() {
            Some(t) => Ok(t.clone()),
            None => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Template, TemplateMode, TeraEngine};
    use iron::prelude::*;
    use tera::Context;

    fn from_context_response() -> IronResult<Response> {
        let resp = Response::new();
        let mut context = Context::new();
        context.add("greeting", &"hi!");
        Ok(resp.set(
            Template::new("./test_template/users/foo.html", context),
        ))
    }

    #[test]
    fn test_from_context() {
        let mut resp = from_context_response().ok().expect("response expected");
        match resp.get::<TeraEngine>() {
            Ok(h) => {
                assert_eq!(h.name, "./test_template/users/foo.html".to_string());
                if let TemplateMode::TeraContext(context) = h.mode {
                    assert_eq!(
                        context
                            .as_json()
                            .unwrap()
                            .get("greeting")
                            .unwrap()
                            .as_str()
                            .unwrap(),
                        "hi!"
                    );
                } else {
                    panic!("TeraContext expected");
                }
            }
            _ => panic!("template expected"),
        }
    }
}
