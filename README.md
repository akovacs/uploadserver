A simple standalone webserver for which you can upload and download files from.

# Setup
1. Install rust toolchain manager:

        # Arch Linux
        sudo pacman -S rustup

        # Other distributions
        curl https://sh.rustup.rs -sSf | sh

2. Download rust nightly compiler:

        rustup install nightly
        rustup default nightly

3. Compile and execute the server:

        cargo run --release

   Or alternatively if you have multiple toolchains installed:

        cargo run --release --target x86_64-unknown-linux-gnu

4. Browse to http://localhost:8000 to upload and download files.

    ![Upload server web interface](/doc/uploadserver.png)

# Advanced

* Generate SHA256 hashes for each file in uploads

        cargo run --release -- --generate_sha256

* Upload files from commandline:

        curl -X POST --data-binary @file_to_upload.txt http://localhost:8000/file_to_upload.txt
