use std::io::SeekFrom;
use std::io::Seek;
use obsidian_lib::ObbyArchive;
use std::fs::File;
use std::io::{self, Read, Cursor};

fn main() -> io::Result<()> {
    // Example 1: Reading into memory buffer
    println!("Example 1: Memory Buffer");
    {
        // Read the entire .obby file into memory
        let mut file = File::open("./test_dir/ObsidianPlugin.obby")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Create archive from memory buffer
        let cursor = Cursor::new(buffer);
        let mut archive = ObbyArchive::new(cursor)?;

        // List all entries
        println!("Entries in archive:");
        for entry in archive.list_entries() {
            println!("- {}", entry);
        }

        // Extract plugin.json
        let json_data = archive.extract_entry("plugin.json")?;
        let json_string = String::from_utf8_lossy(&json_data);
        println!("\nPlugin JSON contents:\n{}", json_string);
    }

    // Example 2: Simulated network stream (using a local file for demonstration)
    println!("\nExample 2: Network Stream Simulation");
    {
        // In a real network scenario, you might have something like:
        // let response = reqwest::blocking::get("https://example.com/plugin.obby")?;
        // let reader = response.bytes()?;

        // For demonstration, we'll create a "stream" from the local file
        struct ChunkedReader {
            file: File,
            position: u64,
        }

        impl Read for ChunkedReader {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.file.read(buf)
            }
        }

        impl Seek for ChunkedReader {
            fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
                self.position = self.file.seek(pos)?;
                Ok(self.position)
            }
        }

        let file = File::open("./test_dir/ObsidianPlugin.obby")?;
        let reader = ChunkedReader {
            file,
            position: 0,
        };

        // Create archive from our "stream"
        let mut archive = ObbyArchive::new(reader)?;

        // List all entries
        println!("Entries in archive:");
        for entry in archive.list_entries() {
            println!("- {}", entry);
        }

        // Extract plugin.json
        let json_data = archive.extract_entry("plugin.json")?;
        let json_string = String::from_utf8_lossy(&json_data);
        println!("\nPlugin JSON contents:\n{}", json_string);
    }

    Ok(())
}