fn main() {
    tonic_build::configure()
        .out_dir("src/generated")
        .file_descriptor_set_path("src/generated/lwd_descriptor.bin")
        .compile(
            &["proto/service.proto", "proto/compact_formats.proto"],
            &["proto"],
        )
        .unwrap();
}

