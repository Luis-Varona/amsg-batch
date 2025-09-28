// Copyright 2025 Luis M. B. Varona
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anyhow::{Context, Result, bail};
use clap::Parser;
use csv::ReaderBuilder;
use std::{fs, path::Path, process::Command, thread, time::Duration};
use tracing::{error, info, warn};

const DEFAULT_SERVICE: &str = "iMessage";
const DELAY: Duration = Duration::from_millis(1000);
const MIN_NUMBER_LENGTH: usize = 7;
const MAX_NUMBER_LENGTH: usize = 15;

#[derive(Parser)]
#[command(
    version,
    about = "Send bulk texts via Apple Messages on macOS",
    long_about = r#"
Send bulk texts via Apple Messages on macOS, with optional personalization. A `.csv` path
containing recipients and a `.txt` path containing the message text are required.
Optionally, the service (e.g., iMessage or SMS) and a placeholder for recipient names
(replaced with a name every time it appears in the message) can also be provided.

The CSV file of recipients should have no header and either one or two columns. If
`--placeholder` (or `-p`) is provided, the first should contain recipient names and the
second should contain phone numbers. For example:

    Baron von Murderpillow,+1 (234) 567-8910
    Rt. Hon. John A. Stymers,314159265
    [...]

If `--placeholder` (or `-p`) is not provided, the CSV should have only a single column
containing phone numbers, like so:

    +1 (234) 567-8910
    314159265
    [...]"#
)]
struct Args {
    #[arg(
        short,
        long,
        help = "Path to `.csv` file with recipients' numbers and (if applicable) names"
    )]
    recipients: String,

    #[arg(short, long, help = "Path to `.txt` file with the message to send")]
    message: String,

    #[arg(
        short,
        long,
        help = "Service to use to send messages (e.g., iMessage or SMS)",
        default_value_t = String::from(DEFAULT_SERVICE)
    )]
    service: String,

    #[arg(
        short,
        long,
        help = "(Optional) placeholder to be replaced with recipient name (e.g., {name})"
    )]
    placeholder: Option<String>,
}

struct Recipient {
    name: Option<String>,
    number: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    validate_file_path(&args.recipients, "csv")?;
    validate_file_path(&args.message, "txt")?;

    let has_names = args.placeholder.is_some();

    let recipients = read_recipients(&args.recipients, has_names)?
        .into_iter()
        .filter_map(|r| match process_number(&r.number) {
            Ok(processed_number) => Some(Recipient {
                name: r.name,
                number: processed_number,
            }),
            Err(e) => {
                if let Some(name) = r.name {
                    warn!("Skipping recipient {} due to invalid number: {}", name, e);
                } else {
                    warn!("Skipping recipient due to invalid number: {}", e);
                }
                None
            }
        })
        .collect::<Vec<_>>();
    let template = read_message(&args.message)?;

    for recipient in recipients {
        let message = if let (Some(name), Some(placeholder)) = (&recipient.name, &args.placeholder)
        {
            template.replace(placeholder, name)
        } else {
            template.clone()
        };

        if let Err(e) = send_message(&message, &recipient.number, &args.service) {
            if let Some(name) = &recipient.name {
                error!(
                    "Failed to send message to {} ({}): {}",
                    name, recipient.number, e
                );
            } else {
                error!("Failed to send message to {}: {}", recipient.number, e);
            }
        } else if let Some(name) = &recipient.name {
            info!("Message sent to {} ({})", name, recipient.number);
        } else {
            info!("Message sent to {}", recipient.number);
        }

        thread::sleep(DELAY);
    }

    Ok(())
}

fn validate_file_path(path: &str, extension: &str) -> Result<()> {
    let path_obj = Path::new(path);

    if !path_obj.exists() {
        bail!("Path {} does not exist", path);
    }

    if !path_obj.is_file() {
        bail!("{} exists but is not a file", path);
    }

    if path_obj.extension().and_then(|ext| ext.to_str()) != Some(extension) {
        bail!("File {} does not end with .{}", path, extension);
    }

    Ok(())
}

fn read_recipients(path: &str, has_names: bool) -> Result<Vec<Recipient>> {
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .context(format!("Failed to read CSV from {}", path))?;

    let mut recipients = Vec::new();

    for result in rdr.records() {
        let record = result.context("Failed to read CSV record")?;

        let (name, number) = if has_names {
            (
                Some(
                    record
                        .get(0)
                        .context("Failed to get name from CSV record")?
                        .trim()
                        .to_string(),
                ),
                record
                    .get(1)
                    .context("Failed to get number from CSV record")?
                    .trim()
                    .to_string(),
            )
        } else {
            (
                None,
                record
                    .get(0)
                    .context("Failed to get number from CSV record")?
                    .trim()
                    .to_string(),
            )
        };

        recipients.push(Recipient { name, number });
    }

    Ok(recipients)
}

fn process_number(number: &str) -> Result<String> {
    let number = number.trim();

    let (has_plus, stem) = if let Some(stripped) = number.strip_prefix('+') {
        (true, stripped)
    } else {
        (false, number)
    };

    if !stem
        .chars()
        .all(|c| c.is_ascii_digit() || c == ' ' || c == '-' || c == '(' || c == ')')
    {
        bail!("Invalid characters in phone number: {}", number);
    }

    let digits = stem
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();

    let len = digits.len();

    if len < MIN_NUMBER_LENGTH {
        bail!("Phone number {} is too short", number);
    } else if len > MAX_NUMBER_LENGTH {
        bail!("Phone number {} is too long", number);
    }

    let number = if has_plus {
        format!("+{}", digits)
    } else {
        digits
    };

    Ok(number)
}

fn read_message(path: &str) -> Result<String> {
    fs::read_to_string(path).context(format!("Failed to read message from {}", path))
}

fn send_message(message: &str, number: &str, service: &str) -> Result<()> {
    let apple_script = format!(
        r#"
        tell application "Messages"
            activate
            set targetService to 1st service whose service type = {service}
            set targetBuddy to buddy "{number}" of targetService
            send "{message}" to targetBuddy
        end tell
        "#,
        service = service,
        number = number,
        message = escape_applescript_string(message)
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(apple_script)
        .output()
        .context("Failed to execute AppleScript")?;

    if !output.status.success() {
        bail!(
            "AppleScript execution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn escape_applescript_string(message: &str) -> String {
    message
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
