// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! A cryptographic signing service with an object‐capability interface.

mod cli;
mod local;

#[doc(hidden)]
pub use cli::main;
