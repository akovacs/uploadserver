#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate formdata;
extern crate rocket;

use std::io;
use formdata::FormData;
use rocket::{Data, Outcome, Request};
use rocket::data::{self, FromData};
use rocket::http::{HeaderMap, Status};
use rocket::http::hyper::header::{Headers};
use rocket::response::content;

// Wrap formdata::FormData in order to implement FromData trait
struct RocketFormData(FormData);

fn from(header_map: &HeaderMap) -> Headers {
    let mut headers = Headers::new();
    for header in header_map.iter() {
        let header_value: Vec<u8> = header.value().as_bytes().to_owned();
        headers.append_raw(String::from(header.name()), header_value);
    }
    headers
}

impl FromData for RocketFormData {
    type Error = ();

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        let headers = from(request.headers());
        match formdata::read_formdata(&mut data.open(), &headers) {
            Ok(parsed_form) => return Outcome::Success(RocketFormData(parsed_form)),
            _ => return Outcome::Failure((Status::BadRequest, ()))
        };
    }
}

#[post("/", format = "multipart/form-data", data = "<upload>")]
fn upload(upload: RocketFormData) -> io::Result<String> {
  for (name, value) in upload.0.fields {
    println!("Posted field name={} value={}", name, value);
  }
  for (name, file) in upload.0.files {
    println!("Posted file name={} path={:?}", name, file.path);
    return Ok(format!("Uploaded {}", file.path.to_str().unwrap()));
  }
  return Err(io::Error::new(io::ErrorKind::InvalidInput, "No files uploaded"));
}

#[get("/")]
fn index() -> content::Html<&'static str> {
  content::Html(r#"
    <!doctype html>
    <title>Upload new File</title>
    <h1>Upload new File</h1>
    <form action="" method=post enctype=multipart/form-data>
      <p><input type=file name=file>
         <input type=submit value=Upload>
    </form>
    </html>
  "#)
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index, upload])
}

fn main() {
    rocket().launch();
}
