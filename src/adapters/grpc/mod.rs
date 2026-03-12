pub mod health_impl;
pub mod server;

pub mod core {
    tonic::include_proto!("nexus.core");
}
