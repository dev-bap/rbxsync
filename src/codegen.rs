use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config::{CodegenStyle, Config};
use crate::lockfile::Lockfile;

const LUAU_RESERVED: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while", "continue", "type",
    "export",
];

const TS_RESERVED: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn is_valid_luau_identifier(s: &str) -> bool {
    is_valid_identifier(s) && !LUAU_RESERVED.contains(&s)
}

fn is_valid_ts_identifier(s: &str) -> bool {
    is_valid_identifier(s) && !TS_RESERVED.contains(&s)
}

pub fn format_key(key: &str) -> String {
    if is_valid_luau_identifier(key) {
        key.to_string()
    } else {
        format!("[\"{}\"]", key.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn format_ts_key(key: &str) -> String {
    if is_valid_ts_identifier(key) {
        key.to_string()
    } else {
        format!("\"{}\"", key.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

// ---------------------------------------------------------------------------
// Tree types
// ---------------------------------------------------------------------------

pub type CodegenTree = BTreeMap<String, CodegenNode>;

pub enum CodegenNode {
    Leaf(u64),
    Branch(BTreeMap<String, CodegenNode>),
}

// ---------------------------------------------------------------------------
// Tree building
// ---------------------------------------------------------------------------

/// Insert a leaf (`key = id`) into `tree` at the location described by `segments`.
///
/// Each segment becomes a `Branch`; the final `key` becomes a `Leaf`.
fn insert_into_tree(tree: &mut CodegenTree, segments: &[&str], key: &str, id: u64) {
    if segments.is_empty() {
        tree.insert(key.to_string(), CodegenNode::Leaf(id));
        return;
    }

    let node = tree
        .entry(segments[0].to_string())
        .or_insert_with(|| CodegenNode::Branch(BTreeMap::new()));

    match node {
        CodegenNode::Branch(children) => {
            insert_into_tree(children, &segments[1..], key, id);
        }
        CodegenNode::Leaf(_) => {
            // A leaf already exists at this segment — promote it to a branch.
            // This shouldn't happen with well-formed configs, but handle gracefully.
            let mut children = BTreeMap::new();
            insert_into_tree(&mut children, &segments[1..], key, id);
            *node = CodegenNode::Branch(children);
        }
    }
}

/// Resolve the effective path string for an item.
fn resolve_path<'a>(item_path: Option<&'a str>, section_default: &'a str) -> &'a str {
    item_path.unwrap_or(section_default)
}

/// Build a `CodegenTree` from a lockfile + config, resolving per-item and
/// per-section custom paths. Respects `config.codegen.style`.
pub fn build_tree(lockfile: &Lockfile, config: &Config) -> CodegenTree {
    let mut tree = CodegenTree::new();
    let flat = config.codegen.style == CodegenStyle::Flat;

    let default_pass_path = config.codegen.paths.passes.as_deref().unwrap_or("passes");
    let default_badge_path = config.codegen.paths.badges.as_deref().unwrap_or("badges");
    let default_product_path = config
        .codegen
        .paths
        .products
        .as_deref()
        .unwrap_or("products");

    for (key, lock) in &lockfile.passes {
        let path_str = resolve_path(
            config.passes.get(key).and_then(|c| c.path.as_deref()),
            default_pass_path,
        );
        insert_item(&mut tree, path_str, key, lock.id, flat);
    }

    for (key, lock) in &lockfile.badges {
        let path_str = resolve_path(
            config.badges.get(key).and_then(|c| c.path.as_deref()),
            default_badge_path,
        );
        insert_item(&mut tree, path_str, key, lock.id, flat);
    }

    for (key, lock) in &lockfile.products {
        let path_str = resolve_path(
            config.products.get(key).and_then(|c| c.path.as_deref()),
            default_product_path,
        );
        insert_item(&mut tree, path_str, key, lock.id, flat);
    }

    // Extra entries: "dotted.path.key" = id
    for (full_key, &id) in &config.codegen.extra {
        if flat {
            tree.insert(full_key.clone(), CodegenNode::Leaf(id));
        } else if let Some(dot_pos) = full_key.rfind('.') {
            let path_str = &full_key[..dot_pos];
            let leaf_key = &full_key[dot_pos + 1..];
            let segments: Vec<&str> = path_str.split('.').collect();
            insert_into_tree(&mut tree, &segments, leaf_key, id);
        } else {
            // No dot — insert directly at root
            tree.insert(full_key.clone(), CodegenNode::Leaf(id));
        }
    }

    tree
}

/// Insert an item into the tree, using flat or nested style.
fn insert_item(tree: &mut CodegenTree, path_str: &str, key: &str, id: u64, flat: bool) {
    if flat {
        let flat_key = format!("{path_str}.{key}");
        tree.insert(flat_key, CodegenNode::Leaf(id));
    } else {
        let segments: Vec<&str> = path_str.split('.').collect();
        insert_into_tree(tree, &segments, key, id);
    }
}

/// Build a `CodegenTree` using the default section names (`passes`, `badges`,
/// `products`) with **nested** style. Useful for tests that don't need a full
/// `Config`.
pub fn build_tree_default(lockfile: &Lockfile) -> CodegenTree {
    let mut tree = CodegenTree::new();

    for (key, lock) in &lockfile.passes {
        insert_into_tree(&mut tree, &["passes"], key, lock.id);
    }
    for (key, lock) in &lockfile.badges {
        insert_into_tree(&mut tree, &["badges"], key, lock.id);
    }
    for (key, lock) in &lockfile.products {
        insert_into_tree(&mut tree, &["products"], key, lock.id);
    }

    tree
}

/// Build a `CodegenTree` using the default section names with **flat** style.
/// Useful for tests.
pub fn build_tree_default_flat(lockfile: &Lockfile) -> CodegenTree {
    let mut tree = CodegenTree::new();

    for (key, lock) in &lockfile.passes {
        tree.insert(format!("passes.{key}"), CodegenNode::Leaf(lock.id));
    }
    for (key, lock) in &lockfile.badges {
        tree.insert(format!("badges.{key}"), CodegenNode::Leaf(lock.id));
    }
    for (key, lock) in &lockfile.products {
        tree.insert(format!("products.{key}"), CodegenNode::Leaf(lock.id));
    }

    tree
}

// ---------------------------------------------------------------------------
// Luau rendering
// ---------------------------------------------------------------------------

fn render_luau_node(out: &mut String, node: &CodegenNode, depth: usize) {
    let indent = "\t".repeat(depth);
    match node {
        CodegenNode::Leaf(id) => {
            out.push_str(&format!("{id},\n"));
        }
        CodegenNode::Branch(children) => {
            out.push_str("{\n");
            for (key, child) in children {
                out.push_str(&format!("{indent}\t{} = ", format_key(key)));
                render_luau_node(out, child, depth + 1);
            }
            out.push_str(&format!("{indent}}},\n"));
        }
    }
}

pub fn generate_luau(tree: &CodegenTree, output_path: &Path) -> Result<()> {
    let var_name = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Assets");

    let mut out = String::new();
    out.push_str("-- This file is auto-generated by rbxsync. Do not edit manually.\n\n");
    out.push_str(&format!("local {} = {{\n", var_name));

    for (key, node) in tree {
        out.push_str(&format!("\t{} = ", format_key(key)));
        render_luau_node(&mut out, node, 1);
    }

    out.push_str("}\n\n");
    out.push_str(&format!("return {}\n", var_name));

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    std::fs::write(output_path, out)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// TypeScript rendering
// ---------------------------------------------------------------------------

fn render_ts_node(out: &mut String, node: &CodegenNode, depth: usize) {
    let indent = "\t".repeat(depth);
    match node {
        CodegenNode::Leaf(_) => {
            out.push_str("number\n");
        }
        CodegenNode::Branch(children) => {
            out.push_str("{\n");
            for (key, child) in children {
                out.push_str(&format!("{indent}\t{}: ", format_ts_key(key)));
                render_ts_node(out, child, depth + 1);
            }
            out.push_str(&format!("{indent}}}\n"));
        }
    }
}

pub fn generate_typescript(tree: &CodegenTree, output_path: &Path) -> Result<()> {
    let var_name = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        // Strip .d from "Assets.d.ts" → "Assets"
        .map(|s| s.strip_suffix(".d").unwrap_or(s))
        .unwrap_or("Assets");

    let mut out = String::new();
    out.push_str("// This file is auto-generated by rbxsync. Do not edit manually.\n\n");
    out.push_str(&format!("declare const {}: {{\n", var_name));

    for (key, node) in tree {
        out.push_str(&format!("\t{}: ", format_ts_key(key)));
        render_ts_node(&mut out, node, 1);
    }

    out.push_str("}\n\n");
    out.push_str(&format!("export = {}\n", var_name));

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    std::fs::write(output_path, &out)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    Ok(())
}
