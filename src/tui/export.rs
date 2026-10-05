//! Renders the current request as a shell command, for sharing or scripting.

use crate::{
    config::types::GemonMethodType,
    constants::{DEFAULT_ACCEPT, DEFAULT_CONTENT_TYPE},
};

pub struct ExportRequest<'a> {
    pub method: GemonMethodType,
    pub url: &'a str,
    pub headers: &'a [(String, String)],
    pub body: Option<&'a str>,
    pub form: &'a [(String, String)],
    pub secure: bool,
}

/// The equivalent `gemon` CLI invocation. Placeholders are kept, since the CLI resolves them
/// from the selected environment itself.
pub fn gemon_command(request: &ExportRequest<'_>) -> String {
    let mut args = vec![
        String::from("gemon"),
        String::from("-t=REST"),
        format!("-m={}", request.method),
        shell_quote(&format!("-u={}", request.url)),
    ];
    for (key, value) in request.headers {
        args.push(shell_quote(&format!("-h={key}::{value}")));
    }
    if let Some(body) = request.body {
        args.push(shell_quote(&format!("-b={body}")));
    }
    for (key, value) in request.form {
        args.push(shell_quote(&format!("-fd={key}::{value}")));
    }
    if request.secure {
        args.push(String::from("-sec"));
    }
    args.join(" ")
}

/// A `curl` command sending what gemon sends; expects already resolved values.
pub fn curl_command(request: &ExportRequest<'_>) -> String {
    let mut parts = vec![format!(
        "curl -X {} {}",
        request.method,
        shell_quote(request.url)
    )];

    let has_header = |name: &str| {
        request
            .headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case(name))
    };
    let content_type = if request.form.is_empty() {
        DEFAULT_CONTENT_TYPE
    } else {
        "application/x-www-form-urlencoded"
    };
    if !has_header("content-type") {
        parts.push(format!(
            "-H {}",
            shell_quote(&format!("Content-Type: {content_type}"))
        ));
    }
    if !has_header("accept") {
        parts.push(format!(
            "-H {}",
            shell_quote(&format!("Accept: {DEFAULT_ACCEPT}"))
        ));
    }
    for (key, value) in request.headers {
        parts.push(format!("-H {}", shell_quote(&format!("{key}: {value}"))));
    }
    match request.body {
        Some(body) => parts.push(format!("--data-raw {}", shell_quote(body))),
        None => {
            // curl encodes only the value of `name=value`, so the name is encoded here.
            for (key, value) in request.form {
                parts.push(format!(
                    "--data-urlencode {}",
                    shell_quote(&format!("{}={value}", percent_encode(key)))
                ));
            }
        }
    }
    parts.join(" \\\n  ")
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// Quotes `value` for POSIX shells, leaving simple words untouched.
pub fn shell_quote(value: &str) -> String {
    let safe = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@%+,".contains(c));
    if safe {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(
        headers: &'a [(String, String)],
        form: &'a [(String, String)],
        body: Option<&'a str>,
    ) -> ExportRequest<'a> {
        ExportRequest {
            method: GemonMethodType::Post,
            url: "{base_uri}/items?a=1&b=2",
            headers,
            body,
            form,
            secure: true,
        }
    }

    #[test]
    fn quotes_shell_metacharacters() {
        assert_eq!(shell_quote("-m=GET"), "-m=GET");
        assert_eq!(shell_quote("a&b"), "'a&b'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn gemon_command_includes_every_request_part() {
        let headers = vec![(String::from("X-Id"), String::from("1 2"))];
        let form = vec![(String::from("k"), String::from("v"))];
        let command = gemon_command(&request(&headers, &form, Some("{\"a\":1}")));

        assert_eq!(
            command,
            "gemon -t=REST -m=POST '-u={base_uri}/items?a=1&b=2' '-h=X-Id::1 2' '-b={\"a\":1}' -fd=k::v -sec"
        );
    }

    #[test]
    fn curl_command_uses_form_encoding_without_body() {
        let form = vec![
            (String::from("name"), String::from("a b")),
            (String::from("pass word"), String::from("x")),
        ];
        let command = curl_command(&request(&[], &form, None));

        assert!(command.contains("Content-Type: application/x-www-form-urlencoded"));
        assert!(command.contains("--data-urlencode 'name=a b'"));
        assert!(command.contains("--data-urlencode pass%20word=x"));
    }
}
