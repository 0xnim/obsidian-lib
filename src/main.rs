use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

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

fn unpack_plugin(file_path: &Path) -> io::Result<()> {
    let file = File::open(file_path)?;
    let mut reader = BinaryReader::new(file);
    println!("File opened: {:?}", file_path);

    // Verify header
    let mut header = [0u8; 4];
    reader.reader.read_exact(&mut header)?;
    if &header != b"OBBY" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid plugin header"));
    }
    println!("Header verified: {:?}", header);

    // Read API version
    let api_version = read_csharp_string(&mut reader)?;
    println!("API Version: {}", api_version);

    // Read hash
    let hash = reader.read_bytes(48)?;
    println!("Hash: {:x?}", hash);

    // Read signature flag and optional signature
    let mut is_signed = [0u8; 1];
    reader.reader.read_exact(&mut is_signed)?;
    let is_signed = is_signed[0] != 0;
    println!("Is Signed: {}", is_signed);

    let signature = if is_signed {
        let sig = reader.read_bytes(384)?;
        println!("Signature: {:x?}", sig);
        Some(sig)
    } else {
        None
    };

    // Read data length
    let data_length = reader.read_i32()?;
    println!("Data Length: {}", data_length);

    // Read plugin assembly and version
    let plugin_assembly = read_csharp_string(&mut reader)?;
    let plugin_version = read_csharp_string(&mut reader)?;
    println!("Plugin Assembly: {}", plugin_assembly);
    println!("Plugin Version: {}", plugin_version);

    // Read entry count
    let entry_count = reader.read_i32()? as usize;
    println!("Entry Count: {}", entry_count);

    // Read entries
    let mut entries = HashMap::new();
    for _ in 0..entry_count {
        let name = read_csharp_string(&mut reader)?;
        let length = reader.read_i32()?;
        let compressed_length = reader.read_i32()?;
        println!("Entry: {}, Length: {}, Compressed Length: {}", name, length, compressed_length);
        entries.insert(name, (length, compressed_length));
    }

    // Read and process entry data
    for (name, (length, compressed_length)) in &entries {
        println!("Processing entry: {}", name);
        let compressed_data = reader.read_bytes(*compressed_length as usize)?;

        // Optionally decompress data if needed
        let data = if *compressed_length != *length {
            println!("Decompressing data for entry: {}", name);
            let mut decompressed_data = Vec::new();
            let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
            decoder.read_to_end(&mut decompressed_data)?;
            decompressed_data
        } else {
            compressed_data
        };

        println!("Decompressed {} bytes for entry: {}", data.len(), name);
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let path = Path::new("./test_dir/ObsidianPlugin.obby");
    match unpack_plugin(&path) {
        Ok(_) => {
            println!("Plugin unpacked successfully.");
        }
        Err(e) => {
            eprintln!("Error unpacking plugin: {}", e);
        }
    }
    Ok(())
}
