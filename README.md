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

Note that debug builds can be painfully slow on real data. Release builds are recommended.

### Runnning 

The simplest invocation is:
```sh
$ target/release/lichen ~/Downloads/submissions/
```

where submissions is a directory containing one file per student. The filename (up until the first `@` or `.`) is used as the student identifier. Source language is inferred from file extension. Such a directory tree looks like:

```
submissions
├── student1.c
├── student2.c
├── student3.c
    .
    .
    .
```

This form will also accept a directory where there is a subdirectory for each student. All files in the student subdirectory will be added (recursively). This accepts a directory tree such as:

```
submissions
├── student1
│   └── src
│       ├── a.c
│       ├── b.c
│       └── c.c
├── student2
│   └── src
│       ├── a.c
│       ├── b.c
│       └── c.c
├── student3
│  └── src
│       ├── a.c
|       .
|       .
|       .
```

To view all matches, use the `-t, --thresh` option to set the reporting threshold to zero:

```sh
$ lichen -t 0 ~/Downloads/submissions/ | sort -n
```

If the language cannot be inferred from the filename (such as when AutoLab renames all files `*_handin.c`) it can be manually specified using the `-l, --lang` option. See `lichen --help` for a complete listing of included languages.

```sh
$ lichen --lang OCaml ~/Downloads/submissions
```


To selectively include files, a regular expression can be provided with the `-f, --filter` option. Only filenames matching the expression will be included. This can be used to exclude Makefiles and other configuration files and only consider source files. Note that this filter is currently only applied to filenames and cannot be used to exclude directories.

```sh
$ lichen -f ".*\.(c|h)" ~/Downloads/submissions
```

For fine tuning of the matching, the ngram length can be configured using `-n, --ngram`. Lower values are more resistant to modification of the source but may increase the false positive rate.

```sh
$ lichen -n 12 ~/Downloads/submissions
```

Sources may be explicitly allowed with `-b, --bless` which takes a directory containing files with allowed source code (e.g. handout code). This directory is not currently traversed recursively.

Similarly, the `-c, --corpus` option allows specification of known disallowed sources, such as submissions from previous semesters. Files included via this option are not scored, but similarity will be measured between provided input files is computed and recorded.

```sh
$ lichen -b handout/ -c old_submissions/ ~/Downloads/submissions
```

## License

GNU Affero General Public License v3.0. See [LICENSE](LICENSE) for details.
