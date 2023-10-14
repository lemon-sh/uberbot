# überbot
Multifunctional IRC bot

## Features
- sed expressions
- math expressions
- Leek, fun commands for manipulating text:
    - mock, mock a message made by someone lIkE tHiS
    - leet, coverts specific chars to the number which looks similar, for example C4761rls
    - owo, owofies the text
- Quoting messages
- Fetching pictures from [waifu.pics](https://waifu.pics)
- Title of links sent in a channel, currently supports:
  - HTML webpages (`<title>` tag)
  - Spotify (track metadata - artist, duration, etc.)

## Setup

### Compiling the binary
You should have the latest Rust toolchain installed.

Clone the source code:

`git clone https://git.lemonsh.moe/lemon/uberbot.git`

Compile the source code:

`cd uberbot && cargo build --release`

After the compiling has finished, you can find the binary under `target/release/uberbot`

#### MSRV
The MSRV (Minimum Supported Rust Version) for überbot is currently **1.67**.

### Configuration

überbot uses the environment variable `UBERBOT_CONFIG`, if it is not set
it will look for `uberbot.toml` in the working directory.

An example configuration can be found in `sample_uberbot.toml`
