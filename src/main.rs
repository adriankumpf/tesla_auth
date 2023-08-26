use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use log::LevelFilter;
use oauth2::url::Url;
use simple_logger::SimpleLogger;

use muda::{Menu, PredefinedMenuItem, Submenu};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
#[cfg(target_os = "linux")]
use wry::application::platform::unix::WindowExtUnix;
use wry::application::window::WindowBuilder;
use wry::webview::WebViewBuilder;

mod auth;
mod htime;

const INITIALIZATION_SCRIPT: &str = r#"
window.addEventListener('DOMContentLoaded', (event) => {
    const url = window.location.toString();

    if (url.startsWith("https://auth.tesla.com/void/callback")) {
       document.querySelector("h1.h1").innerText = "Generating Tokens …";
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
    menu_bar.init_for_hwnd(window.hwnd() as _)?;
    #[cfg(target_os = "linux")]
    menu_bar.init_for_gtk_window(window.gtk_window(), window.default_vbox())?;
    #[cfg(target_os = "macos")]
    menu_bar.init_for_nsapp();

    let proxy = event_proxy.clone();

    let webview = WebViewBuilder::new(window)?
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_navigation_handler(move |uri: String| {
            let uri = Url::parse(&uri).expect("not a valid URL");
            proxy.send_event(UserEvent::Navigation(uri)).is_ok()
        })
        .with_clipboard(true)
        .with_url(auth_url.as_str())?
        .with_devtools(true)
        .build()?;

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

            Event::UserEvent(UserEvent::Navigation(url)) => {
                log::debug!("URL changed: {url}");
                tx.send(url).unwrap();
            }

            Event::UserEvent(UserEvent::Failure(error)) => {
                log::error!("{error}");
                webview.evaluate_script(&render_error_view(error)).unwrap();
            }

            Event::UserEvent(UserEvent::Tokens(token)) => {
                println!("{token}");
                webview.evaluate_script(&render_tokens_view(token)).unwrap();
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
                let query: HashMap<_, _> = url.query_pairs().collect();

                if let Some(Cow::Borrowed("login_cancelled")) = query.get("error") {
                    return event_proxy.send_event(UserEvent::LoginCanceled).unwrap();
                }

                let state = query.get("state").expect("No state parameter found");
                let code = query.get("code").expect("No code parameter found");
                let issuer = query.get("issuer").expect("No issuer parameter found");
                let issuer_url = Url::parse(issuer).expect("Issuer URL is not valid");

                let event = match client.retrieve_tokens(code, state, &issuer_url) {
                    Ok(tokens) => UserEvent::Tokens(tokens),
                    Err(error) => UserEvent::Failure(error),
                };

                return event_proxy.send_event(event).unwrap();
            }
        }
    });

    tx
}

fn render_error_view(error: anyhow::Error) -> String {
    r#"
        const html = `
            <h4 style="text-align: center;">An error occurred. Please try again ...</h4>
            <p style="text-align: center;color:red;margin-bottom:20px;">{msg}</p>
        `;
        document.querySelector("h1.h1").outerHTML = html;
    "#
    .replace("{msg}", &error.to_string())
}

fn render_tokens_view(tokens: auth::Tokens) -> String {
    r#"
        const html = `
            <h4 style="text-align: center;">Access Token</h4>
            <textarea readonly onclick="this.setSelectionRange(0, this.value.length)"
                      cols="100" rows="12" style="resize:none;padding:4px;font-size:0.9em;"
            >{access_token}</textarea>
            <h4 style="text-align: center;">Refresh Token</h4>
            <textarea readonly onclick="this.setSelectionRange(0, this.value.length)"
                      cols="100" rows="12" style="resize:none;padding:4px;font-size:0.9em;"
            >{refresh_token}</textarea>
            <small style="margin-top:12px;margin-bottom:20px;text-align:center;color:seagreen;">
            Valid for {expires_in}
            </small>
        `;

        document.querySelector("h1.h1").outerHTML = html;
    "#
    .replace("{access_token}", tokens.access.secret())
    .replace("{refresh_token}", tokens.refresh.secret())
    .replace("{expires_in}", &format!("{}", tokens.expires_in))
}
