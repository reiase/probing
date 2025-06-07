use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::cli::ctrl::{self, ProbeEndpoint};

#[derive(Debug, Default, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub cmd: String,
    pub socket_name: Option<String>,
    pub remote_addr: Option<String>,
    pub children: Vec<ProcessInfo>,
}

/// Collect information about processes with injected probes
pub async fn collect_probe_processes() -> Result<Vec<ProcessInfo>> {
    let mut processes = Vec::new();
    let mut tasks = Vec::new();

    for (pid_val, socket_name_val) in find_probe_sockets()? {
        tasks.push(tokio::spawn(async move {
            let res = get_process_info(pid_val, Some(socket_name_val.clone())).await;
            (pid_val, socket_name_val, res) // Return pid and socket_name along with the result
        }));
    }

    for task_handle in tasks {
        match task_handle.await {
            Ok((_, _, Ok(info))) => processes.push(info),
            Ok((pid, socket_name, Err(e))) => {
                log::warn!("Error getting full info for PID {pid} (socket: {socket_name}): {e}.");
                let ppid = read_parent_pid(pid).unwrap_or(0); // Best effort
                let cmd = read_process_cmdline(pid).unwrap_or_else(|_| String::from("[cmd error]")); // Best effort

                processes.push(ProcessInfo {
                    pid,
                    ppid,
                    cmd,
                    socket_name: Some(socket_name),
                    ..Default::default()
                });
            }
            Err(err) => {
                log::warn!("Task join error (task may have panicked or been cancelled): {err}")
            }
        }
    }
    Ok(processes)
}

/// Find all abstract unix sockets related to probing
fn find_probe_sockets() -> Result<Vec<(i32, String)>, std::io::Error> {
    let mut result = Vec::new();

    // Read /proc/net/unix for abstract sockets
    let file = File::open("/proc/net/unix")?;
    let reader = BufReader::new(file);

    // Skip header
    let mut lines = reader.lines();
    let _ = lines.next();

    // Process socket entries
    for line in lines {
        let line = line?;
        let fields: Vec<&str> = line.split_whitespace().collect();

        // Check if we have enough fields and it's an abstract socket
        if fields.len() >= 8 {
            let socket_name_full = fields[7]; // e.g., @probing-12345

            // Check for the new naming convention: @probing-<pid>
            if socket_name_full.starts_with("@probing-") {
                let pid_str = &socket_name_full[9..]; // Skip "@probing-"
                if let Ok(pid) = pid_str.parse::<i32>() {
                    result.push((pid, socket_name_full.to_string()));
                } else {
                    log::warn!(
                        "Failed to parse PID from socket name: {socket_name_full}. Expected format @probing-<pid>."
                    );
                }
            }
        }
    }

    Ok(result)
}

/// Get information about a process
pub async fn get_process_info(pid: i32, socket_name: Option<String>) -> Result<ProcessInfo> {
    let ppid = read_parent_pid(pid)?;
    let cmd = read_process_cmdline(pid)?;
    let mut remote_addr: Option<String> = None;

    if socket_name.is_some() {
        let endpoint = ProbeEndpoint::Local { pid };
        let config_url = "/config/server.address";

        match ctrl::request(endpoint, config_url, None).await {
            Ok(response_bytes) => match String::from_utf8(response_bytes) {
                Ok(addr_str) => remote_addr = Some(addr_str),
                Err(e) => log::warn!("PID {pid}: Failed to parse server.address: {e}"),
            },
            Err(e) => log::warn!("PID {pid}: HTTP request to {config_url} failed: {e}"),
        }
    }

    Ok(ProcessInfo {
        pid,
        ppid,
        cmd,
        socket_name,
        remote_addr,
        children: Vec::new(), // Initialize children
    })
}

/// Read parent PID from /proc/{pid}/status
fn read_parent_pid(pid: i32) -> Result<i32, std::io::Error> {
    let status_path = format!("/proc/{}/status", pid);

    if !Path::new(&status_path).exists() {
        return Ok(0);
    }

    let file = File::open(status_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("PPid:") {
            if let Some(ppid_str) = line.split_whitespace().nth(1) {
                return Ok(ppid_str.parse::<i32>().unwrap_or(0));
            }
        }
    }

    Ok(0)
}

/// Read process command line
fn read_process_cmdline(pid: i32) -> Result<String, std::io::Error> {
    let cmdline_path = format!("/proc/{}/cmdline", pid);

    if !Path::new(&cmdline_path).exists() {
        return Ok(String::from("[unknown]"));
    }

    let cmdline = std::fs::read_to_string(cmdline_path)?;

    // cmdline is null-byte separated
    let cmd = cmdline.replace('\0', " ").trim().to_string();

    if cmd.is_empty() {
        let comm_path = format!("/proc/{}/comm", pid);
        if Path::new(&comm_path).exists() {
            let comm = std::fs::read_to_string(comm_path)?;
            return Ok(comm.trim().to_string());
        }
        return Ok(String::from("[unknown]"));
    }

    Ok(cmd)
}

/// Build a process tree from a flat list of processes
pub fn build_process_tree(processes: Vec<ProcessInfo>) -> Vec<ProcessInfo> {
    // Map of PID to process info, ensuring children are initialized empty
    let mut pid_map: HashMap<i32, ProcessInfo> = processes
        .into_iter()
        .map(|mut p| {
            p.children = Vec::new(); // Ensure children are empty before putting into map
            (p.pid, p)
        })
        .collect();

    // Set of all PIDs that are in our collection
    let process_pids: HashSet<i32> = pid_map.keys().cloned().collect();

    let mut root_pids = Vec::new();
    // Adjacency list for building the tree: Parent PID -> Vec<Child PID>
    let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();

    for (pid, process_info) in &pid_map {
        // A process is a root if its parent is not in our collection or ppid is 0 (or self-parented, though less common for PPID)
        if !process_pids.contains(&process_info.ppid)
            || process_info.ppid == 0
            || process_info.ppid == *pid
        {
            root_pids.push(*pid);
        } else {
            // This is a child of another process in our collection
            adj.entry(process_info.ppid).or_default().push(*pid);
        }
    }

    let mut final_tree = Vec::new();
    // Keep track of PIDs that have been incorporated into the tree to avoid duplicates
    // if the input data isn't strictly a tree (e.g., shared children, cycles via ppid)
    let mut processed_pids = HashSet::new();

    for root_pid in root_pids {
        if !processed_pids.contains(&root_pid) {
            // Check if this root hasn't been processed as a child of another "root"
            if let Some(root_info_owned) = pid_map.remove(&root_pid) {
                // Take ownership from the map
                processed_pids.insert(root_pid); // Mark as processed
                final_tree.push(build_subtree_recursive(
                    root_info_owned,
                    &adj,
                    &mut pid_map,
                    &mut processed_pids,
                ));
            }
        }
    }
    final_tree
}

/// Build a subtree recursively.
/// Takes ownership of `parent_info`, populates its children by taking them from `pid_map`, and returns it.
fn build_subtree_recursive(
    mut parent_info: ProcessInfo,
    adj: &HashMap<i32, Vec<i32>>,
    pid_map: &mut HashMap<i32, ProcessInfo>, // Mutable to take ownership of children's ProcessInfo
    processed_pids: &mut HashSet<i32>,       // To mark nodes as processed and avoid reprocessing
) -> ProcessInfo {
    parent_info.children = Vec::new(); // Ensure children list is fresh for this parent

    if let Some(child_pids) = adj.get(&parent_info.pid) {
        let sorted_child_pids = child_pids.clone(); // Cloning to potentially sort without affecting adj

        for &child_pid in &sorted_child_pids {
            // Iterate over sorted or original child PIDs
            if !processed_pids.contains(&child_pid) {
                // Ensure child hasn't been processed already
                if let Some(child_info_owned) = pid_map.remove(&child_pid) {
                    // Take ownership of the child's info
                    processed_pids.insert(child_pid); // Mark as processed
                    let child_node =
                        build_subtree_recursive(child_info_owned, adj, pid_map, processed_pids);
                    parent_info.children.push(child_node);
                }
                // If child_info_owned is None, it means it was already taken (e.g., part of another root's tree or error in data)
            }
        }
    }
    parent_info
}

/// Print the process tree
pub fn print_process_tree(nodes: &[ProcessInfo], verbose: bool, prefix: &str) {
    // `is_parent_last` indicates if the direct parent of the current list of `nodes` was the last in its own sibling list.
    // This helps determine the vertical bar character in the prefix for children.
    for (i, node) in nodes.iter().enumerate() {
        let is_current_node_last = i == nodes.len() - 1; // Is the current node the last in *this* list of siblings?

        let connector = if is_current_node_last {
            "└── "
        } else {
            "├── "
        };
        println!("{}{}{}", prefix, connector, format_process(node, verbose));

        if !node.children.is_empty() {
            // The prefix for the children lines depends on whether the *current node* (their parent) is the last in its list.
            let child_prefix = format!(
                "{}{}",
                prefix,
                if is_current_node_last {
                    "    "
                } else {
                    "│   "
                }
            );
            // When calling recursively for the children, the `is_parent_last` for that call is `is_current_node_last`.
            print_process_tree(&node.children, verbose, &child_prefix);
        }
    }
}

/// Format process information for display
fn format_process(info: &ProcessInfo, verbose: bool) -> String {
    if verbose {
        let socket_str = info.socket_name.as_deref().unwrap_or("-");
        let remote_str = info.remote_addr.as_deref().unwrap_or("-");
        format!(
            "{} (socket: {}, remote: {}): {}",
            info.pid, socket_str, remote_str, info.cmd
        )
    } else {
        if let Some(addr) = &info.remote_addr {
            format!("{}: {} (remote: {})", info.pid, info.cmd, addr)
        } else {
            format!("{}: {}", info.pid, info.cmd)
        }
    }
}
