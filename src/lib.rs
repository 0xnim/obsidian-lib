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
///
/// The `ObbyArchive` struct is used to represent an archive file in the `.obby` format,
/// which is used by Obsidian plugins. It allows for listing and extracting the files
/// within the archive, and it handles both reading the metadata and the compressed file data.
///
/// # Type Parameters
///
/// * `R`: A type that implements both `Read` and `Seek` traits, such as `std::fs::File` or `std::io::Cursor`.
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
    /// Creates a new instance of `BinaryReader`
    ///
    /// This function initializes a new binary reader from the provided `reader`.
    ///
    /// # Arguments
    ///
    /// * `reader` - The reader to be used for reading bytes.
    ///
    /// # Returns
    ///
    /// A new `BinaryReader` instance.
    fn new(reader: R) -> Self {
        BinaryReader { reader }
    }

    /// Reads a single byte from the reader
    ///
    /// # Returns
    ///
    /// A `Result` containing the byte if successful, or an error if reading fails.
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut byte = [0u8; 1];
        self.reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    /// Reads a specific number of bytes from the reader
    ///
    /// # Arguments
    ///
    /// * `length` - The number of bytes to read.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` of the read bytes if successful, or an error if reading fails.
    fn read_bytes(&mut self, length: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; length];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Reads a 32-bit integer from the reader
    ///
    /// # Returns
    ///
    /// A `Result` containing the integer if successful, or an error if reading fails.
    fn read_i32(&mut self) -> io::Result<i32> {
        let mut bytes = [0u8; 4];
        self.reader.read_exact(&mut bytes)?;
        Ok(i32::from_le_bytes(bytes))
    }
}

/// Reads a C#-style encoded string from the reader
///
/// The string is encoded with a length prefix in variable-length encoding, where the length
/// is encoded using 7-bit chunks.
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
    /// Creates a new `ObbyArchive` from any source that implements `Read` and `Seek`
    ///
    /// This function reads the `.obby` file format and extracts its metadata and entry
    /// information. It verifies the file header and sets up the internal structure to allow
    /// for extracting files from the archive.
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type that implements the `Read` and `Seek` traits (e.g., `File`, `Cursor`).
    ///
    /// # Returns
    ///
    /// A `Result` containing either the created `ObbyArchive` instance or an `io::Error` if there was an issue reading the archive.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use obsidian_lib::ObbyArchive;
    /// use std::fs::File;
    ///
    /// let file = File::open("plugin.obby").unwrap();
    /// let archive = ObbyArchive::new(file).unwrap();
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

        // Read signature (if present)
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
    /// This function returns a vector of the entry names in the `.obby` archive.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the names of all entries.
    pub fn list_entries(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Extracts a specific entry by name
    ///
    /// This function extracts a specific entry from the `.obby` archive based on its name.
    /// The entry data is returned as a vector of bytes.
    ///
    /// # Arguments
    ///
    /// * `entry_name` - The name of the entry to extract.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` of the extracted entry's data if successful, or an `io::Error` if there was an issue extracting it.
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
/// This is a convenience function that creates an `ObbyArchive` from a file path.
///
/// # Arguments
///
/// * `path` - The path to the `.obby` file.
pub fn open<P: AsRef<Path>>(path: P) -> io::Result<ObbyArchive<File>> {
    let file = File::open(path)?;
    ObbyArchive::new(file)
}

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use std::io::Cursor;
use js_sys::Uint8Array;

/// A wrapper struct for the WebAssembly environment to interact with `.obby` files
///
/// This struct provides a WASM-compatible interface for working with `.obby` archives.
#[wasm_bindgen]
pub struct WasmObbyArchive {
    inner: ObbyArchive<Cursor<Vec<u8>>>
}

#[wasm_bindgen]
impl WasmObbyArchive {
    #[wasm_bindgen(constructor)]
    /// Creates a new `WasmObbyArchive` instance from a byte buffer
    ///
    /// # Arguments
    ///
    /// * `buffer` - A byte slice representing the `.obby` file contents.
    ///
    /// # Returns
    ///
    /// A `WasmObbyArchive` instance.
    pub fn new(buffer: &[u8]) -> Result<WasmObbyArchive, JsValue> {
        let cursor = Cursor::new(buffer.to_vec());
        let inner = ObbyArchive::new(cursor)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(WasmObbyArchive { inner })
    }

    #[wasm_bindgen]
    /// Lists all entries in the `.obby` archive
    ///
    /// # Returns
    ///
    /// A JavaScript array of strings representing the names of all entries.
    pub fn list_entries(&self) -> Box<[JsValue]> {
        self.inner
            .list_entries()
            .into_iter()
            .map(JsValue::from)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    #[wasm_bindgen]
    /// Extracts a specific entry by name
    ///
    /// # Arguments
    ///
    /// * `entry_name` - The name of the entry to extract.
    ///
    /// # Returns
    ///
    /// A `Uint8Array` containing the entry's data.
    pub fn extract_entry(&mut self, entry_name: &str) -> Result<Uint8Array, JsValue> {
        let data = self.inner
            .extract_entry(entry_name)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Uint8Array::from(&data[..]))
    }

    #[wasm_bindgen]
    /// Extracts and returns the contents of the `plugin.json` file from the `.obby` archive
    ///
    /// # Returns
    ///
    /// A `Result<String, JsValue>` containing the parsed JSON string if successful.
    pub fn extract_plugin_json(&mut self) -> Result<String, JsValue> {
        let data = self.extract_entry("plugin.json")?;
        let text = String::from_utf8(data.to_vec())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(text)
    }
}

/// Convenience function to extract and parse the `plugin.json` file from an `.obby` archive
///
/// This function opens the `.obby` file, extracts the `plugin.json` entry, and returns
/// the contents of the file as a `String`.
///
/// # Arguments
///
/// * `path` - Path to the `.obby` file.
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

    fn load_test_obby_bytes() -> Vec<u8> {
        let mut file = File::open("test_dir/ObsidianPlugin.obby").unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();

        buffer
    }

    #[test]
    fn test_memory_buffer() {
        let buffer = load_test_obby_bytes();
        let cursor = Cursor::new(buffer);
        let archive = ObbyArchive::new(cursor);
        assert!(archive.is_ok());
    }
}
