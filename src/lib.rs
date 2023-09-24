pub use tonic;

pub mod rpc {
    pub mod perf {
        pub mod helper {
            pub mod proto {
                pub mod common {
                    pub mod v1 {
                        tonic::include_proto!("perf.helper.proto.common.v1");
                    }
                }
                pub mod direct {
                    pub mod v1 {
                        tonic::include_proto!("perf.helper.proto.direct.v1");
                    }
                }
            }
        }
    }
}

pub mod uuid;

pub mod convert;

pub mod direct;
