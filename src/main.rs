mod auth;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use anyhow::anyhow;
use log::{debug, error, info, LevelFilter};
use oauth2::url::Url;
use simple_logger::SimpleLogger;

use wry::application::accelerator::{Accelerator, SysMods};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use wry::application::keyboard::KeyCode;
use wry::application::menu::{MenuBar, MenuId, MenuItem, MenuItemAttributes, MenuType};
use wry::application::window::{Window, WindowBuilder};
use wry::http::{Request, Response, ResponseBuilder};
use wry::webview::{RpcRequest, RpcResponse, WebViewBuilder};
use wry::Value;

const INITIALIZATION_SCRIPT: &str = r#"
    window.addEventListener('DOMContentLoaded', function(event) {
        var url = window.location.toString();

        if (url.startsWith("https://auth.tesla.com/void/callback")) {
            location.replace("wry://index.html?access=loading...&refresh=loading...");
            rpc.call('url', url);
        }
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

    let webview = WebViewBuilder::new(window)?
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_custom_protocol("wry".into(), protocol_handler)
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

            Event::UserEvent(CustomEvent::Tokens(tokens)) => {
                info!("Received tokens: {:#?}", tokens);

                let url = format!(
                    "location.replace('wry://index.html?access={}&refresh={}');",
                    tokens.access, tokens.refresh
                );

                webview.evaluate_script(&url).unwrap();
            }

            Event::MenuEvent {
                menu_id,
                origin: MenuType::MenuBar,
                ..
            } => {
                match menu_id {
                    id if id == quit_id => *control_flow = ControlFlow::Exit,
                    _ => (),
                };
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

fn protocol_handler(request: &Request) -> wry::Result<Response> {
    let url = request.uri().parse::<Url>()?;

    match url.domain() {
        Some("index.html") => {
            let query = url.query_pairs().collect::<HashMap<_, _>>();

            let content = match (query.get("access"), query.get("refresh")) {
                (Some(access), Some(refresh)) => include_str!("../views/index.html")
                    .replace("{access_token}", access)
                    .replace("{refresh_token}", refresh)
                    .as_bytes()
                    .to_vec(),

                (_, _) => vec![],
            };

            ResponseBuilder::new().mimetype("text/html").body(content)
        }

        domain => unimplemented!("Cannot open {:?}", domain),
    }
}

fn parse_url(params: Value) -> anyhow::Result<Url> {
    match &serde_json::from_value::<Vec<String>>(params)?[..] {
        [url] => Ok(Url::parse(url)?),
        _ => Err(anyhow!("Invalid url param!")),
    }
}
