use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub db_folder: String,
    pub apprise_url: String,
    pub apprise_tag_prefix: String,
    pub dev_username: String,
    pub dev_name: String,
    pub port: u16,
    pub profile: String,
}

impl Config {
    pub fn from_env() -> Self {
        let db_folder = env::var("DB_FOLDER").unwrap_or_else(|_| ".".to_string());
        let apprise_url = env::var("APPRISE_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:8000/notify".to_string());
        let apprise_tag_prefix =
            env::var("APPRISE_TAG_PREFIX").unwrap_or_else(|_| "reminders".to_string());
        let dev_username = env::var("DEV_USERNAME").unwrap_or_else(|_| "dev_user".to_string());
        let dev_name = env::var("DEV_NAME").unwrap_or_else(|_| "Dev User".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080);
        let profile = env::var("APP_PROFILE").unwrap_or_else(|_| "prod".to_string());

        Config {
            db_folder,
            apprise_url,
            apprise_tag_prefix,
            dev_username,
            dev_name,
            port,
            profile,
        }
    }
}
