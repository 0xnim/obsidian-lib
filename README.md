# obsidian-lib

[![Crates.io](https://img.shields.io/crates/v/obsidian-lib.svg)](https://crates.io/crates/obsidian-lib)
[![Documentation](https://docs.rs/obsidian-lib/badge.svg)](https://docs.rs/obsidian-lib)

A Rust library for reading and extracting files from Obsidian `.obby` plugin files.

## Features

- Read `.obby` file metadata
- List all entries in an `.obby` file
- Extract specific files from the archive
- Handles both compressed and uncompressed entries
- Convenience function for extracting `plugin.json`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
obsidian-lib = "0.1.0"
```

## Usage

CLI: 
`obsidian-lib ./ObsidianPlugin.obby`

You can find an example plugin on [![Harbr](https://harbr.dev/plugin/obsidian-vault)]


```rust
use obsidian_lib::{ObbyReader, extract_plugin_json};
use std::path::Path;

// Extract just plugin.json
let json = extract_plugin_json(Path::new("path/to/plugin.obby"))?;
println!("Plugin JSON: {}", json);

// Or work with the archive more generally
let mut reader = ObbyReader::open(Path::new("path/to/plugin.obby"))?;

// List all entries
println!("Available entries: {:?}", reader.list_entries());

// Extract specific entry
let data = reader.extract_entry("plugin.json")?;
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
