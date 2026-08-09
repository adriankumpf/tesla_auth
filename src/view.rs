//! Standalone result pages.
//!
//! These replace the document in the webview once the SSO flow is done, so
//! nothing here has to coexist with — or work around — Tesla's own markup.

use crate::auth::Tokens;

const STYLESHEET: &str = "\
:root {
  color-scheme: light dark;
  --bg: #f3f4f6;
  --card: #ffffff;
  --fg: #111827;
  --muted: #6b7280;
  --border: #d1d5db;
  --danger: #b91c1c;
  --success: #15803d;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0b0d10;
    --card: #16191e;
    --fg: #e5e7eb;
    --muted: #9ca3af;
    --border: #374151;
    --danger: #f87171;
    --success: #4ade80;
  }
}
*, *::before, *::after { box-sizing: border-box; }
body {
  margin: 0;
  min-height: 100vh;
  display: flex;
  padding: 24px;
  background: var(--bg);
  color: var(--fg);
  font: 16px/1.5 -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}
main {
  /* `margin: auto` rather than `align-items: center`, which clips the top of
     the card once the content outgrows the viewport. */
  margin: auto;
  width: min(900px, 100%);
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 28px;
  border-radius: 16px;
  background: var(--card);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.12);
}
h1 { margin: 0; text-align: center; font-size: 30px; line-height: 1.2; }
h2 { margin: 0; font-size: 15px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: var(--muted); }
p { margin: 0; text-align: center; }
p.muted { color: var(--muted); }
p.danger { color: var(--danger); }
p.success { color: var(--success); font-size: 14px; }
textarea {
  width: 100%;
  height: 12em;
  resize: vertical;
  padding: 12px;
  color: inherit;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  font: 13px/1.4 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
";

/// Shown while the authorization code is being exchanged.
pub fn progress() -> String {
    page(
        "<h1>Generating Tokens …</h1>\
         <p class=\"muted\">Exchanging the authorization code with Tesla.</p>",
    )
}

pub fn tokens(tokens: &Tokens) -> String {
    page(&format!(
        "<h1>Tesla API Tokens</h1>\
         <h2>Access Token</h2>\
         <textarea readonly onclick=\"this.select()\">{access}</textarea>\
         <h2>Refresh Token</h2>\
         <textarea readonly onclick=\"this.select()\">{refresh}</textarea>\
         <p class=\"success\">Valid for {expires_in}</p>",
        access = escape(tokens.access.secret()),
        refresh = escape(tokens.refresh.secret()),
        expires_in = escape(&tokens.expires_in.to_string()),
    ))
}

pub fn error(error: &anyhow::Error) -> String {
    page(&format!(
        "<h1>An error occurred</h1>\
         <p class=\"danger\">{}</p>\
         <p class=\"muted\">Please restart tesla_auth and try again.</p>",
        escape(&error.to_string()),
    ))
}

fn page(body: &str) -> String {
    format!(
        "<!DOCTYPE html>\
         <html lang=\"en\">\
         <head>\
         <meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>Tesla Auth</title>\
         <style>{STYLESHEET}</style>\
         </head>\
         <body><main>{body}</main></body>\
         </html>"
    )
}

/// Error messages can echo back attacker-controlled query parameters, so every
/// interpolated value goes through here.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html_metacharacters() {
        assert_eq!(escape("a&b"), "a&amp;b");
        assert_eq!(
            escape("</textarea><script>alert('x')</script>"),
            "&lt;/textarea&gt;&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
        assert_eq!(escape("plain text"), "plain text");
    }
}
