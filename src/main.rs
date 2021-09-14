mod auth;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use oauth2::url::Url;
use oauth2::AuthorizationCode;

use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::{ControlFlow, EventLoop};
use wry::application::window::{Window, WindowBuilder};
use wry::webview::{RpcRequest, WebViewBuilder};
use wry::Value;

const INITIALIZATION_SCRIPT: &str = r#"
    (function () {
        window.addEventListener('DOMContentLoaded', function(event) {
            rpc.call('url', window.location.toString());
        });
    })();
"#;

fn main() -> wry::Result<()> {
    let mut client = auth::Client::new();
    let auth_url = client.authorization_url();

    println!("Opening {} ...", auth_url);

    let event_loop = EventLoop::new();
    let event_proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .build(&event_loop)
        .unwrap();

    let (sender, receiver) = channel();

    let handler = move |_window: &Window, req: RpcRequest| {
        if req.method == "url" {
            let url = parse_url(req.params.unwrap());
            sender.send(url).unwrap();
        }

        None
    };

    let _webview = WebViewBuilder::new(window)
        .unwrap()
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_url(auth_url.as_str())?
        .with_rpc_handler(handler)
        .build()?;

    thread::spawn(move || {
        while let Ok(url) = receiver.recv() {
            if !auth::is_redirect_url(&url) {
                continue;
            }

            let query: HashMap<_, _> = url.query_pairs().collect();

            let state = query.get("state").expect("No state parameter found");
            let code = query.get("code").expect("No code parameter found");

            client.verify_csrf_state(state.to_string());

            let code = AuthorizationCode::new(code.to_string());
            let tokens = client.retrieve_tokens(code);

            println!(
                "Access Token:  {}\nRefresh Token:  {}",
                tokens.access, tokens.refresh
            );

            event_proxy.send_event(()).unwrap();

            break;
        }
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        Event::UserEvent(_event) => *control_flow = ControlFlow::Exit,
        _ => *control_flow = ControlFlow::Wait,
    });
}

fn parse_url(params: Value) -> Url {
    let args = serde_json::from_value::<Vec<String>>(params).unwrap();
    let url = args.first().unwrap();
    Url::parse(&url).expect("Invalid URL")
}
