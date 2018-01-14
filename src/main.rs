#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::response::content;

#[get("/")]
fn index() -> content::Html<&'static str> {
  content::Html(r#"
    <!doctype html>
    <title>Hello World</title>
    <h1>Hello World!</h1>
    </html>
  "#)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index])
}

fn main() {
    rocket().launch();
}
