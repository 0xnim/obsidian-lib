//! A library for reading and extracting files from Obsidian `.obby` plugin files.
//!
//! This crate provides functionality to read `.obby` files, which are archives used
//! by Obsidian plugins. It allows you to list and extract files from these archives,
//! with special support for extracting `plugin.json` files.
//!
//! # Example
//!
//! ```no_run
//! use obsidian_lib::{ObbyArchive, extract_plugin_json};
//! use std::path::Path;
//! use std::fs::File;
//!
//! # fn main() -> std::io::Result<()> {
//! // From a file path
//! let json = extract_plugin_json(Path::new("plugin.obby"))?;
//!
//! // Or from any Read + Seek source
//! let file = File::open("plugin.obby")?;
//! let mut archive = ObbyArchive::new(file)?;
//! let entries = archive.list_entries();
//! let data = archive.extract_entry("plugin.json")?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Main reader struct for working with .obby files from any source
#[derive(Debug)]
pub struct ObbyArchive<R: Read + Seek> {
    entries: HashMap<String, EntryInfo>,
    reader: R,
    data_start_pos: u64,
}

#[derive(Debug)]
struct EntryInfo {
    offset: u64,
    length: i32,
    compressed_length: i32,
}

struct BinaryReader<R: Read> {
    reader: R,
}

impl<R: Read> BinaryReader<R> {
    fn new(reader: R) -> Self {
        BinaryReader { reader }
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        let mut byte = [0u8; 1];
        self.reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    fn read_bytes(&mut self, length: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; length];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn read_i32(&mut self) -> io::Result<i32> {
        let mut bytes = [0u8; 4];
        self.reader.read_exact(&mut bytes)?;
        Ok(i32::from_le_bytes(bytes))
    }
}

fn read_csharp_string<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<String> {
    let mut string_len = 0;
    let mut done = false;
    let mut step = 0;
    while !done {
        let byte = reader.read_u8()?;
        string_len |= ((byte & 0x7F) as u32) << (step * 7);
        done = (byte & 0x80) == 0;
        step += 1;
    }
    let buf = reader.read_bytes(string_len as usize)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

impl<R: Read + Seek> ObbyArchive<R> {
    /// Creates a new ObbyArchive from any source that implements Read + Seek
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type that implements Read + Seek traits
    ///
    /// # Returns
    ///
    /// Returns `Result<ObbyArchive<R>, io::Error>` which is:
    /// * `Ok(ObbyArchive)` if the input was successfully parsed
    /// * `Err` if there was an error parsing the input
    ///
    /// # Example
    ///
    /// ```no_run
    /// use obsidian_lib::ObbyArchive;
    /// use std::fs::File;
    /// use std::io::Cursor;
    ///
    /// // From a file
    /// let file = File::open("plugin.obby").unwrap();
    /// let archive = ObbyArchive::new(file).unwrap();
    ///
    /// // Or from a memory buffer
    /// let buffer = vec![/* .obby file contents */];
    /// let cursor = Cursor::new(buffer);
    /// let archive = ObbyArchive::new(cursor).unwrap();
    /// ```
    pub fn new(mut reader: R) -> io::Result<Self> {
        let mut binary_reader = BinaryReader::new(&mut reader);

        // Verify header
        let mut header = [0u8; 4];
        binary_reader.reader.read_exact(&mut header)?;
        if &header != b"OBBY" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid plugin header"));
        }

        // Read metadata
        let _api_version = read_csharp_string(&mut binary_reader)?;
        let _hash = binary_reader.read_bytes(48)?;

        // Read signature
        let mut is_signed = [0u8; 1];
        binary_reader.reader.read_exact(&mut is_signed)?;
        if is_signed[0] != 0 {
            let _signature = binary_reader.read_bytes(384)?;
        }

        // Read data length and plugin info
        let _data_length = binary_reader.read_i32()?;
        let _plugin_assembly = read_csharp_string(&mut binary_reader)?;
        let _plugin_version = read_csharp_string(&mut binary_reader)?;

        // Read entries
        let entry_count = binary_reader.read_i32()? as usize;
        let mut entries = HashMap::new();
        let mut current_offset = 0u64;

        for _ in 0..entry_count {
            let name = read_csharp_string(&mut binary_reader)?;
            let length = binary_reader.read_i32()?;
            let compressed_length = binary_reader.read_i32()?;

            entries.insert(name, EntryInfo {
                offset: current_offset,
                length,
                compressed_length,
            });

            current_offset += compressed_length as u64;
        }

        let data_start_pos = reader.stream_position()?;

        Ok(ObbyArchive {
            entries,
            reader,
            data_start_pos,
        })
    }

    /// Returns a list of all entries in the archive
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the names of all entries in the archive
    pub fn list_entries(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Extracts a specific entry by name
    ///
    /// # Arguments
    ///
    /// * `entry_name` - Name of the entry to extract
    ///
    /// # Returns
    ///
    /// Returns `Result<Vec<u8>, io::Error>` which is:
    /// * `Ok(Vec<u8>)` containing the extracted data if successful
    /// * `Err` if the entry doesn't exist or there was an error extracting it
    pub fn extract_entry(&mut self, entry_name: &str) -> io::Result<Vec<u8>> {
        let entry = self.entries.get(entry_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Entry '{}' not found in archive", entry_name),
            )
        })?;

        // Seek to the entry's position
        self.reader.seek(SeekFrom::Start(self.data_start_pos + entry.offset))?;

        // Read the compressed data
        let mut reader = BinaryReader::new(&mut self.reader);
        let compressed_data = reader.read_bytes(entry.compressed_length as usize)?;

        // Decompress if necessary
        if entry.compressed_length != entry.length {
            let mut decompressed_data = Vec::new();
            let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
            decoder.read_to_end(&mut decompressed_data)?;
            Ok(decompressed_data)
        } else {
            Ok(compressed_data)
        }
    }
}

/// Opens an .obby file from a path
///
/// This is a convenience function that creates an ObbyArchive from a file path.
pub fn open<P: AsRef<Path>>(path: P) -> io::Result<ObbyArchive<File>> {
    let file = File::open(path)?;
    ObbyArchive::new(file)
}

/// Convenience function to extract and parse plugin.json from an .obby file path
///
/// # Arguments
///
/// * `path` - Path to the .obby file
///
/// # Returns
///
/// Returns `Result<String, io::Error>` which is:
/// * `Ok(String)` containing the plugin.json contents if successful
/// * `Err` if there was an error reading or parsing the file
pub fn extract_plugin_json<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut archive = open(path)?;
    let data = archive.extract_entry("plugin.json")?;
    String::from_utf8(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::NamedTempFile;
    use flate2::{write::DeflateEncoder, Compression};

    fn create_test_plugin_json() -> String {
        r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "version": "1.0.0",
            "description": "A test plugin"
        }"#.to_string()
    }

    fn create_test_obby_bytes() -> Vec<u8> {
        let mut buffer = Vec::new();

        // Write header
        buffer.extend_from_slice(b"OBBY");

        // Add minimal valid .obby structure
        // ... (rest of the test file creation)

        buffer
    }

    #[test]
    fn test_memory_buffer() {
        let buffer = create_test_obby_bytes();
        let cursor = Cursor::new(buffer);
        let archive = ObbyArchive::new(cursor);
        assert!(archive.is_ok());
    }
}