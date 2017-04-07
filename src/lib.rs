//! Please contact me on github and file any issues if you find some, I'm also open to PRs or other suggestions.
//! CHANGELOG (0.2.0):
//! If you're upgrading from iron-tera 0.1.4, the API has changed slightly for serialized values.
//! Serde 0.9.0 to_value returns a `Result`, this means you need to handle the possiblity of a serialization failure.
//! Since we're still pre-1.0 and the API isn't completely stable, I've only incremented the minor version number.
//!
//! ## Examples
//! The following is a complete working example that I've tested with serde 0.9.0, tera 0.7.0 and iron-tera 0.2.0
//! ```rust
//! #[macro_use]
//! extern crate tera;
//!
//! extern crate iron;
//! extern crate router;
//! #[macro_use]
//! extern crate serde_json;
//! extern crate iron_tera;
//!
//! use iron::prelude::*;
//! use iron::status;
//! use router::Router;
//! use tera::Context;
//!
//! use iron_tera::{Template, TeraEngine, TemplateMode};
//!
//! fn main() {
//!     let mut router = Router::new();
//!     router.get("/context", context_handler, "context");
//!     router.get("/json", json_handler, "json");
//!
//!     let mut chain = Chain::new(router);
//!     let teng = TeraEngine::new("src/examples/templates/**/*");
//!     chain.link_after(teng);
//!
//!     Iron::new(chain).http("localhost:5000").unwrap();
//! }
//!
//!
//! fn context_handler(_: &mut Request) -> IronResult<Response> {
//!     let mut resp = Response::new();
//!
//!     let mut context = Context::new();
//!     context.add("username", &"Bob");
//!     context.add("my_var", &"Thing"); // comment out to see alternate thing
//!     context.add("numbers", &vec![1, 2, 3]);
//!     context.add("bio", &"<script>alert('pwnd');</script>");
//!
//!     resp.set_mut(Template::new("users/profile.html", TemplateMode::from_context(context)))
//!         .set_mut(status::Ok);
//!     Ok(resp)
//! }
//! fn json_handler(_: &mut Request) -> IronResult<Response> {
//!     let mut resp = Response::new();
//!
//!     let blob = json!({
//!         "username": "John Doe",
//!         "my_var": "Thing",
//!         "numbers": [
//!             "1",
//!             "+44 2345678",
//!             "3"
//!         ],
//!         "bio": "<script>alert('pwnd');</script>"
//!      });
//!
//!     resp.set_mut(Template::new("users/profile.html",
//!                                TemplateMode::from_serial(&blob).unwrap()))
//!         .set_mut(status::Ok);
//!     Ok(resp)
//! }
//! ```
//! Creating a template from a struct
//! ```rust
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
//!     resp.set_mut(Template::new("product.html", TemplateMode::from_serial(&product).unwrap())) // I made a choice here to return a result and let you handle the serialization error how you see fit.
//!         .set_mut(status::Ok);
//!     Ok(resp)
//! }
//! ```
#![allow(dead_code)]
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate tera;
extern crate iron;
extern crate serde;

use iron::{AfterMiddleware, status, typemap};
use iron::headers::ContentType;
use iron::modifier::Modifier;
use iron::prelude::*;

use serde::ser::Serialize;
use serde_json::{Value, to_value};

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
    pub fn from_serial<S: Serialize>(serialized: S) -> Result<TemplateMode, serde_json::Error> {
        Ok(TemplateMode::Serialized(to_value(serialized)?))
    }
}

/// Our template holds a name (path to template) and a mode (constructed with `from_context` or `from_serial`)
#[derive(Clone)]
pub struct Template {
    mode: TemplateMode,
    name: String,
}

impl Template {
    pub fn new(name: &str, mode: TemplateMode) -> Template {
        Template {
            name: name.into(),
            mode: mode,
        }
    }
}

/// TeraEngine holds the Tera struct so that it can be used by many handlers without explicitly passing
pub struct TeraEngine {
    pub tera: Tera,
}

/// `compile_templates!` is used to parse the contents of a dir for all templates.
impl TeraEngine {
    /// Take a `String` and convert to a slice or preferably a `&str`
    pub fn new(dir: &str) -> TeraEngine {
        TeraEngine { tera: compile_templates!(dir) }
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
        let wrapper =
            resp.extensions
                .remove::<TeraEngine>()
                .and_then(
                    |t| match t.mode {
                        TemplateMode::TeraContext(context) => {
                            Some(self.tera.render(&t.name, context))
                        }
                        TemplateMode::Serialized(value) => {
                            Some(self.tera.value_render(&t.name, &value))
                        }
                    },
                );
        match wrapper {
            Some(result) => {
                result
                    .map_err(|e| IronError::new(e, status::InternalServerError))
                    .and_then(
                        |page| {
                            if !resp.headers.has::<ContentType>() {
                                resp.headers.set(ContentType::html());
                            }
                            resp.set_mut(page);
                            Ok(resp)
                        },
                    )
            }
            None => Ok(resp),
        }
    }

    fn catch(&self, req: &mut Request, mut err: IronError) -> IronResult<Response> {
        err.response = self.after(req, err.response)?;
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::{Template, TemplateMode, TeraEngine};
    use iron::prelude::*;
    use tera::{Context, Tera};
    //
    // #[derive(Serialize)]
    // struct Product {
    //     name: String,
    //     value: i32,
    // }

    fn test_resp() -> IronResult<Response> {
        let resp = Response::new();

        let mut context = Context::new();
        context.add("username", &"Foo");
        context.add("bio", &"Bar");
        context.add("numbers", &vec![1, 2, 3]);

        Ok(resp.set(Template::new("users/foo.html", TemplateMode::from_context(context)),),)
    }

    #[test]
    fn example() {
        let mut resp = test_resp().ok().expect("response expected");

        match resp.get::<TeraEngine>() {
            Ok(h) => {
                assert_eq!(h.name, "users/profile.html".to_string());
                assert_eq!(
                    h.name
                        .as_object()
                        .unwrap()
                        .get(&"Foo".to_string())
                        .unwrap()
                        .as_string()
                        .unwrap(),
                    "Bar"
                );
            }
            _ => panic!("template expected"),
        }
        // let teng = TeraEngine::new("src/test_template/**/*");
        // // Context option 1
        // let mut context = Context::new();
        // context.add("firstname", &"Foo");
        // context.add("lastname", &"Bar");
        //
        // let t = Template::new("index", TemplateMode::TeraContext(context));
        //
        // // Context option 2
        // let mut c2 = Context::new();
        // c2.add("firstname", &"Foo");
        // c2.add("lastname", &"Bar");
        //
        // let t4 = Template::new("index", TemplateMode::from_context(c2));
        //
        // // Using serialized values
        // let product = Product {
        //     name: "Foo".into(),
        //     value: 42,
        // };
        // // option 2
        // let t2 = Template::new("index", TemplateMode::from_serial(&product));
    }
}
