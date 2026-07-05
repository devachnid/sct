// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The standard `--format text|json|yaml` output selector, honoured across
//! commands (see `~/code/house-style/rust-cli.md`: "data on stdout ... a global
//! `--format text|json[|yaml]` enum, honoured by every command. Text by default
//! for humans; json for scripts and agents").
//!
//! Commands that also let the user tune the *human* line rendering use a
//! separate `--template` flag (see [`crate::format`]); `--format` is reserved
//! for choosing between text and the structured machine formats.

use clap::ValueEnum;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text (default).
    #[default]
    Text,
    /// Pretty JSON.
    Json,
    /// YAML.
    #[value(alias = "yml")]
    Yaml,
}

impl OutputFormat {
    /// Honour a deprecated `--json` boolean: when set, force [`Json`](Self::Json)
    /// and print a one-line stderr deprecation note; otherwise return `self`.
    /// Lets commands migrate from a `--json` flag to `--format` without breaking
    /// existing invocations.
    pub fn or_json_flag(self, json: bool) -> Self {
        if json {
            eprintln!("warning: --json is deprecated; use --format json");
            Self::Json
        } else {
            self
        }
    }

    /// Whether this is one of the structured (machine-readable) formats.
    pub fn is_structured(self) -> bool {
        matches!(self, Self::Json | Self::Yaml)
    }

    /// Serialise `value` for the structured formats, or `None` for [`Text`](Self::Text)
    /// (the caller renders its own human output).
    pub fn render<T: serde::Serialize>(self, value: &T) -> anyhow::Result<Option<String>> {
        Ok(match self {
            Self::Text => None,
            Self::Json => Some(serde_json::to_string_pretty(value)?),
            Self::Yaml => Some(serde_yaml_ng::to_string(value)?),
        })
    }

    /// Print `value` for the structured formats and return `true`; return `false`
    /// (printing nothing) for [`Text`](Self::Text), so callers can fall through to
    /// their human rendering:
    ///
    /// ```ignore
    /// if !format.print(&data)? {
    ///     // ... human-readable output ...
    /// }
    /// ```
    pub fn print<T: serde::Serialize>(self, value: &T) -> anyhow::Result<bool> {
        match self.render(value)? {
            Some(s) => {
                println!("{s}");
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
