mod network;

use crate::network::HTTP;
use crate::network::response::RaftMetrics;
use anyhow::bail;
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Address of any node in the cluster
    #[arg(required = true, short, long, default_value = "127.0.0.1:8000")]
    server: String,

    /// Command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize the cluster
    Init {
        /// Node list, format as "id=ip:port", e.g. "1=127.0.0.1:8000"
        #[arg(required = true, value_parser = parse_node)]
        nodes: Vec<(u64, String)>,
    },
    /// Add a learner node to the cluster
    AddLearner {
        /// Node ID
        #[arg(required = true, value_parser = parse_node)]
        node: (u64, String),
    },
    /// Promote some learner node to a full member, must call "add-learner" first
    Promote {
        /// One or more node IDs
        #[arg(required = true)]
        node_ids: Vec<u64>,
    },
    /// Remove a node from the cluster
    RemoveNode {
        /// Node ID
        #[arg(required = true)]
        node_id: u64,
    },
    /// Get cluster status
    Status,
    /// Monitor cluster status
    Monitor {
        /// Monitoring interval (seconds)
        #[arg(short, long, default_value_t = 5)]
        interval: u64,
    },
}

fn parse_node(s: &str) -> Result<(u64, String), String> {
    let parts: Vec<&str> = s.split('=').collect();
    if parts.len() != 2 {
        return Err("Node format should be 'id=ip:port'".to_string());
    }
    match u64::from_str(parts[0]) {
        Ok(id) => Ok((id, parts[1].to_string())),
        Err(_) => Err("Invalid node ID, node ID should be a positive integer".to_string()),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match &args.command {
        Commands::Init { nodes } => {
            init_cluster(&args.server, nodes).await?;
        }
        Commands::AddLearner { node } => {
            add_learner(&args.server, node).await?;
        }
        Commands::Promote { node_ids } => {
            promote_nodes(&args.server, node_ids).await?;
        }
        Commands::RemoveNode { node_id } => {
            remove_node(&args.server, *node_id).await?;
        }
        Commands::Status => {
            get_status(&args.server).await?;
        }
        Commands::Monitor { interval } => {
            monitor_cluster(&args.server, *interval).await?;
        }
    }

    Ok(())
}

fn build_url(server: &str, path: &str) -> String {
    format!("http://{}/api/cluster{}", server, path)
}

async fn init_cluster(server: &str, nodes: &Vec<(u64, String)>) -> anyhow::Result<()> {
    println!(
        "initializing cluster, nodes: \n {}",
        nodes
            .iter()
            .map(|(id, addr)| format!("Node ID: {}, Address: {}", id, addr))
            .collect::<Vec<_>>()
            .join("\n ")
    );
    match HTTP.post::<String>(build_url(server, "/init"), nodes).await {
        Ok(res) => {
            // SAFE: res always a string.
            println!(" ✅ {}", res.unwrap());
        }
        Err(e) => {
            println!("  ❌ Failed to initialize cluster: {}", e);
        }
    }

    Ok(())
}

async fn add_learner(server: &str, node: &(u64, String)) -> anyhow::Result<()> {
    println!("Adding learner: Node ID: {}, Address: {}", node.0, node.1);
    match HTTP
        .post::<Value>(build_url(server, "/add-learner"), node)
        .await
    {
        Ok(_) => {
            println!(" ✅ Learner added successfully");
        }
        Err(e) => {
            println!(" ❌ Failed to initialize cluster: {}", e);
        }
    }
    Ok(())
}

async fn change_membership(server: &str, node_ids: &Vec<u64>) -> anyhow::Result<()> {
    HTTP.post::<Value>(build_url(server, "/change-membership"), node_ids)
        .await?;
    Ok(())
}

async fn promote_nodes(server: &str, node_ids: &Vec<u64>) -> anyhow::Result<()> {
    let status = get_status(server).await?;
    let exiting_node_ids = status
        .membership_config
        .membership
        .configs
        .first()
        .cloned()
        .unwrap_or(Vec::new());
    let ids = [exiting_node_ids.as_slice(), node_ids.as_slice()].concat();
    for id in node_ids.iter() {
        if exiting_node_ids.contains(id) {
            bail!(" ❌ Node {} is already a member of the cluster", id);
        }
    }
    match change_membership(server, &ids).await {
        Ok(_) => {
            println!(
                " ✅ Nodes {:?} have been promoted to regular nodes and can now participate in voting",
                node_ids
            );
        }
        Err(e) => {
            println!(" ❌ Failed to promote nodes {:?}: {}", node_ids, e);
        }
    }
    Ok(())
}
async fn remove_node(server: &str, node_id: u64) -> anyhow::Result<()> {
    let status = get_status(server).await?;
    let mut exiting_node_ids = status
        .membership_config
        .membership
        .configs
        .first()
        .cloned()
        .unwrap_or(Vec::new());
    if !exiting_node_ids.contains(&node_id) {
        println!(
            " ⚠️  Node {} does not exist in the cluster, current cluster nodes: {:?}",
            node_id, exiting_node_ids
        );
        return Ok(());
    }
    exiting_node_ids.retain(|id| *id != node_id);
    match change_membership(server, &exiting_node_ids).await {
        Ok(_) => {
            println!(" ✅ Node {} has been removed", node_id);
        }
        Err(e) => {
            println!(" ❌ Failed to remove node {}: {}", node_id, e);
        }
    }
    Ok(())
}

async fn get_status(server: &str) -> anyhow::Result<RaftMetrics> {
    match HTTP
        .get::<RaftMetrics>(build_url(server, "/metrics"), None::<String>)
        .await
    {
        Ok(res) => {
            if res.is_none() {
                bail!("Failed to get cluster status, server returned empty");
            }
            Ok(res.unwrap())
        }
        Err(e) => {
            println!(" ❌ Failed to get cluster status: {}", e);
            bail!(e);
        }
    }
}

#[rustfmt::skip]
fn print_status(metrics: &RaftMetrics) {
    println!("┌────────────────────────────────────────────────────────────────┐");
    println!("│                        Cluster Status                          │");
    println!("├────────────────────────────────────────────────────────────────┤");
    println!("│ Current Node ID               : {:<30} │", metrics.id);
    println!("│ Current Node Status           : {:<30} │", metrics.state);
    println!("│ Current Node Term             : {:<30} │", metrics.current_term);
    println!("│ Leader                        : {:<30} │", metrics.current_leader.unwrap_or(0));
    println!("│ Last Log Index                : {:<30} │", metrics.last_log_index.unwrap_or(0));
    println!("│ Last Applied Index            : {:<30} │", if let Some(last_applied) = &metrics.last_applied{last_applied.index} else{0});
    println!("│ Communication delay           : {:<30} │", metrics.millis_since_quorum_ack.map(|x| format!("{} ms", x)).unwrap_or("-".to_string()));
    println!("│                                                                │");
    println!("│ Members:                                                       │");

    for (id, node) in &metrics.membership_config.membership.nodes {
        println!("│   - Node {}                    : {:<30} │", id, node.addr);
    }

    println!("│                                                                │");
    println!("│ Replication:                                                   │");

    if let Some(replications) = &metrics.replication{
        for (id, replication) in replications {
            println!("│   - Node {}                    : Index {:<24} │", id, replication.clone().map(|r| r.index.to_string()).unwrap_or("N/A".to_string()));
        }
    }


    println!("└────────────────────────────────────────────────────────────────┘");
}

async fn monitor_cluster(server: &str, interval: u64) -> anyhow::Result<()> {
    loop {
        match get_status(server).await {
            Ok(status) => {
                print_status(&status);
            }
            Err(e) => {
                println!("Failed to get cluster status: {}", e);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
        if cfg!(target_os = "windows") {
            std::process::Command::new("cmd")
                .args(["/C", "cls"])
                .status()?;
        } else {
            std::process::Command::new("clear").status()?;
        }
    }
}
