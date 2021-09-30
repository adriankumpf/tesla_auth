use std::fmt;
use std::time::Duration;

use anyhow::anyhow;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::url::Url;
use oauth2::{
    AccessToken, AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};

const CLIENT_ID: &str = "ownerapi";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/authorize";
const TOKEN_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const REDIRECT_URL: &str = "https://auth.tesla.com/void/callback";

pub fn is_redirect_url(url: &Url) -> bool {
    url.to_string().starts_with(REDIRECT_URL)
}

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access: AccessToken,
    pub refresh: RefreshToken,
    pub expires_in: Duration,
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

{} hours
                "#,
            self.access.secret(),
            self.refresh.secret(),
            self.expires_in.as_secs() / (60 * 60)
        )
    }
}

pub struct Client {
    auth_url: Url,
    oauth_client: BasicClient,
    pkce_verifier: PkceCodeVerifier,
    csrf_token: CsrfToken,
}

impl Client {
    pub fn new() -> Client {
        let oauth_client = BasicClient::new(
            ClientId::new(CLIENT_ID.to_string()),
            None,
            AuthUrl::new(AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(TOKEN_URL.to_string()).unwrap()),
        )
        .set_auth_type(AuthType::RequestBody)
        .set_redirect_uri(RedirectUrl::new(REDIRECT_URL.to_string()).unwrap());

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
            auth_url,
            pkce_verifier,
            csrf_token,
        }
    }

    pub fn authorize_url(&self) -> Url {
        self.auth_url.clone()
    }

    pub fn retrieve_tokens(self, code: &str, state: &str) -> anyhow::Result<Tokens> {
        if state != self.csrf_token.secret() {
            return Err(anyhow!("CSRF state does not match!"));
        }

        let tokens = self
            .oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(self.pkce_verifier)
            .request(http_client)?;

        Ok(Tokens {
            access: tokens.access_token().clone(),
            refresh: tokens.refresh_token().unwrap().clone(),
            expires_in: (tokens.expires_in().unwrap()),
        })
    }
}
