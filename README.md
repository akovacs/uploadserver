A simple server for which you can upload and download files from.

# Setup
1. Install rust toolchain manager

    # Arch Linux
    sudo pacman -S rustup

    # Other distributions
    curl https://sh.rustup.rs -sSf | sh

2. Download rust nightly compiler

    rustup install nightly
    rustup default nightly

3. Compile and execute the server

    cargo run --release
