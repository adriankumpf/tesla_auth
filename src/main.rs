use std::thread;

use log::LevelFilter;
use oauth2::url::{Position, Url};
use simple_logger::SimpleLogger;

use muda::{Menu, PredefinedMenuItem, Submenu};
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use tao::platform::unix::WindowExtUnix;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopClosed, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use wry::{WebView, WebViewBuilder};

mod auth;
mod htime;
mod view;

#[derive(Debug)]
enum UserEvent {
    /// The SSO flow reached the callback URL.
    Redirect(Url),
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

    /// print the version and exit
    #[argh(switch, short = 'v')]
    version: bool,
}

fn main() -> anyhow::Result<()> {
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    disable_dmabuf_renderer();

    let args: Args = argh::from_env();

    if args.version {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    init_logger(args.debug)?;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    let auth_client = auth::Client::new();

    let window = WindowBuilder::new()
        .with_title("Tesla Auth")
        .with_resizable(true)
        .build(&event_loop)?;

    // Kept alive for as long as the window: dropping it would tear down the
    // native menus it installed.
    let _menu_bar = build_menu_bar(&window)?;

    let nav_proxy = event_loop.create_proxy();
    let webview = build_webview(&window, args.debug, move |uri| {
        handle_navigation(&nav_proxy, uri)
    })?;

    // Must happen before the first navigation, otherwise the stale session
    // cookies we are supposed to drop are sent along with it.
    if args.clear_browsing_data {
        webview.clear_all_browsing_data()?;
    }

    webview.load_url(auth_client.authorize_url().as_str())?;

    // Taken by the first callback: `authenticate` consumes the client.
    let mut auth_client = Some(auth_client);
    let event_proxy = event_loop.create_proxy();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let page = match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
                return;
            }

            Event::UserEvent(UserEvent::Redirect(url)) => {
                if let Some(client) = auth_client.take() {
                    spawn_token_exchange(client, event_proxy.clone(), url);
                }
                view::progress()
            }

            Event::UserEvent(UserEvent::Tokens(tokens)) => {
                println!("{tokens}");
                view::tokens(&tokens)
            }

            Event::UserEvent(UserEvent::Failure(error)) => {
                log::error!("{error}");
                view::error(&error)
            }

            Event::UserEvent(UserEvent::LoginCanceled) => {
                log::warn!("Login canceled");
                *control_flow = ControlFlow::Exit;
                return;
            }

            _ => return,
        };

        if let Err(e) = webview.load_html(&page) {
            log::error!("Failed to render page: {e}");
        }
    });
}

/// Opts out of WebKitGTK's DMA-BUF renderer.
///
/// It fails to allocate buffers on a number of drivers — the NVIDIA proprietary
/// one above all — which shows up as `Failed to create GBM buffer`, a blank
/// window, or a window that vanishes as soon as it opens. A login form has
/// nothing to gain from GPU compositing, so take the safe path by default;
/// `WEBKIT_DISABLE_DMABUF_RENDERER=0` puts it back.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn disable_dmabuf_renderer() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

    if std::env::var_os(VAR).is_none() {
        // SAFETY: called as the first statement of `main`, so the process is
        // still single-threaded. It must also stay ahead of the event loop,
        // which brings up GTK and with it threads that read the environment.
        unsafe { std::env::set_var(VAR, "1") };
    }
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

fn build_menu_bar(window: &Window) -> anyhow::Result<Menu> {
    let menu_bar = Menu::new();

    #[cfg(target_os = "macos")]
    let app_menu = Submenu::with_items(
        "App",
        true,
        &[
            &PredefinedMenuItem::about(None, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ],
    )?;

    let edit_menu = Submenu::with_items(
        "&Edit",
        true,
        &[
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::undo(None),
            #[cfg(target_os = "macos")]
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
        ],
    )?;

    // The predicates mirror muda's platform support: `fullscreen` exists on
    // macOS only, `minimize` everywhere but the GTK backend.
    #[cfg(target_os = "macos")]
    let view_menu = Submenu::with_items("&View", true, &[&PredefinedMenuItem::fullscreen(None)])?;

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let window_menu = Submenu::with_items("&Window", true, &[&PredefinedMenuItem::minimize(None)])?;

    menu_bar.append_items(&[
        #[cfg(target_os = "macos")]
        &app_menu,
        &edit_menu,
        #[cfg(target_os = "macos")]
        &view_menu,
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        &window_menu,
    ])?;

    #[cfg(target_os = "windows")]
    unsafe {
        menu_bar.init_for_hwnd(window.hwnd() as _)?;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    menu_bar.init_for_gtk_window(window.gtk_window(), window.default_vbox())?;
    #[cfg(target_os = "macos")]
    {
        // The menu bar belongs to the application rather than to any window.
        let _ = window;
        menu_bar.init_for_nsapp();
    }

    Ok(menu_bar)
}

fn build_webview(
    window: &Window,
    devtools: bool,
    navigation_handler: impl Fn(String) -> bool + 'static,
) -> anyhow::Result<WebView> {
    let builder = WebViewBuilder::new()
        .with_navigation_handler(navigation_handler)
        .with_clipboard(true)
        .with_devtools(devtools);

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(window)?;

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use wry::WebViewBuilderExtUnix;
        let vbox = window
            .default_vbox()
            .ok_or_else(|| anyhow::anyhow!("Window has no GTK container"))?;
        builder.build_gtk(vbox)?
    };

    Ok(webview)
}

/// Decides whether the webview may follow a navigation.
///
/// The callback URL uses a `tesla://` scheme that nothing on the system handles,
/// so following it would at best fail and at worst hand the authorization code
/// to whichever application happens to claim the scheme. Cancel it and take over
/// the window instead.
///
/// Everything else is allowed through, a URI we cannot parse included: the
/// handler also sees subframe navigations, so cancelling one risks taking an
/// embedded captcha down with it, and a URI that fails to parse is by
/// definition not the callback.
fn handle_navigation(event_proxy: &EventLoopProxy<UserEvent>, uri: String) -> bool {
    let Ok(url) = Url::parse(&uri) else {
        log::debug!("Navigating to a URL we could not parse ...");
        return true;
    };

    if !auth::is_redirect_url(&url) {
        // Everything past the path is redacted: it carries the CSRF state.
        log::debug!("Navigating to {} ...", &url[..Position::AfterPath]);
        return true;
    }

    let _ = event_proxy.send_event(UserEvent::Redirect(url));
    false
}

/// Exchanges the authorization code on a background thread; the request blocks
/// and would otherwise freeze the event loop.
fn spawn_token_exchange(
    client: auth::Client,
    event_proxy: EventLoopProxy<UserEvent>,
    callback_url: Url,
) {
    thread::spawn(move || {
        let event = match client.authenticate(&callback_url) {
            Ok(auth::Outcome::Authorized(tokens)) => UserEvent::Tokens(tokens),
            Ok(auth::Outcome::Canceled) => UserEvent::LoginCanceled,
            Err(error) => UserEvent::Failure(error),
        };

        // The window may have been closed while the exchange was in flight. The
        // tokens exist either way, so fall back to stdout rather than lose them.
        if let Err(EventLoopClosed(event)) = event_proxy.send_event(event) {
            match event {
                UserEvent::Tokens(tokens) => println!("{tokens}"),
                UserEvent::Failure(error) => log::error!("{error}"),
                _ => (),
            }
        }
    });
}
