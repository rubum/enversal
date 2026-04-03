//! Build script to hook into the Cargo compile process.
//! Used exclusively to compile `.proto` interface definitions with `tonic_build`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/control.proto")?;
    Ok(())
}
