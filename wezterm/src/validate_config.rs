use crate::Opt;
use anyhow::{anyhow, Context};
use clap::{Parser, ValueEnum};
use config::ConfigValidationSnapshot;
use serde::Serialize;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ValidateConfigFormat {
    Human,
    Json,
}

#[derive(Debug, Parser, Clone)]
pub struct ValidateConfigCommand {
    /// Output format for validation results
    #[arg(long, value_enum, default_value = "human")]
    format: ValidateConfigFormat,

    /// Suppress success-only output in human mode
    #[arg(long)]
    quiet: bool,
}

#[derive(Debug, Serialize)]
struct ValidateConfigOutput {
    valid: bool,
    config_file: Option<String>,
    warnings: Vec<String>,
    watch_paths: Vec<String>,
    error: Option<String>,
    using_default_config: bool,
    generation: usize,
}

impl ValidateConfigOutput {
    fn from_snapshot(snapshot: ConfigValidationSnapshot) -> Self {
        Self {
            valid: snapshot.error.is_none(),
            config_file: snapshot.config_file.map(|path| path.display().to_string()),
            warnings: snapshot.warnings,
            watch_paths: snapshot
                .watch_paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            error: snapshot.error,
            using_default_config: snapshot.using_default_config,
            generation: snapshot.generation,
        }
    }

    fn failed(error: anyhow::Error) -> Self {
        Self {
            valid: false,
            config_file: None,
            warnings: vec![],
            watch_paths: vec![],
            error: Some(format!("{error:#}")),
            using_default_config: false,
            generation: 0,
        }
    }
}

impl ValidateConfigCommand {
    pub fn run(&self, opts: &Opt) -> anyhow::Result<()> {
        let output = match config::common_init(
            opts.config_file.as_ref(),
            &opts.config_override,
            opts.skip_config,
        )
        .context("config::common_init")
        {
            Ok(()) => ValidateConfigOutput::from_snapshot(config::configuration_validation()),
            Err(error) => ValidateConfigOutput::failed(error),
        };

        self.emit(&output)?;

        if output.valid {
            Ok(())
        } else {
            Err(anyhow!(
                "{}",
                output
                    .error
                    .clone()
                    .unwrap_or_else(|| "configuration validation failed".to_string())
            ))
        }
    }

    fn emit(&self, output: &ValidateConfigOutput) -> anyhow::Result<()> {
        match self.format {
            ValidateConfigFormat::Human => {
                if output.valid {
                    if !self.quiet {
                        println!("Config validation passed");
                    }
                } else {
                    println!("Config validation failed");
                }

                if let Some(config_file) = &output.config_file {
                    println!("Config file: {config_file}");
                } else if !self.quiet || !output.valid {
                    println!("Config file: <default configuration>");
                }

                if !output.valid {
                    if let Some(error) = &output.error {
                        println!("Error:");
                        println!("{error}");
                    }
                }

                if !output.warnings.is_empty() {
                    println!("Warnings:");
                    for warning in &output.warnings {
                        println!("  - {warning}");
                    }
                }

                if !self.quiet || !output.watch_paths.is_empty() || !output.valid {
                    println!("Watch paths:");
                    if output.watch_paths.is_empty() {
                        println!("  - <none>");
                    } else {
                        for path in &output.watch_paths {
                            println!("  - {path}");
                        }
                    }
                }

                Ok(())
            }
            ValidateConfigFormat::Json => {
                println!("{}", serde_json::to_string_pretty(output)?);
                Ok(())
            }
        }
    }
}
