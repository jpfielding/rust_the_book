// https://dev.to/timclicks/two-trie-implementations-in-rust-ones-super-fast-2f3m
use std::collections::HashMap;

#[derive(Default, Debug)]
struct TrieNode {
    is_end_of_word: bool,
    children: HashMap<char, TrieNode>,
}

#[derive(Default, Debug)]
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            root: TrieNode::default(),
        }
    }

    pub fn insert(&mut self, word: &str) {
        let mut current_node = &mut self.root;

        for c in word.chars() {
            current_node = current_node.children.entry(c).or_default();
        }
        current_node.is_end_of_word = true;
    }

    pub fn contains(&self, word: &str) -> bool {
        let mut current_node = &self.root;

        for c in word.chars() {
            match current_node.children.get(&c) {
                Some(node) => current_node = node,
                None => return false,
            }
        }

        current_node.is_end_of_word
    }
}

fn main() {
    let mut trie = Trie::new();
    trie.insert("hello");
    trie.insert("hi");
    trie.insert("hey");
    trie.insert("world");

    println!("{trie:#?}");

    println!("hiiii? {}", trie.contains("hiiii"));
}