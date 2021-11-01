extern crate static_vcruntime;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use anyhow::anyhow;
use log::LevelFilter;
use oauth2::url::Url;
use simple_logger::SimpleLogger;

use wry::application::accelerator::{Accelerator, SysMods};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use wry::application::keyboard::KeyCode;
use wry::application::menu::{MenuBar, MenuId, MenuItem, MenuItemAttributes, MenuType};
use wry::application::window::{Window, WindowBuilder};
use wry::webview::{RpcRequest, RpcResponse, WebViewBuilder};

mod auth;
mod htime;

const INITIALIZATION_SCRIPT: &str = r#"
window.addEventListener('DOMContentLoaded', (event) => {
    const url = window.location.toString();

    if (url.startsWith("https://auth.tesla.com/void/callback")) {
       document.querySelector("h1.h1").innerText = "Generating Tokens …";
    }

    rpc.call('url', url);
});
"#;

#[derive(Debug)]
enum CustomEvent {
    Tokens(auth::Tokens),
    Failure(anyhow::Error),
}

#[derive(argh::FromArgs, Debug)]
/// Tesla API tokens generator
struct Args {
    /// exchange SSO access token for long-lived Owner API token
    #[argh(switch, short = 'o')]
    owner_api_token: bool,

    /// print debug output
    #[argh(switch, short = 'd')]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    init_logger(args.debug)?;

    let event_loop = EventLoop::<CustomEvent>::with_user_event();
    let event_proxy = event_loop.create_proxy();

    let auth_client = auth::Client::new();
    let auth_url = auth_client.authorize_url();

    let (menu, quit_id) = build_menu();

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .with_menu(menu)
        .build(&event_loop)?;

    let webview = WebViewBuilder::new(window)?
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_url(auth_url.as_str())?
        .with_rpc_handler(url_handler(auth_client, event_proxy, args.owner_api_token))
        .build()?;

    log::debug!("Opening {} ...", auth_url);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::MenuEvent {
                menu_id,
                origin: MenuType::MenuBar,
                ..
            } if menu_id == quit_id => *control_flow = ControlFlow::Exit,

            Event::UserEvent(CustomEvent::Failure(e)) => {
                log::error!("{}", e);
                webview.evaluate_script(&render_error_view(e)).unwrap();
            }

            Event::UserEvent(CustomEvent::Tokens(t)) => {
                println!("{}", t);
                webview.evaluate_script(&render_tokens_view(t)).unwrap();
            }

            _ => (),
        }
    });
}

fn init_logger(debug: bool) -> anyhow::Result<()> {
    let level_filter = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Error
    };

    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("reqwest", level_filter)
        .with_module_level("tesla_auth", level_filter)
        .init()?;

    Ok(())
}

fn build_menu() -> (MenuBar, MenuId) {
    let mut menu_bar = MenuBar::new();
    let mut menu = MenuBar::new();

    menu.add_native_item(MenuItem::Copy);
    menu.add_native_item(MenuItem::Paste);
    menu.add_native_item(MenuItem::Separator);
    menu.add_native_item(MenuItem::Hide);
    let quit_item = menu.add_item(
        MenuItemAttributes::new("Quit")
            .with_accelerators(&Accelerator::new(SysMods::Cmd, KeyCode::KeyQ)),
    );

    menu_bar.add_submenu("", true, menu);

    (menu_bar, quit_item.id())
}

fn url_handler(
    client: auth::Client,
    event_proxy: EventLoopProxy<CustomEvent>,
    exchange_sso_token: bool,
) -> impl Fn(&Window, RpcRequest) -> Option<RpcResponse> {
    let (tx, rx) = channel();

    thread::spawn(move || {
        while let Ok(url) = rx.recv() {
            if auth::is_redirect_url(&url) {
                let query: HashMap<_, _> = url.query_pairs().collect();

                let state = query.get("state").expect("No state parameter found");
                let code = query.get("code").expect("No code parameter found");
                let issuer = query.get("issuer").expect("No issuer parameter found");
                let issuer_url = Url::parse(issuer).expect("Issuer URL is not valid");

                let event =
                    match client.retrieve_tokens(code, state, &issuer_url, exchange_sso_token) {
                        Ok(tokens) => CustomEvent::Tokens(tokens),
                        Err(error) => CustomEvent::Failure(error),
                    };

                return event_proxy.send_event(event).unwrap();
            }
        }
    });

    move |_window: &Window, req: RpcRequest| {
        if let ("url", Some(params)) = (req.method.as_str(), req.params) {
            if let Ok(url) = parse_url(params) {
                log::debug!("URL changed: {}", url);
                tx.send(url).unwrap();
            }
        }

        None
    }
}

fn parse_url(params: wry::Value) -> anyhow::Result<Url> {
    match &serde_json::from_value::<Vec<String>>(params)?[..] {
        [url] => Ok(Url::parse(url)?),
        _ => Err(anyhow!("Invalid url param!")),
    }
}

fn render_error_view(error: anyhow::Error) -> String {
    r#"
        const html = `
            <h4 style="text-align: center;">An error occured. Please try again ...</h4>
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
