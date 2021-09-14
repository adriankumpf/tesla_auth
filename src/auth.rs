use std::collections::HashMap;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::url::Url;
use oauth2::{
    AccessToken, AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};

use reqwest::header::AUTHORIZATION;

use serde::Deserialize;

const CLIENT_ID: &str = "ownerapi";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/authorize";
const TOKEN_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const REDIRECT_URL: &str = "https://auth.tesla.com/void/callback";

const SSO_CLIENT_ID: &str = "81527cff06843c8634fdc09e8ac0abefb46ac849f38fe1e431c2ef2106796384";
const SSO_CLIENT_SECRET: &str = "c7257eb71a564034f9419ee651c7d0e5f7aa6bfbd18bafb5c5c033b093bb2fa3";
const SSO_TOKEN_URL: &str = "https://owner-api.teslamotors.com/oauth/token";

pub fn is_redirect_url(url: &Url) -> bool {
    url.to_string().starts_with(REDIRECT_URL)
}

#[derive(Deserialize, Debug)]
struct SsoTokenResponse {
    access_token: String,
}

#[derive(Debug)]
pub struct Tokens {
    pub access: String,
    pub refresh: String,
}

pub struct Client {
    oauth_client: BasicClient,
    pkce_verifier: Option<PkceCodeVerifier>,
    csrf_token: Option<CsrfToken>,
}

impl Client {
    pub fn new() -> Self {
        let client = BasicClient::new(
            ClientId::new(CLIENT_ID.to_string()),
            None,
            AuthUrl::new(AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(TOKEN_URL.to_string()).unwrap()),
        )
        .set_auth_type(AuthType::RequestBody)
        .set_redirect_uri(RedirectUrl::new(REDIRECT_URL.to_string()).unwrap());

        Client {
            oauth_client: client,
            pkce_verifier: None,
            csrf_token: None,
        }
    }

    pub fn authorization_url(&mut self) -> Url {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self
            .oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("offline_access".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        self.pkce_verifier = Some(pkce_verifier);
        self.csrf_token = Some(csrf_token);

        auth_url
    }

    pub fn verify_csrf_state(&self, state: String) {
        let csrf_token = self.csrf_token.as_ref().unwrap();
        assert_eq!(&state, csrf_token.secret());
    }

    pub fn retrieve_tokens(self, code: AuthorizationCode) -> Tokens {
        let tokens = self
            .oauth_client
            .exchange_code(code)
            .set_pkce_verifier(self.pkce_verifier.unwrap())
            .request(http_client)
            .unwrap();

        let access_token = exchange_sso_access_token(tokens.access_token());
        let refresh_token = tokens.refresh_token().unwrap().secret().to_string();

        Tokens {
            access: access_token,
            refresh: refresh_token,
        }
    }
}

fn exchange_sso_access_token(access_token: &AccessToken) -> String {
    let mut body = HashMap::new();
    body.insert("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer");
    body.insert("client_id", SSO_CLIENT_ID);
    body.insert("client_secret", SSO_CLIENT_SECRET);

    let tokens: SsoTokenResponse = reqwest::blocking::Client::new()
        .post(SSO_TOKEN_URL)
        .header(AUTHORIZATION, format!("Bearer {}", access_token.secret()))
        .json(&body)
        .send()
        .unwrap()
        .json()
        .unwrap();

    tokens.access_token
}
