#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate formdata;
#[macro_use]
extern crate lazy_static;
extern crate rocket;
extern crate time;

use std::env;
use std::fs;
use std::io;
use std::path;
use std::process;
use formdata::FormData;
use rocket::{Data, Outcome, Request};
use rocket::data::{self, FromData};
use rocket::http::{HeaderMap, Status};
use rocket::http::hyper::header::{Headers};
use rocket::response::content;

const UPLOAD_DIR: &'static str = "uploads";
lazy_static! {
    static ref UPLOAD_PATH: path::PathBuf = {
      let mut path = env::current_dir().unwrap();
      path.push(UPLOAD_DIR);
      path.to_owned()
    };
}

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
  for (name, mut file) in upload.0.files {
    //file.do_not_delete_on_drop(); // don't delete temporary file
    let filename = match file.filename() {
      Ok(Some(original_filename)) => original_filename,
      _ => time::now().to_timespec().sec.to_string()
    };
    println!("Posted file fieldname={} name={} path={:?}", name, filename, file.path);
    let mut upload_location = UPLOAD_PATH.clone();
    upload_location.push(filename);
    match fs::copy(&file.path, upload_location.as_path()) {
      Ok(_) => return Ok(format!("Uploaded {}", upload_location.to_str().unwrap())),
      Err(e) => return Err(io::Error::new(io::ErrorKind::Other, format!("Cannot write to {} directory due to {:?}", UPLOAD_DIR, e)))
    };
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

fn create_upload_directory() -> io::Result<bool> {
    fs::create_dir_all(UPLOAD_PATH.as_path())?;
    return Ok(true);
}

fn rocket() -> rocket::Rocket {
    rocket::ignite().mount("/", routes![index, upload])
}

fn main() {
    match create_upload_directory() {
        Err(error) => {
            eprintln!("Could not create ./{} directory: {}", UPLOAD_DIR, error);
            process::exit(1);
        },
        Ok(_) => {
            rocket().launch();
        }
    }
}
