use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl,
};
use url::Url;
use anyhow::Result;

pub struct GitHubAuth {
    client: BasicClient,
}

impl GitHubAuth {
    pub fn new() -> Self {
        let client_id = ClientId::new("GITHUB_CLIENT_ID".to_string());
        let client_secret = ClientSecret::new("GITHUB_CLIENT_SECRET".to_string());
        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap();
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap();

        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(RedirectUrl::new("dellij://auth".to_string()).unwrap());

        Self { client }
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("repo".to_string()))
            .add_scope(Scope::new("user".to_string()))
            .url()
    }
}
