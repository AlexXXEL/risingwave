// Copyright 2022 Singularity Data
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(trait_alias)]
#![feature(binary_heap_drain_sorted)]
#![feature(generators)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![cfg_attr(coverage, feature(no_coverage))]

#[macro_use]
extern crate tracing;

pub mod rpc;
pub mod server;

use clap::clap_derive::ArgEnum;
use clap::Parser;

#[derive(Debug, Clone, ArgEnum)]
pub enum AsyncStackTraceOption {
    Off,
    On, // default
    Verbose,
}

/// Command-line arguments for compute-node.
#[derive(Parser, Clone, Debug)]
pub struct ComputeNodeOpts {
    // TODO: rename to listen_address and separate out the port.
    #[clap(long, default_value = "127.0.0.1:5688")]
    pub host: String,

    // Optional, we will use listen_address if not specified.
    #[clap(long)]
    pub client_address: Option<String>,

    #[clap(long, default_value = "hummock+memory")]
    pub state_store: String,

    #[clap(long, default_value = "127.0.0.1:1222")]
    pub prometheus_listener_addr: String,

    /// Used for control the metrics level, similar to log level.
    /// 0 = close metrics
    /// >0 = open metrics
    #[clap(long, default_value = "0")]
    pub metrics_level: u32,

    #[clap(long, default_value = "http://127.0.0.1:5690")]
    pub meta_address: String,

    /// Enable reporting tracing information to jaeger.
    #[clap(long)]
    pub enable_jaeger_tracing: bool,

    /// Enable async stack tracing for risectl.
    #[clap(long, arg_enum, default_value_t = AsyncStackTraceOption::On)]
    pub async_stack_trace: AsyncStackTraceOption,

    /// Path to file cache data directory.
    /// Left empty to disable file cache.
    #[clap(long, default_value = "")]
    pub file_cache_dir: String,

    /// Endpoint of the connector node
    #[clap(long, env = "CONNECTOR_RPC_ENDPOINT")]
    pub connector_rpc_endpoint: Option<String>,

    /// Te path of `risingwave.toml` configuration file.
    ///
    /// If empty, default configuration values will be used.
    ///
    /// Note that internal system parameters should be defined in the configuration file at
    /// [`risingwave_common::config`] instead of command line arguments.
    #[clap(long, default_value = "")]
    pub config_path: String,
}

use std::future::Future;
use std::pin::Pin;

use crate::server::compute_node_serve;

/// Start compute node
pub fn start(opts: ComputeNodeOpts) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    // WARNING: don't change the function signature. Making it `async fn` will cause
    // slow compile in release mode.
    Box::pin(async move {
        tracing::info!("Compute node options: {:?}", opts);

        let listen_address = opts.host.parse().unwrap();
        tracing::info!("Server Listening at {}", listen_address);

        let client_address = opts
            .client_address
            .as_ref()
            .unwrap_or_else(|| {
                tracing::warn!("Client address is not specified, defaulting to host address");
                &opts.host
            })
            .parse()
            .unwrap();
        tracing::info!("Client address is {}", client_address);

        let (join_handle_vec, _shutdown_send) =
            compute_node_serve(listen_address, client_address, opts).await;

        for join_handle in join_handle_vec {
            join_handle.await.unwrap();
        }
    })
}
