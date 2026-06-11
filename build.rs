// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Generate the code for the Cap’n Proto schema.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=schema/autopen.capnp");
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/autopen.capnp")
        .run()?;
    Ok(())
}
