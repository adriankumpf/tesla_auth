extern crate static_vcruntime;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use anyhow::anyhow;
use log::{debug, error, LevelFilter};
use oauth2::url::Url;
use simple_logger::SimpleLogger;

use wry::application::accelerator::{Accelerator, SysMods};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use wry::application::keyboard::KeyCode;
use wry::application::menu::{MenuBar, MenuId, MenuItem, MenuItemAttributes, MenuType};
use wry::application::window::{Window, WindowBuilder};

use wry::webview::{RpcRequest, RpcResponse, WebViewBuilder};
use wry::Value;

mod auth;

const INITIALIZATION_SCRIPT: &str = r#"
    window.addEventListener('DOMContentLoaded', function(event) {
        rpc.call('url', window.location.toString());
    });
"#;

#[derive(Debug, Clone)]
enum CustomEvent {
    Tokens(auth::Tokens),
}

fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("reqwest", LevelFilter::Debug)
        .with_module_level("tesla_auth", LevelFilter::Debug)
        .init()?;

    let event_loop = EventLoop::<CustomEvent>::with_user_event();
    let event_proxy = event_loop.create_proxy();

    let auth_client = auth::Client::new();
    let auth_url = auth_client.authorize_url();

    let (menu, quit_id) = build_menu();

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .with_menu(menu)
        .build(&event_loop)?;

    let _webview = WebViewBuilder::new(window)?
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_url(auth_url.as_str())?
        .with_rpc_handler(rpc_url_handler(auth_client, event_proxy))
        .build()?;

    debug!("Opening {} ...", auth_url);

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

            Event::UserEvent(CustomEvent::Tokens(tokens)) => {
                print!(
                    r#"
--------------------------------- ACCESS TOKEN ---------------------------------

{}

--------------------------------- REFRESH TOKEN --------------------------------

{}

                "#,
                    tokens.access, tokens.refresh
                );

                *control_flow = ControlFlow::Exit
            }

            _ => (),
        }
    });
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

fn rpc_url_handler(
    client: auth::Client,
    event_proxy: EventLoopProxy<CustomEvent>,
) -> impl Fn(&Window, RpcRequest) -> Option<RpcResponse> {
    let (tx, rx) = channel();

    let handler = move |_window: &Window, req: RpcRequest| {
        if let ("url", Some(params)) = (req.method.as_str(), req.params) {
            if let Ok(url) = parse_url(params) {
                tx.send(url).unwrap();
            }
        }

        None
    };

    thread::spawn(move || {
        while let Ok(url) = rx.recv() {
            if auth::is_redirect_url(&url) {
                let query: HashMap<_, _> = url.query_pairs().collect();

                let state = query.get("state").expect("No state parameter found");
                let code = query.get("code").expect("No code parameter found");

                match client.retrieve_tokens(code, state) {
                    Ok(tokens) => event_proxy.send_event(CustomEvent::Tokens(tokens)).unwrap(),
                    Err(e) => error!("{}", e),
                };

                break;
            }
        }
    });

    handler
}

fn parse_url(params: Value) -> anyhow::Result<Url> {
    match &serde_json::from_value::<Vec<String>>(params)?[..] {
        [url] => Ok(Url::parse(url)?),
        _ => Err(anyhow!("Invalid url param!")),
    }
}
