use std::collections::HashMap;
use std::sync::mpsc::{Sender, channel};
use std::thread;

use log::LevelFilter;
use oauth2::url::Url;
use simple_logger::SimpleLogger;

use muda::{Menu, PredefinedMenuItem, Submenu};
#[cfg(unix)]
use tao::platform::unix::WindowExtUnix;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

mod auth;
mod htime;

const INITIALIZATION_SCRIPT: &str = r#"
window.addEventListener('DOMContentLoaded', (event) => {
    const url = window.location.toString();

    if (url.startsWith("tesla://auth/callback")) {
       const loadingText = document.querySelector(
           '[class*="validated-success-message"], h1.h1'
       );
       if (loadingText) {
           loadingText.textContent = "Generating Tokens ...";
       }
    }
});
"#;

#[derive(Debug)]
enum UserEvent {
    Navigation(Url),
    Tokens(auth::Tokens),
    Failure(anyhow::Error),
    LoginCanceled,
}

#[derive(argh::FromArgs, Debug)]
/// Tesla API tokens generator
struct Args {
    /// print debug output
    #[argh(switch, short = 'd')]
    debug: bool,

    /// clear browsing data at startup
    #[argh(switch, short = 'c')]
    clear_browsing_data: bool,
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    init_logger(args.debug)?;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let event_proxy = event_loop.create_proxy();

    let auth_client = auth::Client::new();
    let auth_url = auth_client.authorize_url();

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .with_resizable(true)
        .build(&event_loop)?;

    let menu_bar = Menu::new();

    #[cfg(target_os = "macos")]
    {
        let app_m = Submenu::new("App", true);
        menu_bar.append(&app_m)?;
        app_m.append_items(&[
            &PredefinedMenuItem::about(None, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ])?;
    }

    let edit_menu = Submenu::new("&Edit", true);
    edit_menu.append_items(&[
        #[cfg(target_os = "macos")]
        &PredefinedMenuItem::undo(None),
        #[cfg(target_os = "macos")]
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ])?;

    let view_menu = Submenu::new("&View", true);
    view_menu.append_items(&[&PredefinedMenuItem::fullscreen(None)])?;

    let window_menu = Submenu::new("&Window", true);
    window_menu.append_items(&[&PredefinedMenuItem::minimize(None)])?;

    menu_bar.append_items(&[
        &edit_menu,
        #[cfg(target_os = "macos")]
        &view_menu,
        #[cfg(not(target_os = "linux"))]
        &window_menu,
    ])?;

    #[cfg(target_os = "windows")]
    unsafe {
        menu_bar.init_for_hwnd(window.hwnd() as _)?;
    }
    #[cfg(unix)]
    menu_bar.init_for_gtk_window(window.gtk_window(), window.default_vbox())?;
    #[cfg(target_os = "macos")]
    menu_bar.init_for_nsapp();

    let proxy = event_proxy.clone();

    let builder = WebViewBuilder::new()
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_navigation_handler(move |uri: String| {
            let Ok(uri) = Url::parse(&uri) else {
                log::warn!("Ignoring malformed navigation URL: {uri}");
                return false;
            };
            proxy.send_event(UserEvent::Navigation(uri)).is_ok()
        })
        .with_clipboard(true)
        .with_url(auth_url.as_str())
        .with_devtools(true);

    #[cfg(any(target_os = "windows", target_os = "macos",))]
    let webview = builder.build(&window)?;

    #[cfg(not(any(target_os = "windows", target_os = "macos",)))]
    let webview = {
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox)?
    };

    if args.clear_browsing_data {
        webview.clear_all_browsing_data()?;
    }

    let tx = url_handler(auth_client, event_proxy);

    log::debug!("Opening {auth_url} ...");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::UserEvent(UserEvent::Navigation(url)) if url.as_str() != "about:blank" => {
                log::debug!("URL changed: {url}");
                let _ = tx.send(url);
            }

            Event::UserEvent(UserEvent::Failure(error)) => {
                log::error!("{error}");
                if let Err(e) = webview.evaluate_script(&render_error_view(error)) {
                    log::error!("Failed to render error view: {e}");
                }
            }

            Event::UserEvent(UserEvent::Tokens(token)) => {
                println!("{token}");
                if let Err(e) = webview.evaluate_script(&render_tokens_view(token)) {
                    log::error!("Failed to render tokens view: {e}");
                }
            }

            Event::UserEvent(UserEvent::LoginCanceled) => {
                log::warn!("Login canceled");
                *control_flow = ControlFlow::Exit;
            }

            _ => (),
        }
    });
}

fn init_logger(debug: bool) -> anyhow::Result<()> {
    let level_filter = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };

    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("reqwest", level_filter)
        .with_module_level("tesla_auth", level_filter)
        .init()?;

    Ok(())
}

fn url_handler(client: auth::Client, event_proxy: EventLoopProxy<UserEvent>) -> Sender<Url> {
    let (tx, rx) = channel();

    thread::spawn(move || {
        while let Ok(url) = rx.recv() {
            if auth::is_redirect_url(&url) {
                let event = handle_redirect(&url, client);
                let _ = event_proxy.send_event(event);
                return;
            }
        }
    });

    tx
}

fn handle_redirect(url: &Url, client: auth::Client) -> UserEvent {
    let query: HashMap<_, _> = url.query_pairs().collect();

    if query.get("error").is_some_and(|v| v == "login_cancelled") {
        return UserEvent::LoginCanceled;
    }

    let (Some(state), Some(code), Some(issuer)) =
        (query.get("state"), query.get("code"), query.get("issuer"))
    else {
        return UserEvent::Failure(anyhow::anyhow!(
            "Redirect URL missing required query parameters (state, code, or issuer)"
        ));
    };

    let issuer_url = match Url::parse(issuer) {
        Ok(url) => url,
        Err(e) => return UserEvent::Failure(anyhow::anyhow!("Invalid issuer URL: {e}")),
    };

    match client.retrieve_tokens(code, state, &issuer_url) {
        Ok(tokens) => UserEvent::Tokens(tokens),
        Err(error) => UserEvent::Failure(error),
    }
}

// Encode a string as a JSON string literal for safe JS interpolation.
#[expect(clippy::unwrap_used)] // serde_json string serialization is infallible
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap()
}

fn render_error_view(error: anyhow::Error) -> String {
    let msg = js_string(&error.to_string());
    format!(
        r#"(function() {{
            var mount = document.getElementById("tesla-auth-result");
            if (!mount) {{
                mount = document.createElement("div");
                mount.id = "tesla-auth-result";
                mount.style.cssText = "position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;padding:24px;background:rgba(255,255,255,0.98);overflow:auto";
                document.body.appendChild(mount);
            }}

            mount.replaceChildren();

            var card = document.createElement("div");
            card.style.cssText = "width:min(720px,100%);display:flex;flex-direction:column;gap:12px;padding:28px;border-radius:16px;background:#fff;box-shadow:0 16px 48px rgba(0,0,0,0.12);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif";

            var h4 = document.createElement("h4");
            h4.style.cssText = "margin:0;text-align:center;font-size:28px;line-height:1.2";
            h4.textContent = "An error occurred. Please try again ...";
            var p = document.createElement("p");
            p.style.cssText = "margin:0;text-align:center;color:#b91c1c";
            p.textContent = {msg};

            card.append(h4, p);
            mount.appendChild(card);
        }})()"#
    )
}

fn render_tokens_view(tokens: auth::Tokens) -> String {
    let access = js_string(tokens.access.secret());
    let refresh = js_string(tokens.refresh.secret());
    let expires = js_string(&tokens.expires_in.to_string());
    format!(
        r#"(function() {{
            var mount = document.getElementById("tesla-auth-result");
            if (!mount) {{
                mount = document.createElement("div");
                mount.id = "tesla-auth-result";
                mount.style.cssText = "position:fixed;inset:0;z-index:2147483647;display:flex;align-items:center;justify-content:center;padding:24px;background:rgba(255,255,255,0.98);overflow:auto";
                document.body.appendChild(mount);
            }}

            mount.replaceChildren();

            var card = document.createElement("div");
            card.style.cssText = "width:min(900px,100%);display:flex;flex-direction:column;gap:16px;padding:28px;border-radius:16px;background:#fff;box-shadow:0 16px 48px rgba(0,0,0,0.12);font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif";

            function addToken(label, value) {{
                var h4 = document.createElement("h4");
                h4.style.cssText = "margin:0;text-align:center;font-size:24px;line-height:1.2";
                h4.textContent = label;
                card.appendChild(h4);
                var ta = document.createElement("textarea");
                ta.readOnly = true;
                ta.cols = 100;
                ta.rows = 12;
                ta.style.cssText = "width:100%;resize:vertical;padding:12px;font:13px/1.4 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;border:1px solid #d1d5db;border-radius:12px;box-sizing:border-box";
                ta.value = value;
                ta.addEventListener("click", function() {{ this.setSelectionRange(0, this.value.length); }});
                card.appendChild(ta);
            }}

            var title = document.createElement("h3");
            title.style.cssText = "margin:0;text-align:center;font-size:30px;line-height:1.2";
            title.textContent = "Tesla API Tokens";
            card.appendChild(title);

            addToken("Access Token", {access});
            addToken("Refresh Token", {refresh});

            var small = document.createElement("small");
            small.style.cssText = "display:block;margin-top:4px;text-align:center;color:seagreen;font-size:14px";
            small.textContent = "Valid for " + {expires};
            card.appendChild(small);

            mount.appendChild(card);
        }})()"#
    )
}
