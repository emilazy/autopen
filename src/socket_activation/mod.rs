// SPDX-FileCopyrightText: 2026 Emily <hello@emily.moe>
//
// SPDX-License-Identifier: BlueOak-1.0.0

//! Socket activation.

#[cfg(not(target_os = "linux"))]
mod fallback;
#[cfg(target_os = "linux")]
mod systemd;

#[cfg(not(target_os = "linux"))]
pub(crate) use fallback::*;
#[cfg(target_os = "linux")]
pub(crate) use systemd::*;
