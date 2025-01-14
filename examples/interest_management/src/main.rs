#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

//! Run with
//! - `cargo run -- server`
//! - `cargo run -- client -c 1`
mod client;
mod protocol;
#[cfg(not(target_family = "wasm"))]
mod server;
mod shared;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::DefaultPlugins;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::format::FmtSpan;

use crate::client::MyClientPlugin;
#[cfg(not(target_family = "wasm"))]
use crate::server::MyServerPlugin;
use lightyear::netcode::{ClientId, Key};
use lightyear::prelude::TransportConfig;

#[cfg(target_family = "wasm")]
use wasm_bindgen_test::*;
#[cfg(target_family = "wasm")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        #[wasm_bindgen_test]
        fn wasm_client() {
            // NOTE: clap argument parsing does not work on WASM
            let client_id = rand::random::<u64>();
            let cli = Cli::Client {
                inspector: false,
                client_id,
                client_port: CLIENT_PORT,
                server_addr: Ipv4Addr::LOCALHOST,
                server_port: SERVER_PORT,
                transport: Transports::WebTransport,
            };
            let mut app = App::new();
            setup_client(&mut app, cli);
            app.run();
        }
    } else {
        #[tokio::main]
        async fn main() {
            let cli = Cli::parse();
            let mut app = App::new();
            setup(&mut app, cli).await;
            app.run();
        }

    }
}

// Use a port of 0 to automatically select a port
pub const CLIENT_PORT: u16 = 0;
pub const SERVER_PORT: u16 = 5000;
pub const PROTOCOL_ID: u64 = 0;

pub const KEY: Key = [0; 32];

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Transports {
    #[cfg(not(target_family = "wasm"))]
    Udp,
    WebTransport,
}

#[derive(Parser, PartialEq, Debug)]
enum Cli {
    #[cfg(not(target_family = "wasm"))]
    Server {
        #[arg(long, default_value = "false")]
        headless: bool,

        #[arg(short, long, default_value = "false")]
        inspector: bool,

        #[arg(short, long, default_value_t = SERVER_PORT)]
        port: u16,

        #[arg(short, long, value_enum, default_value_t = Transports::Udp)]
        transport: Transports,
    },
    Client {
        #[arg(short, long, default_value = "false")]
        inspector: bool,

        #[arg(short, long, default_value_t = 0)]
        client_id: u64,

        #[arg(long, default_value_t = CLIENT_PORT)]
        client_port: u16,

        #[arg(long, default_value_t = Ipv4Addr::LOCALHOST)]
        server_addr: Ipv4Addr,

        #[arg(short, long, default_value_t = SERVER_PORT)]
        server_port: u16,

        #[cfg_attr(not(target_family = "wasm"), arg(short, long, value_enum, default_value_t = Transports::Udp))]
        #[cfg_attr(target_family = "wasm", arg(short, long, value_enum, default_value_t = Transports::WebTransport))]
        transport: Transports,
    },
}

// the function is async because the server needs to load the certificates from a file
async fn setup(app: &mut App, cli: Cli) {
    match cli {
        #[cfg(not(target_family = "wasm"))]
        Cli::Server {
            headless,
            inspector,
            port,
            transport,
        } => {
            let server_plugin = server::create_plugin(port, transport).await;
            if !headless {
                app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>());
            } else {
                app.add_plugins(MinimalPlugins);
            }
            if inspector {
                app.add_plugins(WorldInspectorPlugin::new());
            }
            app.add_plugins(server_plugin);
        }
        Cli::Client { .. } => {
            setup_client(app, cli);
        }
    }
}

fn setup_client(app: &mut App, cli: Cli) {
    let Cli::Client {
        inspector,
        client_id,
        client_port,
        server_addr,
        server_port,
        transport,
    } = cli
    else {
        return;
    };
    let server_addr = SocketAddr::new(server_addr.into(), server_port);
    let client_plugin = client::create_plugin(client_id, client_port, server_addr, transport);

    // use the default bevy logger for now
    // (the lightyear logger doesn't handle wasm)
    app.add_plugins(DefaultPlugins.set(LogPlugin {
        level: Level::INFO,
        filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
    }));

    if inspector {
        app.add_plugins(WorldInspectorPlugin::new());
    }
    app.add_plugins(client_plugin);
}
