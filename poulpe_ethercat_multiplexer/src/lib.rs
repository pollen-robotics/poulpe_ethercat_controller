pub mod pb {
    tonic::include_proto!("poulpe");
}

pub mod client;
pub use client::PoulpeRemoteClient;
