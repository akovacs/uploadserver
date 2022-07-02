A simple standalone webserver which you can upload and download files to/from
using just a web browser. By running just one instance of the uploadserver,
you can transfer files between devices on your local network without
installing anything on them.

# Ready-to-run Binaries
Compiling the source code for your machine by following the steps below will
optimize the server to use your machine to its full capability. However, if you
are lazy or in a hurry, you can [download pre-built binaries on the Releases page](https://github.com/akovacs/uploadserver/releases).

# Setup Rust Toolchain and Compile for your machine
1. Install rust toolchain manager:

        # Arch Linux
        sudo pacman -S rustup

        # Other Linux Distributions
        curl https://sh.rustup.rs -sSf | sh

   On Windows: [Download](https://rust-lang.github.io/rustup/installation/other.html)
   and run the `rustup-init.exe` built for the `x86_64-pc-windows-msvc` target

2. Download the rust nightly compiler (at least v1.45 or higher):

        rustup install nightly
        rustup default nightly

3. Clone the repository:

        git clone https://github.com/akovacs/uploadserver

4. Compile and execute the server:

        cd uploadserver
        cargo run --release

   Or alternatively if you have multiple toolchains installed:

        # For Linux x86-64
        cargo run --release --target x86_64-unknown-linux-gnu

        # For Windows x86-64
        cargo run --release --target x86_64-pc-windows-msvc

5. Browse to port 8000 at your IP address, for example: http://localhost:8000
   to upload and download files.

    ![Upload server web interface](/doc/uploadserver.png)

6. Uploaded files will be added to the `uploads` directory.


# Advanced

* Password protect files via HTTP Basic Authentication (username: admin, specify 1+ passwords)

        cargo run --release -- --password=mypassword

* Generate SHA256 hashes for each file in uploads

        cargo run --release -- --generate_sha256

* Upload files from commandline:

        curl -X POST --data-binary @file_to_upload.txt http://localhost:8000/file_to_upload.txt


# Statically-Linked Linux Binary

The following commands build a statically-linked Linux binary without shared library dependencies. This can be useful when distributing the binary to multiple different Linux distributions or to different versions of a Linux distribution, since libgcc, libc, libpthread, and other dependencies can break due to version changes.

```
/lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.33' not found
(required by ./uploadserver-x86_64-linux)
```

Note that performance may be lower than the dynamically-linked binaries described above.

1. Add musl toolchain:

        rustup target add x86_64-unknown-linux-musl
        sudo apt install musl-tools

2. Compile Linux binary using the musl toolchain:

        RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl

# Compile for Old Versions of Mac OS X

1. Install the latest version of Xcode from the App Store, or an older XCode version from [Apple Developer Downloads](https://developer.apple.com/download/all/).

2. Switch to the specified Xcode version if necessary:

        xcode-select --switch /Volumes/YOUR_VOLUME/Applications/Xcode.app

3. Configure the minimum supported OS X version (down to 10.7 Lion) in `Cargo.toml`:


        [target.x86_64-apple-darwin]
        rustflags=["-C", "link-arg=-mmacosx-version-min=10.7"]

4. Compile the binary:

        MACOSX_DEPLOYMENT_TARGET=10.7 cargo build --target=x86_64-apple-darwin --release


# TODO: Support ARM Apple Silicon (M1 processor family)
Requires MacOS Catalina 10.15.4 (Intel-based Mac) or MacOS Big Sur 11 (Apple Silicon Mac) or later.

Xcode 12.2 and later is a requirement for building universal binaries. Earlier versions of Xcode don't contain the support needed to build and test universal versions of MacOS code.

- [Github: Cargo-Lipo](https://github.com/TimNN/cargo-lipo) can automatically create universal libraries (fat binaries supporting both Intel x86_64 processors and Apple Silicon) for iOS and Mac.
- [Github: Homebrew MacOS Cross-Compilation Toolchains](https://github.com/messense/homebrew-macos-cross-toolchains)
- [Stack Overflow: How do I cross compile a Rust application from macOS x86 to macOS Silicon?](https://stackoverflow.com/questions/66849112/how-do-i-cross-compile-a-rust-application-from-macos-x86-to-macos-silicon)


# Further Info
- [The Rust Book: Platform Support](https://doc.rust-lang.org/rustc/platform-support.html)
- [Rust-Lang Users: Compile rust binary for older versions of Mac OSX](https://users.rust-lang.org/t/compile-rust-binary-for-older-versions-of-mac-osx/38695/6)
- [William Saar's Blog: Shipping Linux binaries that don't break with Rust](https://saarw.github.io/dev/2020/06/18/shipping-linux-binaries-that-dont-break-with-rust.html)
- [The World Aflame: Cross-compiling a simple Rust web app](https://www.andrew-thorburn.com/cross-compiling-a-simple-rust-web-app/)
