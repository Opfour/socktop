fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the protoc binary path to use the vendored version for CI compatibility
    // SAFETY: We're only setting PROTOC in a build script environment, which is safe
    unsafe {
        std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    }
    
    prost_build::compile_protos(&["processes.proto"], &["."])?;
    Ok(())
}
