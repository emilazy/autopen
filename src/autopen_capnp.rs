// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

// TODO: Fix `capnp::generated_code!` with custom lint settings.
#![expect(warnings, reason = "generated code")]

include!(concat!(env!("OUT_DIR"), "/autopen_capnp.rs"));
