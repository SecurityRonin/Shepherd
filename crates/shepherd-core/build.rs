fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["proto/iterm2-api.proto"], &["proto/"])?;
    Ok(())
}
