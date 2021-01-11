# archetect
A powerful code-centric content generator

![Rust](https://github.com/archetect/archetect/workflows/Rust/badge.svg)

## Documentation 
[Archetect Documentation](https://archetect.github.io/archetect.html)

## Binary Releases
[Releases for OSX, Windows, and Linux](https://github.com/archetect/archetect/releases)

## Installation
[Installation Guide](https://archetect.github.io/getting_started/installation.html)

## *Usage*
```
archetect 0.3.1
Jimmie Fulton <jimmie.fulton@gmail.com>


USAGE:
    archetect [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -o, --offline    Only use directories and already-cached remote git URLs
    -V, --version    Prints version information
    -v, --verbose    Increases the level of verbosity

OPTIONS:
    -a, --answer <key=value>...    Supply a key=value pair as an answer to a variable question.
    -A, --answer-file <path>...    Supply an answers file as answers to variable questions.
    -s, --switch <switches>...     Enable switches that may trigger functionality within Archetypes

SUBCOMMANDS:
    cache          Manage/Select from Archetypes cached from Git Repositories
    catalog        Select From a Catalog
    completions    Generate shell completions
    help           Prints this message or the help of the given subcommand(s)
    render         Creates content from an Archetype
    system         archetect system configuration
```
