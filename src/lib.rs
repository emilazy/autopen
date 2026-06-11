// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! A cryptographic signing service with an object‐capability interface.

mod autopen_capnp;
mod cli;
mod local;
mod signing_key;
mod verification_key;

#[doc(hidden)]
pub use cli::main;
