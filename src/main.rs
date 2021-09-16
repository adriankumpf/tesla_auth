mod auth;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use log::{debug, info, LevelFilter};
use simple_logger::SimpleLogger;

use oauth2::url::Url;

use wry::application::accelerator::{Accelerator, SysMods};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop};
use wry::application::keyboard::KeyCode;
use wry::application::menu::{MenuBar as Menu, MenuItem, MenuItemAttributes, MenuType};
use wry::application::window::{Window, WindowBuilder};
use wry::http::ResponseBuilder;
use wry::webview::{RpcRequest, WebViewBuilder};
use wry::Value;

const INITIALIZATION_SCRIPT: &str = r#"
    (function () {
        window.addEventListener('DOMContentLoaded', function(event) {
            rpc.call('url', window.location.toString());
        });
    })();
"#;

#[derive(Debug, Clone)]
enum CustomEvent {
    Tokens(auth::Tokens),
}

fn main() -> wry::Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("reqwest", LevelFilter::Debug)
        .with_module_level("tesla_auth", LevelFilter::Debug)
        .init()
        .unwrap();

    let mut client = auth::Client::new();
    let auth_url = client.authorization_url();

    info!("Opening {} ...", auth_url);

    let event_loop = EventLoop::<CustomEvent>::with_user_event();
    let event_proxy = event_loop.create_proxy();

    let mut menu_bar_menu = Menu::new();
    let mut menu = Menu::new();

    menu.add_native_item(MenuItem::About("Todos".to_string()));
    menu.add_native_item(MenuItem::Services);
    menu.add_native_item(MenuItem::Separator);
    menu.add_native_item(MenuItem::Hide);
    let quit_item = menu.add_item(
        MenuItemAttributes::new("Quit")
            .with_accelerators(&Accelerator::new(SysMods::Cmd, KeyCode::KeyQ)),
    );
    menu.add_native_item(MenuItem::Copy);
    menu.add_native_item(MenuItem::Paste);

    menu_bar_menu.add_submenu("First menu", true, menu);

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .with_menu(menu_bar_menu)
        .build(&event_loop)
        .unwrap();

    let (tx, rx) = channel();

    let handler = move |_window: &Window, req: RpcRequest| {
        if req.method == "url" {
            let url = parse_url(req.params.unwrap());
            tx.send(url).unwrap();
        }

        None
    };

    thread::spawn(move || {
        let mut tokens_retrieved = false;

        while let Ok(url) = rx.recv() {
            if !auth::is_redirect_url(&url) || tokens_retrieved {
                debug!("URL changed: {}", &url);
                continue;
            }

            let query: HashMap<_, _> = url.query_pairs().collect();

            let state = query.get("state").expect("No state parameter found");
            let code = query.get("code").expect("No code parameter found");

            client.verify_csrf_state(state.to_string());

            let tokens = client.retrieve_tokens(code);

            tokens_retrieved = true;

            event_proxy.send_event(CustomEvent::Tokens(tokens)).unwrap();
        }
    });

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_custom_protocol("wry".into(), move |request| {
            let url: Url = request.uri().parse()?;

            match url.domain() {
                Some("index.html") => {
                    let query = url.query_pairs().collect::<HashMap<_, _>>();

                    let (access, refresh) =
                        (query.get("access").unwrap(), query.get("refresh").unwrap());

                    let content = include_str!("../views/index.html")
                        .replace("{access_token}", access)
                        .replace("{refresh_token}", refresh);

                    ResponseBuilder::new()
                        .mimetype("text/html")
                        .body(content.as_bytes().to_vec())
                }

                domain => unimplemented!("Cannot open {:?}", domain),
            }
        })
        .with_url(auth_url.as_str())?
        .with_rpc_handler(handler)
        .build()?;

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
                if menu_id == quit_item.clone().id() {
                    *control_flow = ControlFlow::Exit;
                }
                println!("Clicked on {:?}", menu_id);
            }
            _ => (),
        }
    });
}

fn parse_url(params: Value) -> Url {
    let args = serde_json::from_value::<Vec<String>>(params).unwrap();
    let url = args.first().unwrap();
    Url::parse(url).expect("Invalid URL")
}
