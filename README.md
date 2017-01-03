iron-tera
-------


This is a [Tera](https://github.com/Keats/tera/) middleware for [Iron](https://github.com/iron/iron/).

Check me out on [crates.io](https://crates.io/crates/iron-tera) or read the [documentation](https://docs.rs/iron-tera/).

After the initial template engine is created, you can render templates in a given handler using either a Tera `Context`, or a value that implementes serde's `Serialize`.

```rust
extern crate iron_tera;

use iron_tera::{TeraEngine, Template, TemplateMode};

fn main() {
    let mut router = Router::new();
    router.get("/user", user_handler, "user");

    let mut chain = Chain::new(router);
    let teng = TeraEngine::new("src/examples/templates/**/*"); // create tera engine from these templates
    chain.link_after(teng);

    Iron::new(chain).http("localhost:5000").unwrap();
}

fn user_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();

    // Use a context
    let mut context = Context::new();
    context.add("username", &"Bob");
    context.add("numbers", &vec![1, 2, 3]);
    context.add("bio", &"<script>alert('pwnd');</script>");

    resp.set_mut(Template::new("users/profile.html", TemplateMode::from_context(context)))
        .set_mut(status::Ok);
    Ok(resp)
}

#[derive(Serialize)]
struct Product {
    name: String,
    value: i32,
}

fn produce_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();

    // Using serialized values
    let product = Product {
        name: "Foo".into(),
        value: 42,
    };
    resp.set_mut(Template::new("product.html", TemplateMode::from_serial(&product)))
        .set_mut(status::Ok);
    Ok(resp)
}
```
