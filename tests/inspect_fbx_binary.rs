#[test]
#[ignore]
fn inspect_exported_fbx_binary() {
    use std::io::Read;

    let path = "assets/exports/Armature-ArmatureAction.fbx";

    println!("\n=== Raw FBX Binary Inspection ===\n");
    println!("File: {}\n", path);

    if let Ok(mut file) = std::fs::File::open(path) {
        let mut buffer = [0u8; 27];
        if file.read_exact(&mut buffer).is_ok() {
            println!("FBX Header:");
            println!("  Signature: {}", String::from_utf8_lossy(&buffer[0..23]));

            let version_bytes = &buffer[23..27];
            let version = u32::from_le_bytes([
                version_bytes[0],
                version_bytes[1],
                version_bytes[2],
                version_bytes[3],
            ]);
            println!("  Version: {}", version);
        }

        let mut buf = vec![0u8; 8192];
        let mut total_read = 0;
        println!("\nSearching for 'Model' and 'NodeAttribute' nodes...");

        loop {
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let content = String::from_utf8_lossy(&buf[..n]);
                    let model_count = content.matches("Model").count();
                    let attr_count = content.matches("NodeAttribute").count();

                    if model_count > 0 {
                        println!(
                            "  Found {} 'Model' strings in chunk at offset {}",
                            model_count, total_read
                        );
                    }
                    if attr_count > 0 {
                        println!(
                            "  Found {} 'NodeAttribute' strings in chunk at offset {}",
                            attr_count, total_read
                        );
                    }

                    total_read += n;
                }
                Err(e) => {
                    println!("Error reading: {}", e);
                    break;
                }
            }
        }

        println!("\nTotal file size: {} bytes", total_read);
    } else {
        println!("Failed to open file");
    }
}
