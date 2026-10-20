# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/)
and this project adheres to [Semantic Versioning](https://semver.org/).

Changelog notes for each version may be split into:

 - Breaking changes: user-visible changes which may require updates in downstream components.
 - Compatibility: which compiler and dependency versions are supported.
 - Features: new functionality, or backwards-compatible changes in existing functionality.
 - Fixes: bugfixes, security fixes, removing sharp edges.
 - Performance: performance-oriented improvements: memory-usage, cpu usage, etc.
 - Releases: ready-to-run compiled binaries which may be distributed via Github Releases
    or other distribution systems.


## [1.0.1] - 2026-10-17

### Features

 - Includes rudimentary password protection functionality using --password=mypassword option which enables HTTP Basic Authentication.

### Compatibility: Compiled using
 - cargo 1.85.0-nightly (769f622e1 2024-12-14)
 - rustc 1.85.0-nightly (0aeaa5eb2 2024-12-14)

## [1.0.0] - 2022-05-15

### Features

 - Initial release of server which lists `uploads/` directory with links to downloadable files. - "Select a file to upload" form which permits you to choose a file and automatically upload it to the server as an HTTP POST via in-browser Javascript/ECMAScript or commandline tools like curl.
 - Includes `--generate_sha256` option to generate SHA256 hashes for each file added to the uploads directory.

### Performance

- Reduced binary size via release mode flags such as stripping symbols, link time optimization, and only a single compilation unit to reduce the final binary size

### Releases

 - `uploadserver-x86_64-linux` (2.80 MB): statically linked using musl libc for 64-bit GNU/Linux (tested on Ubuntu 22.04)
 - `uploadserver-x86_64-osx` (2.31 MB): for 64-bit Intel Mac OS X (supports 10.7 Lion and higher, tested on 10.11 El Capitan)
 - `uploadserver-x86_64-windows.exe` (2.52 MB): for 64-bit Windows (tested on Windows 10)

```
sha256sum *
85fb867c705e5fe18b3101de845ec1697709d487f70f7a342b981dced14206e8  uploadserver-x86_64-linux
25b7c11e2cbca9d80e022dc7fad7806bafe324e2947fd9e00ac11b328e0d0a3e  uploadserver-x86_64-osx
78882cb199386130582414e567c5d6fca5a654805be8d1efc5e2b8c4afdd2264  uploadserver-x86_64-windows.exe
```

### Compatibility: Compiled using
 - cargo 1.62.0-nightly (3f052d8ee 2022-05-12)
 - rustc 1.62.0-nightly (70b3681bf 2022-05-14)
