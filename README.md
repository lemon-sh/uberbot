# überbot
An IRC bot Above All [other bots].

## Features
- SED expressions
- Leek, fun commands for manipulating text:
    - mock, mock a message made by someone lIkE tHiS
    - leet, coverts specific chars to the number which looks similar, for example C4761rls
    - owo, self explanatory
- Quoting
- Fetching pictures from [waifu.pics](https://waifu.pics)
- Title of links sent in a channel, currently supports:
  - HTTP webpages
  - Spotify
- Rust evaluation using the rust playground

## Setup
### Compiling the binary
To compile the binary, you will first need to have the following installed:
- git
- A rust toolchain

Clone the source code:

`git clone https://git.lemonsh.moe/lemon/uberbot.git`

Compile the source code:

`cd uberbot && cargo build --release`

After the compiling has finished, you can find the binary under `target/release/uberbot`

### Configuration

Uberbot uses the environment variable `UBERBOT_CONFIG`, if it is not set
it will look for `uberbot.toml` in the working directory.

An example configuration can be found in `sample_uberbot.toml`
