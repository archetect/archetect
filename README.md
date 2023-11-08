# archetect

![Rust](https://github.com/archetect/archetect/workflows/Rust/badge.svg)

Archetect is a powerful code-centric, language agnostic content generator, capable of generating single files, complex
projects, or entire architectures. Key features include:

- A Jinja2-like templating syntax
- A Rhai scripting and orchestration syntax
- Easy installation
- Easy archetype authoring and publishing (Git repos or local directories)
- Smart pluralization and singularization functions (soliloquy->soliloquies, calf->calves)
- Smart casing functions (camelCase, PascalCase, snake_case, title-case, CONSTANT_CASE)
- Archetype compositions
- A distributed menu/catalog system

Modules:

- [archetect-cli](archetect-cli/README.md)
- [archetect-core](archetect-core/README.md)

## Quick Start

### Installation

For a more in-depth guide to installing archetect, see the [Installation Guide](https://archetect.github.io/getting_started/installation.html).

Archetect is a CLI application, and can either be installed by downloading a pre-built binary from Archetect's
[Releases Page](https://github.com/archetect/archetect/releases/latest), or by installing with
[Rust Lang's](https://rustup.rs/) build tool, cargo:

```shell
cargo install archetect --force
```

Once you have Archetect successfully installed and added to your shell's path, you can test that everything is working while
also initializing some default settings by generating them with Archetect itself:

```shell
archetect render https://github.com/archetect/archetect-initializer.git ~/.archetect/
```

This will prompt you for your name and email address, and write this into files within the `~/.archetect`, which you can
inspect.

### Rendering Archetypes

From this point, browse the archetypes and catalogs within the [Archetect Github organization](https://github.com/archetect)
for some pre-made archetypes you can use immediately, or for inspiration in making your own. The README.md files commonly
have an archetect command line example that can be copy/pasted to your shell to render new projects quickly and easily.

Example:

```shell
# To generate a Rust microservice using Actix and Diesel
archetect render https://github.com/archetect/archetype-rust-service-actix-diesel-workspace.git

# To select from a catalog of test_archetypes using a command line menu system
archetect catalog --source https://github.com/archetect/catalog-rust.git
```

## Documentation

- [Installation Guide](https://archetect.github.io/getting_started/installation.html)
- [Archetect Documentation](https://archetect.github.io/archetect.html)

## Binary Releases

[Releases for OSX, Windows, and Linux](https://github.com/archetect/archetect/releases)

## _Usage_

```
archetect 0.5.0
Jimmie Fulton <jimmie.fulton@gmail.com>
Generates Content from Archetype Template Directories and Git Repositories.

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
