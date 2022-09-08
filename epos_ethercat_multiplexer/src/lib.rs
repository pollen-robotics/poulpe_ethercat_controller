pub mod pb {
    tonic::include_proto!("epos");
}

mod client;
pub use client::EposRemoteClient;
