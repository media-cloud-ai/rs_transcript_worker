

pub struct Newsbridge {
  token: String,
}

impl Newsbridge {
    pub async fn new(parameters: &WorkerParameters) -> Self {

        let authentication_body = Authentication {
            client_id: "lA5Q51oy97OPYjWkC64NgOLwSpkTdfQd".to_string(),
            username: "adminftv@ftv-staging.fr".to_string(),
            password: "AdminFTV@NB2021!".to_string(),
            connection: "Username-Password-Authentication".to_string(),
            grant_type: "password".to_string(),
            scope: "profile email".to_string(),
        };
        
        let client = reqwest::Client::new();

        let body = client.post("https://auth.newsbridge.io/oauth/token")
                            .header(ACCEPT, "*/*")
                            .header(CONTENT_TYPE, "application/json")
                            .json(&authentication_body)
                            .send()
                            .await?;
    }
}
