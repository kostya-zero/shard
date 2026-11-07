use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EchoParams {
    /// Text to return back.
    pub text: String,
}
