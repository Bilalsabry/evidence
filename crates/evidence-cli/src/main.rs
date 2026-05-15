//! Headless CLI for evidence.

#![forbid(unsafe_code)]

fn main() {
    println!("evidence v{}", evidence_core::version());
}
