use std::io;

fn main() -> Result<(), io::Error> {
    tonic_build::configure().build_server(true).compile(
        &[
            "perf/helper/proto/common/v1/uuid.proto",
            "perf/helper/proto/direct/v1/helper.proto",
        ],
        &["rs-perf-helper-proto"],
    )?;
    Ok(())
}
