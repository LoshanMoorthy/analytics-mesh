use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("generated");
    fs::create_dir_all(&out_dir)?;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let proto_dir    = manifest_dir.join("proto");
    let proto_file   = proto_dir.join("events.proto");

    tonic_build::configure()
        .build_server(true)
        .out_dir(&out_dir)
        .type_attribute(
            "analytics.Event",
            "#[derive(::serde::Serialize, ::serde::Deserialize)]",
        )
        .type_attribute(
            "analytics.Empty",
            "#[derive(::serde::Serialize, ::serde::Deserialize)]",
        )
        .compile(
            &[ proto_file.to_str().unwrap() ],
            &[ proto_dir.to_str().unwrap()  ],
        )?;

    Ok(())
}
