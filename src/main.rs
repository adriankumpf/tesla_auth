use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::url::Url;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};

use reqwest;
use reqwest::header::AUTHORIZATION;

use serde::Deserialize;
use serde_json::Value;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread;

use wry::{
    application::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Window, WindowBuilder},
    },
    webview::{RpcRequest, RpcResponse, WebViewBuilder},
};

const INITIALIZATION_SCRIPT: &str = r#"
    (function () {
        window.addEventListener('DOMContentLoaded', async (event) => {
            await rpc.call('url', window.location.toString());
        });
    })();
"#;

#[derive(Deserialize, Debug)]
struct SsoTokenResponse {
    access_token: String,
}

fn main() -> wry::Result<()> {
    let (sender, receiver) = channel();

    let client = BasicClient::new(
        ClientId::new("ownerapi".to_string()),
        None,
        AuthUrl::new("https://auth.tesla.com/oauth2/v3/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://auth.tesla.com/oauth2/v3/token".to_string(),
        )?),
    )
    .set_auth_type(AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(
        "https://auth.tesla.com/void/callback".to_string(),
    )?);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("offline_access".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // println!("Browse to: {}", auth_url);

    let event_loop = EventLoop::new();
    let event_proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .build(&event_loop)
        .unwrap();

    let handler = move |_window: &Window, mut req: RpcRequest| {
        let mut response = None;

        if &req.method == "url" {
            if let Some(params) = req.params.take() {
                if let Ok(mut args) = serde_json::from_value::<Vec<String>>(params) {
                    let result = if !args.is_empty() {
                        let url = args.swap_remove(0);

                        let sender = sender.clone();
                        sender.send(url).unwrap();

                        Some(Value::String("ok".into()))
                    } else {
                        None
                    };

                    response = Some(RpcResponse::new_result(req.id.take(), result));
                }
            }
        }

        response
    };

    let _webview = WebViewBuilder::new(window)
        .unwrap()
        .with_initialization_script(INITIALIZATION_SCRIPT)
        .with_url(auth_url.as_str())?
        .with_rpc_handler(handler)
        .build()?;

    thread::spawn(move || {
        while let Ok(url) = receiver.recv() {
            let url = Url::parse(&url).unwrap();

            if url.path() != "/void/callback" {
                continue;
            }

            if let Some((_, state)) = url.query_pairs().find(|(key, _value)| key == "state") {
                assert_eq!(&state.to_string(), csrf_token.secret());
            };

            if let Some((_, code)) = url.query_pairs().find(|(key, _value)| key == "code") {
                let code = AuthorizationCode::new(code.into_owned());

                let token_result = client
                    .exchange_code(code)
                    .set_pkce_verifier(pkce_verifier)
                    .request(http_client)
                    .unwrap();

                let req_client = reqwest::blocking::Client::new();

                let mut body = HashMap::new();
                body.insert("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer");
                body.insert(
                    "client_id",
                    "81527cff06843c8634fdc09e8ac0abefb46ac849f38fe1e431c2ef2106796384",
                );
                body.insert(
                    "client_secret",
                    "c7257eb71a564034f9419ee651c7d0e5f7aa6bfbd18bafb5c5c033b093bb2fa3",
                );

                let tokens: SsoTokenResponse = req_client
                    .post("https://owner-api.teslamotors.com/oauth/token")
                    .header(
                        AUTHORIZATION,
                        format!("Bearer {}", token_result.access_token().secret()),
                    )
                    .json(&body)
                    .send()
                    .unwrap()
                    .json()
                    .unwrap();

                println!(
                    "Access Token:  {}\nRefresh Token:  {}",
                    tokens.access_token,
                    token_result.refresh_token().unwrap().secret()
                );

                event_proxy.send_event(()).unwrap();

                break;
            };
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(_event) => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
