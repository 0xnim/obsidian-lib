use std::path::Path;
use obsidian_lib::{extract_plugin_json, ObbyReader};

fn main() -> std::io::Result<()> {
    // To get just plugin.json
    let json = extract_plugin_json(Path::new("./test_dir/ObsidianPlugin.obby"))?;
    println!("Plugin JSON: {}", json);

    // Or to work with the archive more generally
    let mut reader = ObbyReader::open(Path::new("./test_dir/ObsidianPlugin.obby"))?;

    // List all entries
    println!("Available entries: {:?}", reader.list_entries());

    // Extract specific entry
    let data = reader.extract_entry("plugin.json")?;

    println!("Plugin JSON: {}", String::from_utf8(data).unwrap());

    Ok(())
}