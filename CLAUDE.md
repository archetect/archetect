# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Archetect is a powerful code-centric, language-agnostic content generator with a Jinja2-like templating syntax and Rhai scripting. It generates single files, complex projects, or entire architectures from Git repositories or local directories.

## Workspace Structure

This is a Cargo workspace with multiple crates:

- **archetect-core**: Core functionality including archetype rendering, scripting, and template processing
- **archetect-bin**: CLI application (main binary)
- **archetect-api**: Public API definitions for IoDriver and command interfaces  
- **archetect-templating**: Custom Jinja2-like templating engine (vendored minijinja fork)
- **archetect-terminal-io**: Terminal I/O driver implementation
- **archetect-terminal-prompts**: Interactive prompts library (vendored inquire fork)
- **archetect-inflections**: String manipulation utilities (pluralization, case conversion)
- **archetect-validations**: Input validation utilities
- **xtask**: Build automation tasks

## Common Development Commands

```bash
# Build the entire workspace
cargo build

# Run tests for the entire workspace
cargo test

# Run tests for a specific crate
cargo test -p archetect-core

# Install the CLI locally for testing
cargo xtask install

# Build with static OpenSSL (useful for distribution)
cargo xtask install --static-openssl

# Run clippy linting
cargo clippy --all-targets --all-features

# Format code
cargo fmt

# Run the CLI locally
cargo run -p archetect-bin -- <args>
```

## Architecture Highlights

### Core Components

- **Archetect**: Main orchestrator (`archetect-core/src/archetect/archetect.rs`) that coordinates archetype rendering
- **Archetype**: Represents a template project (`archetect-core/src/archetype/archetype.rs`) with manifest, directory structure, and rendering logic
- **Source**: Handles Git repositories and local directories as archetype sources (`archetect-core/src/source.rs`)
- **IoDriver**: Abstraction for user interaction (terminal prompts, file operations) defined in `archetect-api`

### Templating & Scripting

- Uses custom Jinja2-like templating engine in `archetect-templating`
- Rhai scripting engine integration for orchestration (`archetect-core/src/script/rhai/`)
- Rich set of template functions: case conversion, pluralization, file operations, prompts

### Key Features

- Smart string inflections (camelCase, snake_case, pluralization)
- Interactive prompts (text, select, multiselect, bool, editor)
- Git repository and local directory sources with caching
- Template inheritance and composition
- Configuration management with YAML files

## Testing

Tests are primarily in `archetect-core/tests/` with integration tests for prompts and utilities. The templating engine has extensive snapshot tests in `archetect-templating/tests/`.

## Build System

- Uses standard Cargo workspace
- Custom `xtask` for installation with optional static OpenSSL linking
- Cargo alias: `cargo xtask` runs build automation tasks