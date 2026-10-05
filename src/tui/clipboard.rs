//! Clipboard access without extra dependencies: the platform's clipboard tool when one is
//! installed, otherwise the OSC 52 terminal escape sequence.

use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

/// Copies `text` and returns a short description of the mechanism that was used.
pub fn copy(text: &str) -> Result<&'static str, String> {
    if let Some(tool) = copy_with_system_tool(text) {
        return Ok(tool);
    }
    copy_with_osc52(text)
        .map(|_| "terminal clipboard")
        .map_err(|err| format!("Could not copy to clipboard: {err}"))
}

fn copy_with_system_tool(text: &str) -> Option<&'static str> {
    let tools: &[(&'static str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };

    tools
        .iter()
        .find(|(program, args)| run_with_stdin(program, args, text))
        .map(|(program, _)| *program)
}

fn run_with_stdin(program: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    let written = child
        .stdin
        .take()
        .map(|mut stdin| stdin.write_all(text.as_bytes()).is_ok())
        .unwrap_or(false);
    let succeeded = child.wait().map(|status| status.success()).unwrap_or(false);
    written && succeeded
}

fn copy_with_osc52(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "\x1b]52;c;{}\x07", base64(text.as_bytes()))?;
    stdout.flush()
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (index, byte)| acc | u32::from(*byte) << (16 - 8 * index));
        for index in 0..4 {
            if index <= chunk.len() {
                encoded.push(ALPHABET[(triple >> (18 - 6 * index) & 0x3F) as usize] as char);
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_reference_values() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
