fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Keep the bundled resource copy of the proto file in sync with the
    // canonical source automatically, so the two can never silently drift
    // apart again (this caused two separate debugging sessions before
    // this fix was added).
    std::fs::copy(
        "proto/ros_engine.proto",
        "resources/devcontainer/proto/ros_engine.proto",
    )?;

    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&["proto/ros_engine.proto"], &["proto"])?;
    tauri_build::build();
    Ok(())
}