use inferno::flamegraph::{self, Options, Palette};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

use super::fetch::PROBING_JSON_PATH;


/// Represents a node in the Trie structure for stack traces.
#[derive(Debug, Clone)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    is_end_of_stack: bool,
    ranks: Vec<u32>,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            is_end_of_stack: false,
            ranks: Vec::new(),
        }
    }

    fn add_rank(&mut self, rank: u32) {
        self.ranks.push(rank);
    }
}

/// Represents a Trie structure for merging stack traces.
struct StackTrie {
    pub root: TrieNode,
    all_ranks: Vec<u32>,
}

impl StackTrie {
    fn new(all_ranks: Vec<u32>) -> Self {
        StackTrie {
            root: TrieNode::new(),
            all_ranks,
        }
    }

    fn insert(&mut self, stack: Vec<&str>, rank: u32) {
        let mut node = &mut self.root;
        for frame in stack {
            node = node.children.entry(frame.to_string()).or_insert_with(TrieNode::new);
            node.add_rank(rank);
        }
        node.is_end_of_stack = true;
        node.add_rank(rank);
    }

    fn format_rank_str(&self, ranks: &[u32]) -> String {
        let mut ranks = ranks.to_vec();
        ranks.sort_unstable();
        let mut leak_ranks: Vec<u32> = self.all_ranks.iter().copied().filter(|r| !ranks.contains(r)).collect();
        leak_ranks.sort_unstable();

        fn inner_format(ranks: &[u32]) -> String {
            let mut str_buf = String::new();
            let mut low = 0;
            let mut high = 0;
            if ranks.len() == 0 {
                return str_buf;
            }
            while high < ranks.len() - 1 {
                let low_value = ranks[low];
                let mut high_value = ranks[high];
                while high < ranks.len() - 1 && high_value + 1 == ranks[high + 1] {
                    high += 1;
                    high_value = ranks[high];
                }
                low = high + 1;
                high += 1;
                if low_value != high_value {
                    str_buf.push_str(&format!("{}-{}", low_value, high_value));
                } else {
                    str_buf.push_str(&low_value.to_string());
                }
                if high < ranks.len() {
                    str_buf.push('/');
                }
            }
            if high == ranks.len() - 1 {
                str_buf.push_str(&ranks[high].to_string());
            }
            str_buf
        }

        let has_stack_ranks = inner_format(&ranks);
        let leak_stack_ranks = inner_format(&leak_ranks);
        format!("@{}|{}", has_stack_ranks, leak_stack_ranks)
    }

    fn traverse_with_all_stack<'a>(&'a self, node: &'a TrieNode, path: Vec<&str>) -> Vec<(Vec<String>, String)> {
        let mut result = Vec::new();
        for (frame, child) in &node.children {
            let rank_str = self.format_rank_str(&child.ranks);
            if child.is_end_of_stack {
                let path_str = path.join(";");
                result.push((vec![path_str, frame.to_string()], rank_str.clone()));
            }
            let mut child_path = path.clone();
            let frame_rank = format!("{}{}", frame, rank_str);
            child_path.push(&frame_rank[..]);
            // child_path.push(rank_str.as_str());
            result.extend(self.traverse_with_all_stack(child, child_path));
        }
        result
    }
}

/// Merges multiple stack traces into a single StackTrie.
fn merge_stacks(stacks: Vec<&str>) -> StackTrie {
    let all_ranks: Vec<u32> = (0..stacks.len() as u32).collect();
    let mut trie = StackTrie::new(all_ranks);
    for (rank, stack) in stacks.iter().enumerate() {
        let stack_frames: Vec<&str> = stack.split(';').collect();
        trie.insert(stack_frames, rank as u32);
    }
    trie
}

/// Represents a frame in the call stack, which can be either a C frame or a Python frame.
#[derive(Debug, Deserialize, Serialize)]
enum Frame {
    CFrame(CFrame),
    PyFrame(PyFrame),
}

/// Represents a C frame in the call stack.
#[derive(Debug, Deserialize, Serialize)]
struct CFrame {
    file: String,
    func: String,
    ip: String,
    lineno: u32,
}

/// Represents a Python frame in the call stack.
#[derive(Debug, Deserialize, Serialize)]
struct PyFrame {
    file: String,
    func: String,
    lineno: u32,
    locals: serde_json::Value,
}

/// Process call stacks from a JSON file and write the processed stacks to a text file.
fn process_callstacks(input_path: &str, output_path: &str) -> io::Result<()> {
    let mut file = File::open(input_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let frames:  Vec<Vec<Frame>> = serde_json::from_str(&contents)?;

    let mut out_stacks = Vec::new();
    for (i, trace) in frames.iter().enumerate() {
        let mut local_stack = Vec::new();
        for frame in trace {
            match frame {
                Frame::CFrame(cframe) => {
                    // println!("  CFrame:");
                    // println!("    File: {:?}", cframe.file);
                    // println!("    Function: {}", cframe.func);
                    // println!("    IP: {}", cframe.ip);
                    // println!("    Line: {}", cframe.lineno);
                }
                Frame::PyFrame(pyframe) => {
                    // println!("  PyFrame:");
                    // println!("    File: {}", pyframe.file);
                    // println!("    Function: {}", pyframe.func);
                    // println!("    Line: {}", pyframe.lineno);
                    // println!("    Locals: {:?}", pyframe.locals);
                }
            }
            local_stack.push(frame.clone());
        }
        local_stack.reverse();
        out_stacks.push(local_stack);
    }

    let mut prepare_stacks = Vec::new();
    for rank in out_stacks {
        if !rank.is_empty() {
            let data = rank
                .iter()
                .map(|entry| match entry {
                    Frame::CFrame(frame) => format!("{} ({}:{})", frame.func, frame.file, frame.lineno),
                    Frame::PyFrame(frame) => format!("{} ({}:{})", frame.func, frame.file, frame.lineno),
                })
                .collect::<Vec<String>>()
                .join(";");
            prepare_stacks.push(data);
        }
    }

    let mut output_file = File::create(output_path)?;
    for stack in prepare_stacks {
        writeln!(output_file, "{}", stack)?;
    }

    Ok(())
}


/// Generates a flamegraph from a stack trace file and saves it as an SVG file.
fn draw_frame_graph(file_path: &str) {
    let file = File::open(file_path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut options = Options::default();
    options.colors = Palette::Multi(flamegraph::color::MultiPalette::Java);

    let mut output_file = File::create("/tmp/flamegraph.svg").expect("Failed to create SVG file");
    flamegraph::from_reader(&mut options, reader, &mut output_file).expect("Failed to generate flamegraph");

    println!("Flamegraph generated and saved as flamegraph.svg");
}

/**
 - Revert json to txt;
 - Merge stacks;
 - Generate flamegraph;
 */
pub fn draw_frame_graph_from_json() -> io::Result<()> {
    let input_path = PROBING_JSON_PATH;
    let output_path = "/tmp/processed_stacks.txt";
    process_callstacks(input_path, output_path)?;

    let file = File::open(output_path)?;
    let reader = BufReader::new(file);

    let mut content = String::new();
    for line in reader.lines() {
        content.push_str(&line?);
        content.push('\n');
    }

    let stacks: Vec<&str> = content.lines().collect();
    let trie = merge_stacks(stacks);

    let output_path = "/tmp/merged_stacks.txt";
    let mut output = File::create(output_path)?;
    for (path, rank_str) in trie.traverse_with_all_stack(&trie.root, Vec::new()) {
        writeln!(output, "{} {} 1", path.join(";"), rank_str)?;
    }
    draw_frame_graph(output_path);
    Ok(())
}