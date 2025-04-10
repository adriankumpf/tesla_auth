use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::time::Duration;

use anyhow::anyhow;

use oauth2::basic::BasicClient;
use oauth2::reqwest;
use oauth2::url::{Host, Url};
use oauth2::{
    AccessToken, AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, ExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, StandardTokenResponse,
    TokenResponse, TokenType, TokenUrl,
};
use oauth2::{EndpointNotSet, EndpointSet};

use crate::htime;

const CLIENT_ID: &str = "ownerapi";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/authorize";
const TOKEN_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const TOKEN_URL_CN: &str = "https://auth.tesla.cn/oauth2/v3/token";
const REDIRECT_URL: &str = "https://auth.tesla.com/void/callback";

pub fn is_redirect_url(url: &Url) -> bool {
    url.to_string().starts_with(REDIRECT_URL)
}

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access: AccessToken,
    pub refresh: RefreshToken,
    pub expires_in: htime::Duration,
}

impl fmt::Display for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"
--------------------------------- ACCESS TOKEN ---------------------------------

{}

--------------------------------- REFRESH TOKEN --------------------------------

{}

----------------------------------- VALID FOR ----------------------------------

{}
                "#,
            self.access.secret(),
            self.refresh.secret(),
            self.expires_in
        )
    }
}

pub struct Client {
    auth_url: Url,
    oauth_client:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    oauth_client_cn:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
    pkce_verifier: PkceCodeVerifier,
    csrf_token: CsrfToken,
}

impl Client {
    pub fn new() -> Client {
        let auth_url =
            AuthUrl::new(AUTH_URL.to_string()).expect("Invalid authorization endpoint URL");
        let redirect_url =
            RedirectUrl::new(REDIRECT_URL.to_string()).expect("Invalid redirect URL");

        let token_url = TokenUrl::new(TOKEN_URL.to_string()).expect("Invalid token endpoint URL");

        let token_url_cn =
            TokenUrl::new(TOKEN_URL_CN.to_string()).expect("Invalid token endpoint URL");

        let oauth_client = BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_auth_type(AuthType::RequestBody)
            .set_redirect_uri(redirect_url.clone())
            .set_auth_uri(auth_url.clone())
            .set_token_uri(token_url.clone());

        let oauth_client_cn = BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_auth_type(AuthType::RequestBody)
            .set_redirect_uri(redirect_url)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url_cn);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("offline_access".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Client {
            oauth_client,
            oauth_client_cn,
            auth_url,
            pkce_verifier,
            csrf_token,
        }
    }

    pub fn authorize_url(&self) -> Url {
        self.auth_url.clone()
    }

    pub fn retrieve_tokens(self, code: &str, state: &str, issuer: &Url) -> anyhow::Result<Tokens> {
        if state != self.csrf_token.secret() {
            return Err(anyhow!("CSRF state does not match!"));
        }

        let client = match issuer.host() {
            Some(Host::Domain("auth.tesla.cn")) => self.oauth_client_cn,
            _global => self.oauth_client,
        };

        let http_client = reqwest::blocking::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");

        let sso_token: SsoToken = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(self.pkce_verifier)
            .request(&http_client)?
            .try_into()?;

        let tokens = Tokens {
            access: sso_token.access_token,
            refresh: sso_token.refresh_token,
            expires_in: sso_token.expires_in.into(),
        };

        Ok(tokens)
    }
}

struct SsoToken {
    access_token: AccessToken,
    refresh_token: RefreshToken,
    expires_in: Duration,
}

impl<EF, TT> TryFrom<StandardTokenResponse<EF, TT>> for SsoToken
where
    EF: ExtraTokenFields,
    TT: TokenType,
{
    type Error = anyhow::Error;

    fn try_from(sso: StandardTokenResponse<EF, TT>) -> Result<Self, Self::Error> {
        let access_token = sso.access_token().clone();

        let refresh_token = sso
            .refresh_token()
            .cloned()
            .ok_or_else(|| anyhow!("refresh_token field missing"))?;

        let expires_in = sso
            .expires_in()
            .ok_or_else(|| anyhow!("expires_in field missing"))?;

        Ok(Self {
            access_token,
            refresh_token,
            expires_in,
        })
    }
}
