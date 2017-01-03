//! # Examples
//!
//! Building a template from a context.
//! ```rust
//! fn main() {
//!     let mut router = Router::new();
//!     router.get("/user", user_handler, "user");
//!
//!     let mut chain = Chain::new(router);
//!     let teng = TeraEngine::new("src/examples/templates/**/*");
//!     chain.link_after(teng);
//!
//!     Iron::new(chain).http("localhost:5000").unwrap();
//! }
//!
//! fn user_handler(_: &mut Request) -> IronResult<Response> {
//!     let mut resp = Response::new();
//!
//!     let mut context = Context::new();
//!     context.add("username", &"Bob");
//!     context.add("numbers", &vec![1, 2, 3]);
//!     context.add("bio", &"<script>alert('pwnd');</script>");
//!
//!     resp.set_mut(Template::new("users/profile.html", TemplateMode::from_context(context)))
//!         .set_mut(status::Ok);
//!     Ok(resp)
//! }
//! ```
//! # Examples
//!
//! Note that serialize requires serde.
//! ```rust
//! #[derive(Serialize)]
//! struct Product {
//!     name: String,
//!     value: i32,
//! }
//!
//! fn produce_handler(_: &mut Request) -> IronResult<Response> {
//!     let mut resp = Response::new();
//!
//!     // Using serialized values
//!     let product = Product {
//!         name: "Foo".into(),
//!         value: 42,
//!     };
//!     resp.set_mut(Template::new("product.html", TemplateMode::from_serial(&product)))
//!         .set_mut(status::Ok);
//!     Ok(resp)
//! }
//! ```

#![feature(proc_macro)]
#![allow(dead_code)]

extern crate serde_json;

#[macro_use] extern crate tera;

extern crate iron;

extern crate serde;

use iron::prelude::*;
use iron::{AfterMiddleware, typemap, status};
use iron::modifier::Modifier;
use iron::headers::ContentType;

use tera::{Tera, Context};

use serde::Serialize;
use serde_json::{Value, to_value};

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
    pub fn from_serial<S: Serialize>(serialized: S) -> TemplateMode {
        TemplateMode::Serialized(to_value(serialized))
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
        let wrapper = resp.extensions.remove::<TeraEngine>().and_then(|t| {
            match t.mode {
                TemplateMode::TeraContext(context) => Some(self.tera.render(&t.name, context)),
                TemplateMode::Serialized(value) => Some(self.tera.value_render(&t.name, &value)),
            }
        });
        match wrapper {
            Some(result) => {
                result.map_err(|e| IronError::new(e, status::InternalServerError))
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

#[cfg(test)]
mod tests {
    use super::{TeraEngine, Template, TemplateMode};
    use iron::prelude::*;
    use tera::{Tera, Context};

    #[derive(Serialize)]
    struct Product {
        name: String,
        value: i32,
    }

    fn test_resp() -> IronResult<Response> {
        let resp = Response::new();

        let mut context = Context::new();
        context.add("username", &"Foo");
        context.add("bio", &"Bar");
        context.add("numbers", &vec![1, 2, 3]);

        Ok(resp.set(Template::new("users/foo.html", TemplateMode::from_context(context))))
    }

    #[test]
    fn example() {
        let mut resp = test_resp().ok().expect("response expected");

        match resp.get::<TeraEngine>() {
            Ok(h) => {
                assert_eq!(h.name.unwrap(), "users/profile.html".to_string());
                assert_eq!(h.value
                               .as_object()
                               .unwrap()
                               .get(&"Foo".to_string())
                               .unwrap()
                               .as_string()
                               .unwrap(),
                           "Bar");
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
