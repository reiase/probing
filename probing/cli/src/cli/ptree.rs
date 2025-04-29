use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub cmd: String,
    pub socket_name: Option<String>,
}

#[derive(Debug)]
pub struct ProcessNode {
    pub info: ProcessInfo,
    pub children: Vec<ProcessNode>,
}

/// Collect information about processes with injected probes
pub fn collect_probe_processes() -> Result<Vec<ProcessInfo>, std::io::Error> {
    // Find all abstract unix sockets related to probing
    let probe_sockets = find_probe_sockets()?;
    let mut processes = Vec::new();

    for (pid, socket_name) in probe_sockets {
        // Get process information
        if let Ok(Some(info)) = get_process_info(pid, Some(socket_name)) {
            processes.push(info);
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
            let socket_name = fields[7];
            if socket_name.starts_with("@") && socket_name.contains("probing") {
                let inode = fields[6];

                // Find process that has this socket
                if let Some(pid) = find_process_by_socket_inode(inode)? {
                    result.push((pid, socket_name.to_string()));
                }
            }
        }
    }

    Ok(result)
}

/// Find which process owns a socket with given inode
fn find_process_by_socket_inode(inode: &str) -> Result<Option<i32>, std::io::Error> {
    let search_str = format!("socket:[{}]", inode);

    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let path = entry.path();

        // Check if entry is a PID directory
        if let Some(name) = path.file_name() {
            if let Some(name_str) = name.to_str() {
                if name_str.chars().all(|c| c.is_ascii_digit()) {
                    let pid = name_str.parse::<i32>().unwrap_or(-1);
                    let fd_dir = path.join("fd");

                    if fd_dir.exists() {
                        // Check all file descriptors
                        if let Ok(fds) = std::fs::read_dir(&fd_dir) {
                            for fd in fds.flatten() {
                                // if let Ok(fd) = fd {
                                if let Ok(link) = std::fs::read_link(fd.path()) {
                                    if link.to_string_lossy() == search_str {
                                        return Ok(Some(pid));
                                    }
                                }
                                // }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Get information about a process
pub fn get_process_info(
    pid: i32,
    socket_name: Option<String>,
) -> Result<Option<ProcessInfo>, std::io::Error> {
    // Get parent PID
    let ppid = read_parent_pid(pid)?;

    // Get command line
    let cmd = read_process_cmdline(pid)?;

    Ok(Some(ProcessInfo {
        pid,
        ppid,
        cmd,
        socket_name,
    }))
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
        // Try to get comm instead
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
pub fn build_process_tree(processes: Vec<ProcessInfo>) -> Vec<ProcessNode> {
    // Map of PID to process info
    let mut pid_map = HashMap::new();

    // Set of all PIDs that are in our collection
    let mut process_pids = HashSet::new();

    // First pass: Build a map of PID to process info
    for process in &processes {
        pid_map.insert(process.pid, process.clone());
        process_pids.insert(process.pid);
    }

    // Collect root nodes (processes whose parent is not in our collection)
    let mut root_nodes = Vec::new();
    let mut child_map: HashMap<i32, Vec<i32>> = HashMap::new();

    // Second pass: Identify parent-child relationships
    for process in &processes {
        if !process_pids.contains(&process.ppid) || process.ppid == 0 {
            // This is a root process in our tree
            root_nodes.push(process.pid);
        } else {
            // Add to children of parent
            child_map.entry(process.ppid).or_default().push(process.pid);
        }
    }

    // Build tree recursively
    let mut result = Vec::new();
    for root_pid in root_nodes {
        if let Some(root_info) = pid_map.get(&root_pid) {
            let root_node = build_subtree(root_info.clone(), &child_map, &pid_map);
            result.push(root_node);
        }
    }

    result
}

/// Build a subtree recursively
fn build_subtree(
    info: ProcessInfo,
    child_map: &HashMap<i32, Vec<i32>>,
    pid_map: &HashMap<i32, ProcessInfo>,
) -> ProcessNode {
    let mut node = ProcessNode {
        info,
        children: Vec::new(),
    };

    // Add children
    if let Some(child_pids) = child_map.get(&node.info.pid) {
        for &child_pid in child_pids {
            if let Some(child_info) = pid_map.get(&child_pid) {
                let child_node = build_subtree(child_info.clone(), child_map, pid_map);
                node.children.push(child_node);
            }
        }
    }

    node
}

/// Print the process tree
pub fn print_process_tree(nodes: &[ProcessNode], verbose: bool, prefix: &str, is_last: bool) {
    for (i, node) in nodes.iter().enumerate() {
        let is_last_node = i == nodes.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };

        // Print current node
        if prefix.is_empty() {
            println!("{}{}", connector, format_process(&node.info, verbose));
        } else {
            println!(
                "{}{}{}",
                prefix,
                connector,
                format_process(&node.info, verbose)
            );
        }

        // Print children with appropriate prefixes
        if !node.children.is_empty() {
            let child_prefix = if prefix.is_empty() {
                if is_last {
                    "    ".to_string()
                } else {
                    "│   ".to_string()
                }
            } else if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            print_process_tree(&node.children, verbose, &child_prefix, is_last_node);
        }
    }
}

/// Format process information for display
fn format_process(info: &ProcessInfo, verbose: bool) -> String {
    if verbose {
        format!(
            "{} ({}): {}",
            info.pid,
            if let Some(socket) = &info.socket_name {
                socket
            } else {
                "-"
            },
            info.cmd
        )
    } else {
        format!("{}: {}", info.pid, info.cmd)
    }
}

impl Clone for ProcessInfo {
    fn clone(&self) -> Self {
        ProcessInfo {
            pid: self.pid,
            ppid: self.ppid,
            cmd: self.cmd.clone(),
            socket_name: self.socket_name.clone(),
        }
    }
}
