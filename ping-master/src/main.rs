use std::{
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use config::WorkerConfig;
use futures::future::join_all;
use once_cell::sync::OnceCell;
use ping_proto::{M2WMessage, W2MMessage};
use serde::{Deserialize, Serialize};
use serde_json::{de::IoRead, StreamDeserializer};
use thiserror::Error;
use tokio::sync::{mpsc::unbounded_channel, oneshot::channel};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use crate::config::Config;

mod config;

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Slash16State {
    Reserved,
    Skipped,
    Scheduled,
    Pending,
    Completed,
    Errored,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value = "ping-config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    if let Err(e) = init().await {
        error!("{e}");
        std::process::exit(1);
    }
}

#[derive(Debug, Error)]
enum InitError {
    #[error("Failed to load config file: {0}")]
    Config(#[from] config::ConfigError),
}

async fn init() -> Result<(), InitError> {
    let args = Args::parse();

    let config = CONFIG.get_or_try_init(|| config::load_config(args.config))?;

    let WorkerConfig::Local(worker_config) = &config.workers else {
        todo!("Remote worker config");
    };

    let mut connect_receivers = Vec::with_capacity(worker_config.count as usize);

    let (w2m_sender, mut w2m_receiver) = unbounded_channel::<(u16, W2MMessage)>();

    for id in 0..worker_config.count {
        let w2m_sender = w2m_sender.clone();

        let (connect_sender, connect_receiver) = channel();
        connect_receivers.push(connect_receiver);

        tokio::task::spawn_blocking(move || {
            /* Create unix domain socket */

            let socket_path = PathBuf::from(format!("./sockets/{}.sock", id));

            std::fs::create_dir_all(socket_path.parent().unwrap())
                .expect("Falied to create sockets directory");

            let listener = match UnixListener::bind(&socket_path) {
                Ok(value) => value,
                Err(e) => {
                    error!("[Worker {id}] Failed to bind unix socket listener: {e:?}");
                    std::process::exit(1);
                }
            };

            /* Spawn Child Process */

            Command::new("ping-worker")
                .arg("--socket")
                .arg(&socket_path)
                .arg("--max-connections")
                .arg(format!("{}", worker_config.max_connections))
                .arg("--retry-limit")
                .arg(format!("{}", worker_config.retry_limit))
                .arg("--timeout-ms")
                .arg(format!("{}", worker_config.timeout))
                .spawn()
                .expect("Failed to spawn worker process");

            /* Accept connection from child process */

            let (stream, _) = listener.accept().unwrap_or_else(|e| {
                error!("[Worker {id}] failed to connect: {e:?}");
                std::process::exit(1);
            });

            info!("[Worker {id}] connected successfully!");

            /* Split our socket so we can read and write independently */

            let (mut writer, reader) = (
                stream.try_clone().expect("Failed to clone unix stream"),
                stream,
            );

            /* Spawn a task to send M2W messages */

            let (m2w_sender, mut m2w_receiver) = unbounded_channel::<M2WMessage>();

            tokio::task::spawn_blocking(move || {
                while let Some(m2w_message) = m2w_receiver.blocking_recv() {
                    if let Err(e) = serde_json::to_writer(&mut writer, &m2w_message) {
                        error!("[Worker {id}] Failed to write into socket ({e:?})");
                        std::process::exit(1);
                    };
                }
            });

            connect_sender.send(m2w_sender).unwrap();

            /* Send any W2M messages we get over the channel */

            let json_stream = StreamDeserializer::new(IoRead::new(reader));

            for message in json_stream {
                let Ok(message) = message else {
                    break;
                };

                w2m_sender.send((id, message)).unwrap();
            }

            error!("[Worker {id}] disconnected!");
            std::process::exit(1);
        });
    }

    /* Wait for all workers to connect */

    // TODO add timeout
    let m2w_senders = join_all(connect_receivers)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    /* Handle W2M messages as they come in */

    tokio::spawn(async move {
        while let Some((id, w2m_message)) = w2m_receiver.recv().await {
            debug!("[Worker {id}] Received W2M message: {:?}", w2m_message);
        }
    });

    Ok(())
}
