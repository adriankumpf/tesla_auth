use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::time::Duration;

use anyhow::anyhow;
use reqwest::header::AUTHORIZATION;

use oauth2::basic::BasicClient;
use oauth2::reqwest::http_client;
use oauth2::url::{Host, Url};
use oauth2::{
    AccessToken, AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, ExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, StandardTokenResponse,
    TokenResponse, TokenType, TokenUrl,
};

use crate::htime;

const CLIENT_ID: &str = "ownerapi";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/authorize";
const TOKEN_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const TOKEN_URL_CN: &str = "https://auth.tesla.cn/oauth2/v3/token";
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
    oauth_client: BasicClient,
    oauth_client_cn: BasicClient,
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

        let oauth_client_cn = BasicClient::new(
            ClientId::new(CLIENT_ID.to_string()),
            None,
            AuthUrl::new(AUTH_URL.to_string()).unwrap(),
            Some(TokenUrl::new(TOKEN_URL_CN.to_string()).unwrap()),
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
            oauth_client_cn,
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
        issuer: &Url,
        exchange_sso_token: bool,
    ) -> anyhow::Result<Tokens> {
        if state != self.csrf_token.secret() {
            return Err(anyhow!("CSRF state does not match!"));
        }

        let client = match issuer.host() {
            Some(Host::Domain("auth.tesla.cn")) => self.oauth_client_cn,
            _global => self.oauth_client,
        };

        let sso_token: SsoToken = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(self.pkce_verifier)
            .request(http_client)?
            .try_into()?;

        let tokens = if exchange_sso_token {
            let oa_token = exchange_sso_access_token(&sso_token.access_token)?;

            Tokens {
                access: oa_token.access_token,
                refresh: sso_token.refresh_token,
                expires_in: oa_token.expires_in.into(),
            }
        } else {
            Tokens {
                access: sso_token.access_token,
                refresh: sso_token.refresh_token,
                expires_in: sso_token.expires_in.into(),
            }
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

#[derive(serde::Deserialize)]
struct OwnerApiTokenResponse {
    access_token: String,
    expires_in: u64,
}

struct OwnerApiToken {
    access_token: AccessToken,
    expires_in: Duration,
}

impl From<OwnerApiTokenResponse> for OwnerApiToken {
    fn from(oa: OwnerApiTokenResponse) -> Self {
        Self {
            access_token: AccessToken::new(oa.access_token.clone()),
            expires_in: Duration::from_secs(oa.expires_in),
        }
    }
}

fn exchange_sso_access_token(access_token: &AccessToken) -> anyhow::Result<OwnerApiToken> {
    let body = HashMap::from([
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("client_id", OA_CLIENT_ID),
        ("client_secret", OA_CLIENT_SECRET),
    ]);

    let req = reqwest::blocking::Client::new()
        .post(OA_TOKEN_URL)
        .header(AUTHORIZATION, format!("Bearer {}", access_token.secret()))
        .json(&body);

    let tokens = req
        .send()?
        .error_for_status()?
        .json::<OwnerApiTokenResponse>()?;

    Ok(tokens.into())
}
