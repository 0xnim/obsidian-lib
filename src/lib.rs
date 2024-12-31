use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub struct ObbyReader {
    entries: HashMap<String, EntryInfo>,
    file: File,
    data_start_pos: u64,
}

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

impl ObbyReader {
    /// Opens an .obby file and reads its metadata
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut reader = BinaryReader::new(&mut file);

        // Verify header
        let mut header = [0u8; 4];
        reader.reader.read_exact(&mut header)?;
        if &header != b"OBBY" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid plugin header"));
        }

        // Read metadata
        let _api_version = read_csharp_string(&mut reader)?;
        let _hash = reader.read_bytes(48)?;

        // Read signature
        let mut is_signed = [0u8; 1];
        reader.reader.read_exact(&mut is_signed)?;
        if is_signed[0] != 0 {
            let _signature = reader.read_bytes(384)?;
        }

        // Read data length and plugin info
        let _data_length = reader.read_i32()?;
        let _plugin_assembly = read_csharp_string(&mut reader)?;
        let _plugin_version = read_csharp_string(&mut reader)?;

        // Read entries
        let entry_count = reader.read_i32()? as usize;
        let mut entries = HashMap::new();
        let mut current_offset = 0u64;

        for _ in 0..entry_count {
            let name = read_csharp_string(&mut reader)?;
            let length = reader.read_i32()?;
            let compressed_length = reader.read_i32()?;

            entries.insert(name, EntryInfo {
                offset: current_offset,
                length,
                compressed_length,
            });

            current_offset += compressed_length as u64;
        }

        let data_start_pos = file.stream_position()?;

        Ok(ObbyReader {
            entries,
            file,
            data_start_pos,
        })
    }

    /// Returns a list of all entries in the archive
    pub fn list_entries(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Extracts a specific entry by name
    pub fn extract_entry(&mut self, entry_name: &str) -> io::Result<Vec<u8>> {
        let entry = self.entries.get(entry_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Entry '{}' not found in archive", entry_name),
            )
        })?;

        // Seek to the entry's position
        self.file.seek(SeekFrom::Start(self.data_start_pos + entry.offset))?;

        // Read the compressed data
        let mut reader = BinaryReader::new(&mut self.file);
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

// Example function to extract and parse plugin.json
pub fn extract_plugin_json(path: &Path) -> io::Result<String> {
    let mut reader = ObbyReader::open(path)?;
    let data = reader.extract_entry("plugin.json")?;
    String::from_utf8(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_obby() -> io::Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;

        // Write header
        file.write_all(b"OBBY")?;

        // Write minimal valid .obby structure
        // This is a simplified test file structure

        Ok(file)
    }

    #[test]
    fn test_open_invalid_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"INVALID").unwrap();

        let result = ObbyReader::open(file.path());
        assert!(result.is_err());
    }
}