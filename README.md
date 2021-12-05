# Lichen: A FLOSS software similarity detector

## Usage

### Installation

#### Binary Installation
```sh
$ cargo install --git https://github.com/iracigt/lichen.git
```

#### Development
```sh
$ git clone https://github.com/iracigt/lichen.git
$ cd lichen
$ cargo build --release
$ target/release/lichen ...
```

Note that development builds can be painfully slow on real data. Release builds are recommended.

### Runnning 

```sh
$ target/release/lichen ~/Downloads/submissions/
```

## License

GNU Affero General Public License v3.0. See [LICENSE](LICENSE) for details.
