use std::io;

fn main() -> Result<(), io::Error> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(
            &[
                "perf/helper/proto/common/v1/uuid.proto",
                "perf/helper/proto/common/v1/retry.proto",
                "perf/helper/proto/direct/v1/helper.proto",
                "perf/helper/proto/indirect/v1/helper.proto",
                "perf/helper/proto/buffer/v1/helper.proto",
            ],
            &["rs-perf-helper-proto"],
        )?;
    Ok(())
}
