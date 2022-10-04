use anyhow::Result;
use serde::Deserialize;

use crate::User;

#[derive(Deserialize)]
struct LoginUserData {
    #[serde(rename = "subscriptionKey")]
    subscription_key: String,
}
#[derive(Deserialize)]
struct LoginData {
    #[serde(rename = "userData")]
    user_data: LoginUserData,
}
pub async fn login(user: &User) -> Result<String> {
    let token = reqwest::Client::new()
        .post("https://www.flightradar24.com/user/login")
        .form(&[
            ("remember", "true"),
            ("type", "web"),
            ("email", &user.mail),
            ("password", &user.password),
        ])
        .header("Origin", "https://www.flightradar24.com")
        .header("Referer", "https://www.flightradar24.com")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 6.1; WOW64; rv:28.0) Gecko/20100101 Firefox/28.0",
        )
        .send()
        .await?
        .json::<LoginData>()
        .await?
        .user_data
        .subscription_key;

    Ok(token)
}