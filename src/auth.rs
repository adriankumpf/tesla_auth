use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::url::Url;
use oauth2::{
    AccessToken, AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};

use crate::htime::FormattedDuration;

const CLIENT_ID: &str = "ownerapi";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/authorize";
const TOKEN_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const REDIRECT_URL: &str = "https://auth.tesla.com/void/callback";

const OA_CLIENT_ID: &str = "81527cff06843c8634fdc09e8ac0abefb46ac849f38fe1e431c2ef2106796384";
const OA_CLIENT_SECRET: &str = "c7257eb71a564034f9419ee651c7d0e5f7aa6bfbd18bafb5c5c033b093bb2fa3";
const OA_TOKEN_URL: &str = "https://owner-api.teslamotors.com/oauth/token";

pub fn is_redirect_url(url: &Url) -> bool {
    url.to_string().starts_with(REDIRECT_URL)
}

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access: AccessToken,
    pub refresh: RefreshToken,
    pub expires_in: FormattedDuration,
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

    pub fn retrieve_tokens(
        self,
        code: &str,
        state: &str,
        exchange_sso_token: bool,
    ) -> anyhow::Result<Tokens> {
        if state != self.csrf_token.secret() {
            return Err(anyhow!("CSRF state does not match!"));
        }

        let tokens = self
            .oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(self.pkce_verifier)
            .request(http_client)?;

        let tokens = if exchange_sso_token {
            let oa_response = exchange_sso_access_token(tokens.access_token())?;

            Tokens {
                access: oa_response.access_token(),
                refresh: tokens.refresh_token().unwrap().clone(),
                expires_in: FormattedDuration::new(oa_response.expires_in()),
            }
        } else {
            Tokens {
                access: tokens.access_token().clone(),
                refresh: tokens.refresh_token().unwrap().clone(),
                expires_in: FormattedDuration::new(tokens.expires_in().unwrap()),
            }
        };

        Ok(tokens)
    }
}

#[derive(Deserialize, Debug)]
struct OwnerApiTokenResponse {
    access_token: String,
    expires_in: u64,
}

impl OwnerApiTokenResponse {
    fn access_token(&self) -> AccessToken {
        AccessToken::new(self.access_token.clone())
    }
    fn expires_in(&self) -> Duration {
        Duration::from_secs(self.expires_in)
    }
}

fn exchange_sso_access_token(access_token: &AccessToken) -> anyhow::Result<OwnerApiTokenResponse> {
    let mut body = HashMap::new();
    body.insert("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer");
    body.insert("client_id", OA_CLIENT_ID);
    body.insert("client_secret", OA_CLIENT_SECRET);

    let tokens = reqwest::blocking::Client::new()
        .post(OA_TOKEN_URL)
        .header(AUTHORIZATION, format!("Bearer {}", access_token.secret()))
        .json(&body)
        .send()?
        .error_for_status()?
        .json()?;

    Ok(tokens)
}
