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

4. Browse to http://localhost:8000 to upload and download files.

# Advanced
* Specify size of in-memory cache (in MB) for serving frequently-accessed files:

        cargo run --release -- --filecache_size=1024

* Generate SHA256 hashes for each file in uploads

        cargo run --release -- --generate_sha256
