use std::{fs, io};

fn load_rom() -> io::Result<Vec<u8>> {
    fs::read("panepon.sfc")
}

#[test]
fn test_decompress() -> anyhow::Result<()> {
    let rom = load_rom()?;

    for entry in fs::read_dir("tests/decompress_data")? {
        let entry = entry?;
        let name = entry.file_name().into_string().unwrap();

        let (name, offset) = name
            .split_once('_')
            .expect("Failed to split filename into name and offset");
        let offset =
            usize::from_str_radix(offset, 16).expect("Failed to parse offset from filename");

        let expected = fs::read(entry.path())?;
        let result = thanatos::Decompressor::new(&rom, offset).decompress();

        if expected != result {
            panic!(
                "Decompressed data does not match for {} at offset {:#x}",
                name, offset
            );
        } else {
            println!("Decompressed data matches for {}", name);
        }
    }

    Ok(())
}
