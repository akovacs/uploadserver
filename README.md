A simple standalone webserver which you can upload and download files from.

# Setup
1. Install rust toolchain manager:

        # Arch Linux
        sudo pacman -S rustup

        # Other Linux Distributions
        curl https://sh.rustup.rs -sSf | sh

   On Windows: [Download](https://rust-lang.github.io/rustup/installation/other.html) and run the `rustup-init.exe` built
   for the `x86_64-pc-windows-msvc` target

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

* Generate SHA256 hashes for each file in uploads

        cargo run --release -- --generate_sha256

* Upload files from commandline:

        curl -X POST --data-binary @file_to_upload.txt http://localhost:8000/file_to_upload.txt


# Statically-Linked Binary

The following commands build a statically-linked Linux binary without shared library dependencies. This can be useful for embedded systems or other applications where you do not want to rely on libgcc, libc, libpthread, and other dependencies which can break due to version changes. Note that performance may be lower than the dynamically-linked binaries described above.

1. Add musl toolchain

        rustup target add x86_64-unknown-linux-musl
        sudo apt install musl-tools

2. Compile Linux binary using musl toolchain

        RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl
