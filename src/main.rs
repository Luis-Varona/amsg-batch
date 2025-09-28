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
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    recipients: String,

    #[arg(short, long)]
    message: String,

    #[arg(short, long, default_value_t = String::from(DEFAULT_SERVICE))]
    service: String,

    #[arg(short, long)]
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
                warn!(
                    "Skipping recipient {} due to invalid number: {}",
                    r.name.as_deref().unwrap_or("unknown"),
                    e
                );
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
            error!(
                "Failed to send message to {}: {}",
                recipient.name.as_deref().unwrap_or("unknown"),
                e
            );
        } else {
            info!(
                "Message sent to {}",
                recipient.name.as_deref().unwrap_or(&recipient.number)
            );
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
