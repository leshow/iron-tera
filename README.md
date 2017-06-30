iron-tera
-------


This is a [Tera](https://github.com/Keats/tera/) middleware for [Iron](https://github.com/iron/iron/).

Check me out on [crates.io](https://crates.io/crates/iron-tera) or read the [documentation](https://docs.rs/iron-tera/).

After the initial template engine is created, you can render templates in a given handler using either a Tera `Context`, or a value that implementes serde's `Serialize`.

```rust
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
    context.add("my_var", &"Thing"); 
    context.add("numbers", &vec![1, 2, 3]);
    context.add("bio", &"<script>alert('pwnd');</script>");

    resp.set_mut(Template::new(
        "users/profile.html",
        TemplateMode::from_context(context),
    )).set_mut(status::Ok);
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
```
