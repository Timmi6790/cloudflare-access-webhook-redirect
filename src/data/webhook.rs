use crate::error::Error;
use crate::Result;
use regex::RegexSet;
use reqwest::header::HeaderValue;
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;
use secrecy::{ExposeSecret, Secret};

#[derive(Getters)]
#[getset(get = "pub")]
pub struct WebHookData {
    client: ClientWithMiddleware,
    #[getset(skip)]
    target_host: Url,
    allowed_paths: RegexSet,
    access_id: HeaderValue,
    access_secret: HeaderValue,
}

impl WebHookData {
    pub fn new(
        client: ClientWithMiddleware,
        target_host: Url,
        allowed_paths: Vec<String>,
        access_id: Secret<String>,
        access_secret: Secret<String>,
    ) -> Result<Self> {
        let allowed_paths = RegexSet::new(allowed_paths)?;

        let access_id = HeaderValue::from_str(access_id.expose_secret())
            .map_err(|_| Error::custom("Failed to map access id to header value"))?;
        let access_secret = HeaderValue::from_str(access_secret.expose_secret())
            .map_err(|_| Error::custom("Failed to map access secret to header value"))?;
        Ok(Self {
            client,
            target_host,
            allowed_paths,
            access_id,
            access_secret,
        })
    }

    pub fn get_target_url(&self, path: &str) -> Result<Url> {
        self.target_host
            .join(path)
            .map_err(|e| Error::custom(format!("Failed to join URL: {}", e)))
    }

    pub fn is_allowed_path(&self, path: &str) -> bool {
        self.allowed_paths.is_match(path)
    }
}

#[cfg(test)]
mod tests {
    use crate::data::WebHookData;
    use reqwest::Url;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use secrecy::Secret;

    pub struct TestWebHookData {
        client: ClientWithMiddleware,
        target_host: Url,
        allowed_paths: Vec<String>,
        access_id: Secret<String>,
        access_secret: Secret<String>,
    }

    impl Default for TestWebHookData {
        fn default() -> Self {
            Self {
                client: ClientBuilder::new(reqwest::Client::new()).build(),
                target_host: Url::parse("https://example.com").unwrap(),
                allowed_paths: vec!["/test".to_string()],
                access_id: Secret::new("test id".to_string()),
                access_secret: Secret::new("test secret".to_string()),
            }
        }
    }

    impl From<TestWebHookData> for WebHookData {
        fn from(test_web_hook_data: TestWebHookData) -> Self {
            WebHookData::new(
                test_web_hook_data.client,
                test_web_hook_data.target_host,
                test_web_hook_data.allowed_paths,
                test_web_hook_data.access_id,
                test_web_hook_data.access_secret,
            )
            .unwrap()
        }
    }

    #[test]
    fn test_get_target_url() {
        let base_url = Url::parse("https://example.com").unwrap();
        let web_hook_data: WebHookData = TestWebHookData {
            target_host: base_url,
            ..Default::default()
        }
        .into();

        let target_url = web_hook_data.get_target_url("/test").unwrap();
        assert_eq!(target_url.as_str(), "https://example.com/test");
    }

    #[test]
    fn test_get_target_url_trailing_slash() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let web_hook_data: WebHookData = TestWebHookData {
            target_host: base_url,
            ..Default::default()
        }
        .into();

        let target_url = web_hook_data.get_target_url("/test").unwrap();
        assert_eq!(target_url.as_str(), "https://example.com/test");
    }

    #[test]
    fn test_is_allowed_path_invalid_empty() {
        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: vec![],
            ..Default::default()
        }
        .into();

        // Invalid paths
        assert!(!web_hook_data.is_allowed_path(""));
        assert!(!web_hook_data.is_allowed_path("/"));
        assert!(!web_hook_data.is_allowed_path("/test"));
        assert!(!web_hook_data.is_allowed_path("/test/"));
        assert!(!web_hook_data.is_allowed_path("/test/123"));
    }

    #[test]
    fn test_is_allowed_path_no_regex() {
        let paths = vec!["/test/".to_string(), "/data/123".to_string()];

        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: paths,
            ..Default::default()
        }
        .into();

        // Invalid paths
        assert!(!web_hook_data.is_allowed_path(""));
        assert!(!web_hook_data.is_allowed_path("/"));
        assert!(!web_hook_data.is_allowed_path("/api"));
        assert!(web_hook_data.is_allowed_path("/test/access"));
        assert!(!web_hook_data.is_allowed_path("/data"));

        // Valid paths
        assert!(web_hook_data.is_allowed_path("/test/"));
        assert!(web_hook_data.is_allowed_path("/data/123"));
    }

    #[test]
    fn test_is_allowed_path_regex() {
        let paths = vec!["/test/".to_string(), r"/data/\d*/private".to_string()];

        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: paths,
            ..Default::default()
        }
        .into();

        // Invalid paths
        assert!(!web_hook_data.is_allowed_path(""));
        assert!(!web_hook_data.is_allowed_path("/"));
        assert!(!web_hook_data.is_allowed_path("/api"));
        assert!(!web_hook_data.is_allowed_path("/data"));
        assert!(!web_hook_data.is_allowed_path("/data/123/"));
        assert!(!web_hook_data.is_allowed_path("/data/123/test"));
        assert!(!web_hook_data.is_allowed_path("/data/abc/private"));

        // Valid paths
        assert!(web_hook_data.is_allowed_path("/test/"));
        assert!(web_hook_data.is_allowed_path("/data/123/private"));
    }
}
