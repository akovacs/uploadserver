#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate formdata;
extern crate rocket;
extern crate rocket_file_cache;
extern crate time;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use formdata::FormData;
use rocket::{Data, Outcome, Request, State};
use rocket::data::{self, FromData};
use rocket::http::{HeaderMap, Status};
use rocket::http::hyper::header::{Headers};
use rocket::response::content;
use rocket_file_cache::{Cache, CachedFile};

const UPLOAD_DIR: &'static str = "uploads/";

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
    type Error = String;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        let headers = from(request.headers());

        match formdata::read_formdata(&mut data.open(), &headers) {
            Ok(parsed_form) => return Outcome::Success(RocketFormData(parsed_form)),
            _ => return Outcome::Failure((Status::BadRequest, String::from("Failed to read formdata")))
        };
    }
}

#[post("/<filename>", data = "<data>")]
fn upload_binary(filename: String, data: Data) -> io::Result<String> {
    data.stream_to_file(format!("{}/{}", UPLOAD_DIR, filename))
        .map(|numbytes| format!("Uploaded {} bytes as {}", numbytes.to_string(), filename))
}

#[post("/", format = "multipart/form-data", data = "<upload>")]
fn upload_form(upload: RocketFormData) -> io::Result<String> {
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
    let upload_location = Path::new(UPLOAD_DIR).join(&filename);
    match fs::copy(&file.path, &upload_location) {
      Ok(_) => return Ok(format!("Uploaded {}", filename)),
      Err(error) => return Err(io::Error::new(io::ErrorKind::Other,
                        format!("Cannot write to {} directory due to {:?}", UPLOAD_DIR, error)))
    };
  }
  return Err(io::Error::new(io::ErrorKind::InvalidInput, "No files uploaded"));
}

#[get("/")]
fn index() -> content::Html<&'static str> {
  content::Html(r#"
    <!doctype html>
      <title>Upload a file</title>
      <h1>Select a file to upload</h1>
      <form action="" method="post" enctype="multipart/form-data">
        <p>
          <input id="fileinput" name="file" type="file" />
          <input id="upload" type="submit" value="Upload" />
        </p>
        <div id="response"></div>
      </form>
      <script>
      // Check for the various File API support.
      if (window.File && window.FileList) {
        // Disable upload button.
        document.getElementById('upload').style.display='none';
 
        function readFile(fileInputEvent) {
          //Retrieve the first (and only!) File from the FileList object
          var inputFile = fileInputEvent.target.files[0];
          var postToServer = new XMLHttpRequest();
          postToServer.open('POST', '/' + inputFile.name, true);
          postToServer.onreadystatechange = function() {
            if (postToServer.readyState==4 && postToServer.status==200){
              document.getElementById('response').innerHTML = 'Success: '+ postToServer.responseText;
            } else {
              document.getElementById('response').innerHTML = 'Failure: HTTP Error '
                + postToServer.status + ' ' + postToServer.responseText;
            }
            fileInputEvent.target.value = "";
          }
          postToServer.send(inputFile);
        }

        document.getElementById('fileinput').addEventListener('change', readFile, false);
      }
      </script>
    </html>
  "#)
}

#[get("/<file..>")]
fn files(file: PathBuf, cache: State<Cache>) -> CachedFile {
    CachedFile::open(Path::new(UPLOAD_DIR).join(file), cache.inner())
}

fn create_upload_directory() -> io::Result<bool> {
    fs::create_dir_all(UPLOAD_DIR)?;
    return Ok(true);
}

fn main() {
    match create_upload_directory() {
        Err(error) => {
            eprintln!("Could not create ./{} directory: {}", UPLOAD_DIR, error);
            process::exit(1);
        },
        Ok(_) => {
            let cache: Cache = Cache::new(1024 * 1024 * 128); // 128 MB
            rocket::ignite().manage(cache)
                .mount("/", routes![files, index, upload_binary, upload_form]).launch();
        }
    }
}
