use url::Url;

pub enum ResponseTokens {
    File(Url),
    Text(String),
}

#[derive(Default)]
pub struct Response {
    pub tokens: Vec<ResponseTokens>,
}

impl Response {
    pub fn file(mut self, url: Url) -> Self {
        self.tokens.push(ResponseTokens::File(url));
        self
    }

    pub fn text(mut self, text: impl ToString) -> Self {
        self.tokens.push(ResponseTokens::Text(text.to_string()));
        self
    }
}
