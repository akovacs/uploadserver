#![feature(proc_macro_hygiene, decl_macro)]

extern crate chrono;
#[macro_use]
extern crate clap;
extern crate crypto;
extern crate formdata;
extern crate mime_guess;
extern crate notify;
extern crate pretty_bytes;
#[macro_use]
extern crate rocket;
extern crate rocket_basicauth;
extern crate time;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::{Read, ErrorKind};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::{App, Arg};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use formdata::FormData;
use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};
use pretty_bytes::converter::convert;
use rocket::{Data, Outcome, State};
use rocket::data::{self, FromDataSimple};
use rocket::http::{self, Header, HeaderMap, Status};
use rocket::http::hyper::header::{Headers, Authorization};
use rocket::request::{self, Request, FromRequest};
use rocket::response::{content, status, NamedFile, Redirect, Response, Responder};
use rocket::response::status::Unauthorized;
use rocket_basicauth::BasicAuth;

const UPLOADS_DIR: &'static str = "uploads/";
const SHA256_EXTENSION: &'static str = "sha256";


// Custom Authenticated type which we implement FromData for
// in order to guard protected endpoints
// https://github.com/SergioBenitez/Rocket/issues/99
struct Authenticated {}

// #[derive(Clone,Debug)]
struct AuthorizedUser {
  name: String,
  // TODO: add cookie after initial authentication
  password: Vec<String>
}

// Wrap server configuration for password auth in shared State struct
// https://rocket.rs/v0.4/guide/state/#state
struct ServerConfig {
  users: HashMap<String,AuthorizedUser>
}

// Wrap formdata::FormData in order to implement FromData trait
//#[derive(FromForm)]
struct RocketFormData{
  value: FormData
}

fn from(header_map: &HeaderMap) -> Headers {
    let mut headers = Headers::new();
    for header in header_map.iter() {
        let header_value: Vec<u8> = header.value().as_bytes().to_owned();
        headers.append_raw(String::from(header.name()), header_value);
    }
    headers
}

impl FromDataSimple for RocketFormData {
    type Error = String;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        let headers = from(request.headers());

        match formdata::read_formdata(&mut data.open(), &headers) {
            Ok(parsed_form) => return data::Outcome::Success(RocketFormData { value: parsed_form }),
            _ => {
                return data::Outcome::Failure((Status::BadRequest,
                                         String::from("Failed to read formdata")))
            }
        };
    }
}

#[post("/<filename>", data = "<data>")]
fn upload_binary(_auth: Authenticated, filename: String, data: Data) -> io::Result<String> {
    data.stream_to_file(format!("{}/{}", UPLOADS_DIR, filename))
        .map(|numbytes| format!("Uploaded {} bytes as {}", numbytes.to_string(), filename))
}

#[post("/", format = "multipart/form-data", data = "<upload>")]
fn upload_form(_auth: Authenticated, upload: RocketFormData) -> io::Result<String> {
    for (name, value) in upload.value.fields {
        println!("Posted field name={} value={}", name, value);
    }
    for (name, mut file) in upload.value.files {
        // file.do_not_delete_on_drop(); // don't delete temporary file
        let filename = match file.filename() {
            Ok(Some(original_filename)) => original_filename,
            _ => time::now().to_timespec().sec.to_string(),
        };
        println!("Posted file fieldname={} name={} path={:?}",
                 name,
                 filename,
                 file.path);
        let upload_location = Path::new(UPLOADS_DIR).join(&filename);
        match fs::copy(&file.path, &upload_location) {
            Ok(_) => return Ok(format!("Uploaded {}", filename)),
            Err(error) => {
                return Err(io::Error::new(io::ErrorKind::Other,
                                          format!("Cannot write to {} directory due to {:?}",
                                                  UPLOADS_DIR,
                                                  error)))
            }
        };
    }
    return Err(io::Error::new(io::ErrorKind::InvalidInput, "No files uploaded"));
}


// Request guard for basic auth check
// TODO: reenable after upgrading to rocket 0.5
// #[rocket::async_trait]
// impl<'r> FromRequest<'r> for Authenticated {
impl<'a, 'r> FromRequest<'a, 'r> for Authenticated {
    type Error = &'static str;
    // TODO: async after upgrading to rocket 0.5
    // async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
    fn from_request(req: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        // Change when upgrading to rocket 0.5
        // let server_config = req.rocket().state::<ServerConfig>().unwrap();
        let server_config = req.guard::<State<ServerConfig>>().unwrap();
        if server_config.users.is_empty() {
            println!("No users are configured, bypassing basic authentication check");
            return request::Outcome::Success(Authenticated {});
        }

        // let auth = req.guard::<BasicAuth>().await;
        let auth = req.guard::<BasicAuth>();

        let basic_auth_username = match auth {
            // auth.name becomes auth.username in later versions of BasicAuth
            // request::Outcome::Success(ref a) => a.username.as_str(),
            request::Outcome::Success(ref a) => a.name.as_str(),
            _ => "",
        };

        let basic_auth_password = match auth {
            request::Outcome::Success(ref a) => a.password.as_str(),
            _ => "",
        };

        match server_config.users.get(&basic_auth_username.to_string()) {
            Some(authorized_user) => {
                // TODO: constant time password check?
                if authorized_user.password.contains(&basic_auth_password.to_string()) {
                    println!("Successful login for user {}", &authorized_user.name);
                    return request::Outcome::Success(Authenticated {});
                } else {
                    println!("Failed Login attempt with incorrect password for {}", &authorized_user.name);
                    return request::Outcome::Failure((
                        http::Status::Unauthorized,
                        "Auth check failed. Please perform HTTP basic auth with the correct username and password.",
                    ));
                }
            },
            _ => {
                return request::Outcome::Failure((
                    http::Status::Unauthorized,
                    "Auth check failed. Please perform HTTP basic auth with the correct username and password.",
                ));
            }
        }
    }
}

// Catches 401 Unauthorized errors and respond with a request to the client for basic auth login
#[catch(401)]
fn unauthorized_catcher<'r: 'r>() -> impl Responder<'r> {
    struct Resp {}
    impl<'r: 'r> Responder<'r> for Resp {
        fn respond_to(
            self,
            _request: &Request,
        ) -> Result<rocket::Response<'r>, rocket::http::Status> {
            Ok(Response::build()
                   .header(Header::new("WWW-Authenticate", "Basic realm=\"UploadServer Login\", charset=\"UTF-8\""))
                   .status(http::Status::Unauthorized)
                   .finalize())
        }
    }
    Resp {}
}


#[get("/")]
fn index(_auth: Authenticated, server_config: State<ServerConfig>) -> Result<content::Html<&'static str>, Response> {
    // For 3+ possible responses, derive a custom Responder enum
    // https://rocket.rs/master/guide/faq/#multiple-responses
    // https://github.com/SergioBenitez/Rocket/issues/253
    // TODO: factor out into dedicated function
    // TODO: auth.name becomes auth.username in later versions of BasicAuth

    // println!("Basic Auth {:?}", &auth);
    // match server_config.users.get(&auth.name) {
    //     Some(authorized_user) => {
    //         println!("Matched Authorized User {:?}", &authorized_user.name);
    //     },
    //     _ => { return Err(Response::build()
    //                      .status(Status::Unauthorized)
    //                      .raw_header("WWW-Authenticate", "Basic realm=\"UploadServer Login\", charset=\"UTF-8\"")
    //                      .finalize()) }
    // }

    return Ok(content::Html(r#"
    <!doctype html>
      <head>
        <title>Upload a file</title>
        <style>
          body, html {
            width: 100%; height: 100%; margin: 0; padding: 0;
          }
          .first-row {
            position: absolute; top: 0; left: 0; left: 0; right: 0;
            height: 10em; margin: 10px;
          }
          .second-row {
            position: absolute; top: 10em; left: 0; right: 0; bottom: 0;
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
          <p>Or <code>curl -X POST --data-binary @file_to_upload.txt http://localhost:8000/file_to_upload.txt</code></p>
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
            document.getElementById('response').innerHTML =
              'Transfer: ' + percentComplete + '% complete';
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
              document.getElementById('response').innerHTML =
                'Success: '+ postToServer.responseText;
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
  "#))
}

#[get("/uploads/<file..>")]
fn files(_auth: Authenticated, file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new(UPLOADS_DIR).join(file)).ok()
}

fn create_upload_directory() -> io::Result<bool> {
    fs::create_dir_all(UPLOADS_DIR)?;
    return Ok(true);
}

#[get("/list")]
fn list(_auth: Authenticated) -> content::Html<String> {
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
                    }
                    _ => String::from("Unknown Modification Time"),
                };
                let file_name = &file.file_name();
                let file_name_string = file_name.to_string_lossy();
                let path = &file.path();
                let mime_type = mime_guess::guess_mime_type(&path).to_string();
                let file_size = convert(metadata.len() as f64);
                let path_string = path.display();
                table.push(format!("<tr><td><a href=\"/{}\">{}</a></td><td>{}</td><td \
                                    align='right'>{}</td><td>{}</td>",
                                   path_string,
                                   file_name_string,
                                   mime_type,
                                   file_size,
                                   modified_time));
            }
        }
        table.push("</table>".to_string());
        return content::Html(table.join("\n"));
    }
    return content::Html(String::from("Error listing directory"));
}


fn compute_sha256(path: &PathBuf) -> io::Result<String> {
    let mut sha256_hasher = Sha256::new();
    let mut buffer = [0; 4096];
    let mut file = fs::File::open(&path)?;
    loop {
        let len = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(len) => len,
            Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        };
        sha256_hasher.input(&buffer[..len]);
    }
    return Ok(sha256_hasher.result_str());
}


fn write_sha256(pathbuf: &mut PathBuf) -> io::Result<bool> {
    let new_extension = match pathbuf.extension() {
        Some(extension) => {
            if extension == SHA256_EXTENSION {
                println!("Skipping SHA256 computation of {}", pathbuf.display());
                return Ok(false);
            } else {
                [extension.to_string_lossy(), Cow::Borrowed(SHA256_EXTENSION)].join(".")
            }
        }
        None => String::from(SHA256_EXTENSION),
    };
    let sha256_hash = compute_sha256(&pathbuf)?;
    println!("SHA256 of {} is {}", pathbuf.display(), sha256_hash);
    pathbuf.set_extension(new_extension);
    let path = pathbuf.as_path();
    let mut sha256_file = fs::File::create(path)?;
    sha256_file.write_all(sha256_hash.as_bytes())?;
    sha256_file.sync_all()?;
    return Ok(true);
}


fn write_sha256_ignoring_errors(pathbuf: &mut PathBuf) {
    match write_sha256(pathbuf) {
        Err(error) => {
            println!("Encountered error while writing {}: {}",
                     pathbuf.display(),
                     error);
            return;
        }
        _ => return,
    }
}


fn main() {
    let matches = App::new("Fileserver")
        .version("0.1.0")
        .about("Simple standalone webserver which you can upload and download files from")
        .arg(Arg::with_name("generate_sha256")
            .long("generate_sha256")
            .help("Generate SHA256 hashes for each uploaded file"))
        .arg(Arg::with_name("password")
            .long("password")
            // TODO: customize users via interleaved arguments or config file:
            // --user admin --password password
            .help("Protect access via HTTP Basic Authentication and require a password to access files. Default user is admin.")
            .multiple(true)
            .min_values(1))
        .get_matches();

    match create_upload_directory() {
        Err(error) => {
            eprintln!("Could not create ./{} directory: {}", UPLOADS_DIR, error);
            process::exit(1);
        }
        Ok(_) => {
            let server_config: ServerConfig = match matches.values_of("password") {
               Some(passwds) => {
                   let passwords = passwds.map(|passwd| passwd.to_string()).collect();
                   // if &passwords.len() > &0 {
                   //     println!("Authorized Passwords: {:?}", &passwords);
                   // }
                   ServerConfig {
                       users: HashMap::from([(String::from("admin"), AuthorizedUser { name: String::from("admin"), password: passwords })])
                   }
               }
               None => { ServerConfig { users: HashMap::new() } }
            };
            if matches.is_present("generate_sha256") {
                if let Ok(entries) = fs::read_dir(UPLOADS_DIR) {
                    let mut paths_to_hash: Vec<_> = entries.filter_map(|entry| entry.ok())
                        .map(|valid_entry| valid_entry.path())
                        .filter(|path| !path.is_dir())
                        .filter(|path| {
                            match path.extension() {
                                Some(extension) => extension != SHA256_EXTENSION,
                                None => true,
                            }
                        })
                        .collect();
                    for mut pathbuf in paths_to_hash {
                        write_sha256_ignoring_errors(&mut pathbuf);
                    }
                }
                thread::spawn(|| {
                    // Create a channel to receive the events.
                    let (tx, rx) = channel();

                    // Automatically select the best implementation for your platform.
                    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

                    // Add a path to be watched. All files and directories at that path and
                    // below will be monitored for changes.
                    watcher.watch(UPLOADS_DIR, RecursiveMode::Recursive).unwrap();
                    loop {
                        match rx.recv() {
                            Ok(event) => {
                                println!("{:?}", event);
                                match event {
                                    DebouncedEvent::Create(mut created_path) => {
                                        write_sha256_ignoring_errors(&mut created_path)
                                    }
                                    DebouncedEvent::Write(mut modified_path) => {
                                        write_sha256_ignoring_errors(&mut modified_path)
                                    }
                                    DebouncedEvent::Rename(_, mut after_path) => {
                                        write_sha256_ignoring_errors(&mut after_path)
                                    }
                                    _ => continue,

                                };
                            }
                            Err(e) => println!("watch error: {:?}", e),
                        }
                    }
                });
            }
            rocket::ignite()
                .manage(server_config)
                .mount("/", routes![files, index, list, upload_binary, upload_form])
                // Upgrade for 0.5:
                // .register("/", catchers![unauthorized_catcher,])
                .register(catchers![unauthorized_catcher,])
                .launch();
        }
    }
}
