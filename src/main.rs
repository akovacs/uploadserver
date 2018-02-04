#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate chrono;
#[macro_use]
extern crate clap;
extern crate formdata;
extern crate mime_guess;
extern crate pretty_bytes;
extern crate rocket;
extern crate rocket_file_cache;
extern crate time;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use chrono::{DateTime, Utc};
use clap::App;
use formdata::FormData;
use pretty_bytes::converter::convert;
use rocket::{Data, Outcome, Request, State};
use rocket::data::{self, FromData};
use rocket::http::{HeaderMap, Status};
use rocket::http::hyper::header::{Headers};
use rocket::response::{content, Content};
use rocket_file_cache::{Cache, CacheBuilder, CachedFile};

const UPLOADS_DIR: &'static str = "uploads/";

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
    data.stream_to_file(format!("{}/{}", UPLOADS_DIR, filename))
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
    let upload_location = Path::new(UPLOADS_DIR).join(&filename);
    match fs::copy(&file.path, &upload_location) {
      Ok(_) => return Ok(format!("Uploaded {}", filename)),
      Err(error) => return Err(io::Error::new(io::ErrorKind::Other,
                        format!("Cannot write to {} directory due to {:?}", UPLOADS_DIR, error)))
    };
  }
  return Err(io::Error::new(io::ErrorKind::InvalidInput, "No files uploaded"));
}

#[get("/")]
fn index() -> content::Html<&'static str> {
  content::Html(r#"
    <!doctype html>
      <head>
        <title>Upload a file</title>
        <style>
          body, html {
            width: 100%; height: 100%; margin: 0; padding: 0;
          }
          .first-row {
            position: absolute; top: 0; left: 0; left: 0; right: 0;
            height: 8em; margin: 10px;
          }
          .second-row {
            position: absolute; top: 8em; left: 0; right: 0; bottom: 0;
          }
          .second-row iframe {
            position: absolute; top: 0; left: 0; width: 100%; height: 100%;
            border: none; margin: 0; padding: 0;
          }
        </style>
      </head>
      <div class="first-row">
        <h1>Select a file to upload</h1>
        <form action="" method="post" enctype="multipart/form-data">
          <p>
            <input id="fileinput" name="file" type="file" />
            <input id="upload" type="submit" value="Upload" />
            <span id="response"></span>
          </p>
        </form>
      </div>
      <div class="second-row">
        <iframe src="list/" frameborder="0"></iframe>
      </div>
      <script>
      // Check for the various File API support.
      if (window.File && window.FileList) {
        // Disable upload button.
        document.getElementById('upload').style.display='none';

        function updateProgress (transferEvent) {
          if (transferEvent.lengthComputable) {
            var percentComplete = (transferEvent.loaded / transferEvent.total * 100).toFixed(2);
            document.getElementById('response').innerHTML = 'Transfer: ' + percentComplete + '% complete';
          }
        }

        function transferFailed(evt) {
          console.log("An error occurred while transferring the file.");
        }

        function transferCanceled(evt) {
          console.log("The transfer has been canceled by the user.");
        }

        function readFile(fileInputEvent) {
          //Retrieve the first (and only!) File from the FileList object
          var inputFile = fileInputEvent.target.files[0];
          var postToServer = new XMLHttpRequest();
          postToServer.open('POST', '/' + inputFile.name, true);
          postToServer.upload.addEventListener("progress", updateProgress);
          postToServer.upload.addEventListener("error", transferFailed);
          postToServer.upload.addEventListener("abort", transferCanceled);
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

#[get("/uploads/<file..>")]
fn files(file: PathBuf, cache: State<Cache>) -> CachedFile {
    CachedFile::open(Path::new(UPLOADS_DIR).join(file), cache.inner())
}

fn create_upload_directory() -> io::Result<bool> {
    fs::create_dir_all(UPLOADS_DIR)?;
    return Ok(true);
}

#[get("/list")]
fn list() -> content::Html<String> {
    if let Ok(entries) = fs::read_dir(UPLOADS_DIR) {
        let mut table = vec![String::from(r#"
        <table border="1">
          <tr>
            <th>File Name</th>
            <th>Type</th>
            <th>Size</th>
            <th>Modified</th>
          </tr>"#)];
        let mut files: Vec<_> = entries.filter_map(|entry| entry.ok()).collect();
        files.sort_by_key(|file| file.path());
        for file in files {
            if let Ok(metadata) = file.metadata() {
                let modified_time = match metadata.modified() {
                    Ok(system_time) => {
                      let datetime: DateTime<Utc> = system_time.into();
                      datetime.to_rfc2822()
                    },
                    _ => String::from("Unknown Modification Time")
                };
                let file_name = &file.file_name();
                let file_name_string = file_name.to_string_lossy();
                let path = &file.path();
                let mime_type = mime_guess::guess_mime_type(&path).to_string();
                let file_size = convert(metadata.len() as f64);
                let path_string = path.display();
                table.push(format!("<tr><td><a href=\"/{}\">{}</a></td><td>{}</td><td align='right'>{}</td><td>{}</td>", path_string, file_name_string, mime_type, file_size, modified_time));
            }
        }
        table.push("</table>".to_string());
        return content::Html(table.join("\n"));
    }
    return content::Html(String::from("Error listing directory"));
}


fn main() {
    let matches = App::new("Fileserver")
        .version("0.1.0")
        .about("Simple standalone webserver which you can upload and download files from")
        .args_from_usage(
            "--filecache_size <CACHE_SIZE> 'In-memory cache of frequently-accessed files in MB (default is 128 MB)'")
        .get_matches();
    let filecache_size = value_t!(matches, "filecache_size", usize).unwrap_or(128);
    println!("Using {} MB file cache", filecache_size);

    match create_upload_directory() {
        Err(error) => {
            eprintln!("Could not create ./{} directory: {}", UPLOADS_DIR, error);
            process::exit(1);
        },
        Ok(_) => {
            let cache: Cache = CacheBuilder::new().size_limit(1024 * 1024 * filecache_size).build().unwrap();
            rocket::ignite().manage(cache)
                .mount("/", routes![files, index, list, upload_binary, upload_form]).launch();
        }
    }
}
