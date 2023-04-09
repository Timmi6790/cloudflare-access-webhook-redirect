use crate::error::Error;
use crate::Result;
use derive_new::new;
use regex::RegexSet;
use reqwest::header::HeaderValue;
use reqwest::Url;
use reqwest_middleware::ClientWithMiddleware;
use secrecy::{ExposeSecret, Secret};
use std::collections::{HashMap, HashSet};

#[derive(Getters)]
#[getset(get = "pub")]
pub struct WebHookData {
    client: ClientWithMiddleware,
    #[getset(skip)]
    target_host: Url,
    allowed_paths: AllowedPaths,
    access_id: HeaderValue,
    access_secret: HeaderValue,
}

impl WebHookData {
    pub fn new(
        client: ClientWithMiddleware,
        target_host: Url,
        allowed_paths: AllowedPaths,
        access_id: Secret<String>,
        access_secret: Secret<String>,
    ) -> Result<Self> {
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

    pub fn is_allowed_path(&self, path: &str, method: &actix_web::http::Method) -> bool {
        self.allowed_paths.is_allowed(path, method)
    }
}

#[derive(new, Getters)]
#[getset(get = "pub")]
pub struct AllowedPaths {
    allowed_paths: RegexSet,
    allowed_methods: HashMap<String, AllowedPath>,
}

impl AllowedPaths {
    pub fn is_allowed(&self, path: &str, method: &actix_web::http::Method) -> bool {
        let matches = self.allowed_paths.matches(path);
        matches
            .into_iter()
            .map(|i| self.allowed_paths.patterns().get(i).unwrap())
            .filter_map(|p| self.allowed_methods.get(p))
            .any(|p| p.is_allowed(method))
    }
}

#[derive(new)]
pub struct AllowedPath {
    all: bool,
    methods: HashSet<actix_web::http::Method>,
}

impl AllowedPath {
    pub fn is_allowed(&self, method: &actix_web::http::Method) -> bool {
        self.all || self.methods.contains(method)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::AllowedMethod;
    use crate::data::{AllowedPath, AllowedPaths, WebHookData};
    use lazy_static::lazy_static;
    use regex::RegexSet;
    use reqwest::Url;
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use secrecy::Secret;
    use std::collections::{HashMap, HashSet};
    use std::str::FromStr;

    lazy_static! {
        static ref ALL_HTTP_METHODS: Vec<actix_web::http::Method> = vec![
            actix_web::http::Method::GET,
            actix_web::http::Method::POST,
            actix_web::http::Method::PUT,
            actix_web::http::Method::DELETE,
            actix_web::http::Method::HEAD,
            actix_web::http::Method::CONNECT,
            actix_web::http::Method::OPTIONS,
            actix_web::http::Method::TRACE,
            actix_web::http::Method::PATCH,
        ];
    }
    pub struct TestWebHookData {
        client: ClientWithMiddleware,
        target_host: Url,
        allowed_paths: HashMap<String, HashSet<AllowedMethod>>,
        access_id: Secret<String>,
        access_secret: Secret<String>,
    }

    impl TestWebHookData {
        fn convert_paths(&self) -> AllowedPaths {
            let paths: Vec<String> = self.allowed_paths.keys().map(|s| s.to_string()).collect();
            let paths = RegexSet::new(paths).unwrap();

            let mut allowed_paths = HashMap::with_capacity(self.allowed_paths.len());
            for (path, methods) in &self.allowed_paths {
                let mut filtered_methods = HashSet::with_capacity(methods.len());
                let mut all = false;
                for method in methods {
                    if method == &AllowedMethod::ALL {
                        all = true;
                        continue;
                    }

                    let method = actix_web::http::Method::from_str(method.name()).unwrap();
                    filtered_methods.insert(method);
                }

                allowed_paths.insert(path.clone(), AllowedPath::new(all, filtered_methods));
            }

            AllowedPaths::new(paths, allowed_paths)
        }
    }

    impl Default for TestWebHookData {
        fn default() -> Self {
            let mut allowed_paths = HashMap::new();
            allowed_paths.insert(
                "/test".to_string(),
                vec![AllowedMethod::GET, AllowedMethod::POST]
                    .into_iter()
                    .collect(),
            );

            Self {
                client: ClientBuilder::new(reqwest::Client::new()).build(),
                target_host: Url::parse("https://example.com").unwrap(),
                allowed_paths,
                access_id: Secret::new("test id".to_string()),
                access_secret: Secret::new("test secret".to_string()),
            }
        }
    }

    impl From<TestWebHookData> for WebHookData {
        fn from(test_web_hook_data: TestWebHookData) -> Self {
            let paths = test_web_hook_data.convert_paths();
            WebHookData::new(
                test_web_hook_data.client,
                test_web_hook_data.target_host,
                paths,
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
            allowed_paths: HashMap::new(),
            ..Default::default()
        }
        .into();

        // Invalid paths
        ALL_HTTP_METHODS.iter().for_each(|method| {
            assert!(!web_hook_data.is_allowed_path("", method));
            assert!(!web_hook_data.is_allowed_path("/", method));
            assert!(!web_hook_data.is_allowed_path("/test", method));
            assert!(!web_hook_data.is_allowed_path("/test/", method));
            assert!(!web_hook_data.is_allowed_path("/test/123", method));
        });
    }

    #[test]
    fn test_is_allowed_path_no_regex() {
        let mut paths = HashMap::new();
        paths.insert(
            "/test/".to_string(),
            vec![AllowedMethod::ALL].into_iter().collect(),
        );

        paths.insert(
            "/data/123".to_string(),
            vec![AllowedMethod::ALL].into_iter().collect(),
        );

        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: paths,
            ..Default::default()
        }
        .into();

        // Invalid paths
        ALL_HTTP_METHODS.iter().for_each(|method| {
            assert!(!web_hook_data.is_allowed_path("", method));
            assert!(!web_hook_data.is_allowed_path("/", method));
            assert!(!web_hook_data.is_allowed_path("/api", method));
            assert!(web_hook_data.is_allowed_path("/test/access", method));
            assert!(!web_hook_data.is_allowed_path("/data", method));
        });

        // Valid paths
        ALL_HTTP_METHODS.iter().for_each(|method| {
            assert!(web_hook_data.is_allowed_path("/test/", method));
            assert!(web_hook_data.is_allowed_path("/data/123", method));
        });
    }

    #[test]
    fn test_is_allowed_path_regex() {
        let mut paths = HashMap::new();
        paths.insert(
            "/test/".to_string(),
            vec![AllowedMethod::GET, AllowedMethod::POST]
                .into_iter()
                .collect(),
        );

        paths.insert(
            r"/data/\d*/private".to_string(),
            vec![AllowedMethod::ALL].into_iter().collect(),
        );

        let web_hook_data: WebHookData = TestWebHookData {
            allowed_paths: paths,
            ..Default::default()
        }
        .into();

        // Invalid paths
        ALL_HTTP_METHODS.iter().for_each(|method| {
            assert!(!web_hook_data.is_allowed_path("", method));
            assert!(!web_hook_data.is_allowed_path("/", method));
            assert!(!web_hook_data.is_allowed_path("/api", method));
            assert!(!web_hook_data.is_allowed_path("/data", method));
            assert!(!web_hook_data.is_allowed_path("/data/123/", method));
            assert!(!web_hook_data.is_allowed_path("/data/123/test", method));
            assert!(!web_hook_data.is_allowed_path("/data/abc/private", method));
        });

        // Check /test/ path
        ALL_HTTP_METHODS.iter().for_each(|method| {
            if method == &actix_web::http::Method::GET || method == &actix_web::http::Method::POST {
                assert!(web_hook_data.is_allowed_path("/test/", method));
            } else {
                assert!(!web_hook_data.is_allowed_path("/test/", method));
            }
        });

        // Check /data/123/private path
        ALL_HTTP_METHODS.iter().for_each(|method| {
            assert!(web_hook_data.is_allowed_path("/data/123/private", method));
        });
    }
}
