# C77Merge

A linux-native mod manager using a virtual filesystem to dynamically apply mod profiles.

### Status

extremely alpha software. has many missing features, almost no documentation, several known bugs and limitations, and multiple fundamental parts of the achitecture are subject to complete redesign at some point.

use at your own risk, on data you are able to replace.

there is a ratelimiter on the nexus api code, and i try hard to be a good citizen when using the api, but you are solely responsible for ensuring your api keys do not get banned.

### Subprojects

- binaries
  - [merge](bins/merge/)
    - applies a mod profile to a namespace, and runs the supplied command inside of it
  - [nexus_download](bins/nexus_downloa/d)
    - TODO. download the mods needed for a mod profile
  - [profile_from_downloads](bins/profile_from_downloads/)
    - create a basic profile from a directory of downloaded mods
  - [updatemods](bins/updatemods/)
    - TODO. check the nexus for updated verssions of the mods in a profile, and download and extract the newer files
- libraries
  - [c77conf](libs/c77conf/)
    - common configurations used across the tools
  - [libc77](libs/libc77)
    - common utilities and macros used across the tools
  - [acf](libs/acf/)
    - library for parsing steam's acf and vdf files
    - has a different license, as its forked from an upstream project. see the readme.
  - [nexusapi](libs/nexusapi/)
    - library for interfacing with the nexusmods api
  - [ovfs](libs/ovfs/)
    - library for building and applying overlayfs stacks

### Building

A nightly rust toolchain is required, at least until [`min_adt_const_params`](https://github.com/rust-lang/rust/issues/154042) is stabilized.
If you don't have one, I recommend installing one using rustup.

```sh
## building

# for debug builds (faster to compile, slower to run)
cargo build

# for release builds
# if you do not have wild installed, either install it or delete .cargo/config.toml
cargo build --release
```
