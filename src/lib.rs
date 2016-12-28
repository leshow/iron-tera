#![feature(proc_macro)]
#![allow(dead_code)]

#[macro_use]
extern crate serde_derive;

extern crate serde_json;

#[macro_use]
extern crate tera;

extern crate iron;
extern crate router;

extern crate serde;

/// example of how to create templates:
///
/// ```
///
/// fn main() {
///     let mut router = Router::new();
///     router.get("/user", user_handler, "user");
///
///     let mut chain = Chain::new(router);
///     let teng = TeraEngine::new("src/examples/templates/**/*");
///     chain.link_after(teng);
///
///     Iron::new(chain).http("localhost:5000").unwrap();
/// }
///
///
/// fn user_handler(_: &mut Request) -> IronResult<Response> {
///     let mut resp = Response::new();
///
///     let mut context = Context::new();
///     context.add("username", &"Bob");
///     context.add("numbers", &vec![1, 2, 3]);
///     context.add("bio", &"<script>alert('pwnd');</script>");
///
///     resp.set_mut(Template::new("users/profile.html", TemplateMode::from_context(context)))
///         .set_mut(status::Ok);
///     Ok(resp)
/// }
/// #[derive(Serialize)]
/// struct Product {
///     name: String,
///     value: i32,
/// }
/// fn produce_handler(_: &mut Request) -> IronResult<Response> {
///     let mut resp = Response::new();
///
///     // Using serialized values
///     let product = Product {
///         name: "Foo".into(),
///         value: 42,
///     };
///     resp.set_mut(Template::new("product.html", TemplateMode::from_serial(&product)))
///         .set_mut(status::Ok);
///     Ok(resp)
/// }
/// ```

use iron::prelude::*;
use iron::{AfterMiddleware, typemap, status};
use iron::modifier::Modifier;
use iron::headers::ContentType;

use tera::{Tera, Context};

use serde::Serialize;
use serde_json::{Value, to_value};

pub enum TemplateMode {
    TeraContext(Context),
    Serialized(Value),
}

impl TemplateMode {
    pub fn from_context(context: Context) -> TemplateMode {
        TemplateMode::TeraContext(context)
    }
    pub fn from_serial<S: Serialize>(serialized: S) -> TemplateMode {
        TemplateMode::Serialized(to_value(serialized))
    }
}

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

pub struct TeraEngine {
    pub tera: Tera,
}

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


impl AfterMiddleware for TeraEngine {
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

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//     }
// }
