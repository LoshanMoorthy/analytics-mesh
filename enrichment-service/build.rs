use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("generated");
    fs::create_dir_all(&out_dir)?;

    let proto_dir  = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");
    let proto_file = proto_dir.join("events.proto");

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&[proto_file], &[proto_dir])?;

    Ok(())
}