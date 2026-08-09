use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use anyhow::anyhow;

use oauth2::basic::BasicClient;
use oauth2::reqwest;
use oauth2::url::Url;
use oauth2::{
    AccessToken, AuthType, AuthUrl, AuthorizationCode, ClientId, CsrfToken, ExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, StandardTokenResponse,
    TokenResponse, TokenType, TokenUrl,
};
use oauth2::{EndpointNotSet, EndpointSet};

use crate::htime;

type ConfiguredClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

const CLIENT_ID: &str = "ownerapi";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/authorize";
const TOKEN_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const TOKEN_URL_CN: &str = "https://auth.tesla.cn/oauth2/v3/token";
const REDIRECT_URL: &str = "tesla://auth/callback";
const SCOPES: &[&str] = &["openid", "email", "offline_access"];

/// Without a timeout an unresponsive endpoint would leave the window stuck
/// mid-flow forever.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

pub fn is_redirect_url(url: &Url) -> bool {
    url.as_str().starts_with(REDIRECT_URL)
}

#[derive(Debug)]
pub enum Outcome {
    Authorized(Tokens),
    Canceled,
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
    authorize_url: Url,
    oauth_client: ConfiguredClient,
    token_url_cn: TokenUrl,
    pkce_verifier: PkceCodeVerifier,
    csrf_token: CsrfToken,
}

impl Client {
    pub fn new() -> Client {
        let oauth_client = BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_auth_type(AuthType::RequestBody)
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URL.to_string()).expect("valid URL"))
            .set_auth_uri(AuthUrl::new(AUTH_URL.to_string()).expect("valid URL"))
            .set_token_uri(TokenUrl::new(TOKEN_URL.to_string()).expect("valid URL"));

        let token_url_cn = TokenUrl::new(TOKEN_URL_CN.to_string()).expect("valid URL");

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (authorize_url, csrf_token) = oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(SCOPES.iter().map(|scope| Scope::new(scope.to_string())))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Client {
            authorize_url,
            oauth_client,
            token_url_cn,
            pkce_verifier,
            csrf_token,
        }
    }

    pub fn authorize_url(&self) -> &Url {
        &self.authorize_url
    }

    /// Exchanges the authorization code carried by the callback URL for a set
    /// of tokens.
    ///
    /// Consumes the client: both the PKCE verifier and the authorization code
    /// are single-use.
    pub fn authenticate(self, callback_url: &Url) -> anyhow::Result<Outcome> {
        let query: HashMap<_, _> = callback_url.query_pairs().collect();

        if query
            .get("error")
            .is_some_and(|error| error == "login_cancelled")
        {
            return Ok(Outcome::Canceled);
        }

        let (Some(code), Some(state), Some(issuer)) =
            (query.get("code"), query.get("state"), query.get("issuer"))
        else {
            return Err(anyhow!(
                "Callback URL is missing required query parameters (code, state or issuer)"
            ));
        };

        if state != self.csrf_token.secret() {
            return Err(anyhow!("CSRF state does not match!"));
        }

        let issuer = Url::parse(issuer).map_err(|e| anyhow!("Invalid issuer URL: {e}"))?;

        // Accounts registered in China are issued tokens by a separate endpoint.
        let oauth_client = if issuer.host() == self.token_url_cn.url().host() {
            self.oauth_client.set_token_uri(self.token_url_cn)
        } else {
            self.oauth_client
        };

        let http_client = reqwest::blocking::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(EXCHANGE_TIMEOUT)
            .build()?;

        let sso_token: SsoToken = oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(self.pkce_verifier)
            .request(&http_client)?
            .try_into()?;

        Ok(Outcome::Authorized(Tokens {
            access: sso_token.access_token,
            refresh: sso_token.refresh_token,
            expires_in: sso_token.expires_in.into(),
        }))
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
