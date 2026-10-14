//! Typed JSON <-> Firestore adapter. Commits make review and release changes atomic.
use crate::config::Config;
use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, RequestBuilder};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Map, Value};

#[derive(Clone)]
pub struct Firestore {
    client: Client,
    base: String,
    resource: String,
    emulator: bool,
}
impl Firestore {
    pub fn new(config: &Config) -> Self {
        let resource = format!("projects/{}/databases/(default)/documents", config.project);
        let origin = config
            .emulator
            .as_ref()
            .map(|host| format!("http://{host}/v1"))
            .unwrap_or_else(|| "https://firestore.googleapis.com/v1".into());
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap(),
            base: format!("{origin}/{resource}"),
            resource,
            emulator: config.emulator.is_some(),
        }
    }
    async fn auth(&self, request: RequestBuilder) -> Result<RequestBuilder> {
        if self.emulator {
            Ok(request)
        } else {
            Ok(request.bearer_auth(access_token(&self.client).await?))
        }
    }
    // Retry only idempotent reads. Commits may have ambiguous outcomes and are not retried.
    async fn read(&self, request: RequestBuilder) -> Result<reqwest::Response> {
        let request = self.auth(request).await?;
        for attempt in 0..3 {
            let response = request
                .try_clone()
                .context("read request cannot be cloned")?
                .send()
                .await;
            match response {
                Ok(response)
                    if attempt < 2
                        && (response.status().is_server_error()
                            || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS) => {}
                Ok(response) => return Ok(response),
                Err(error) if attempt == 2 => return Err(error.into()),
                Err(_) => {}
            }
            tokio::time::sleep(std::time::Duration::from_millis(100 << attempt)).await;
        }
        unreachable!("bounded attempts return on the final attempt")
    }
    pub async fn get<T: DeserializeOwned>(&self, collection: &str, id: &str) -> Result<Option<T>> {
        let response = self
            .read(self.client.get(format!("{}/{collection}/{id}", self.base)))
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let doc: Value = response.error_for_status()?.json().await?;
        Ok(Some(serde_json::from_value(decode(
            &doc["fields"]["data"],
        )?)?))
    }
    pub async fn list<T: DeserializeOwned>(&self, collection: &str) -> Result<Vec<T>> {
        let mut records = Vec::new();
        let mut token = String::new();
        loop {
            let response = self
                .read(
                    self.client
                        .get(format!("{}/{collection}", self.base))
                        .query(&[("pageSize", "300"), ("pageToken", &token)]),
                )
                .await?;
            let page: Value = response.error_for_status()?.json().await?;
            if let Some(docs) = page["documents"].as_array() {
                for doc in docs {
                    records.push(serde_json::from_value(decode(&doc["fields"]["data"])?)?);
                }
            }
            token = page["nextPageToken"].as_str().unwrap_or_default().into();
            if token.is_empty() {
                break;
            }
            if records.len() > 100_000 {
                bail!("collection exceeds V1 safety limit");
            }
        }
        Ok(records)
    }
    pub async fn set<T: Serialize>(&self, collection: &str, id: &str, data: &T) -> Result<()> {
        self.commit(vec![(
            collection.into(),
            id.into(),
            serde_json::to_value(data)?,
        )])
        .await
    }
    pub async fn commit(&self, rows: Vec<(String, String, Value)>) -> Result<()> {
        let writes: Vec<Value> = rows
            .into_iter()
            .map(|(c, id, data)| {
                json!({"update": {
                    "name": format!("{}/{c}/{id}", self.resource), "fields": {"data": encode(&data)}
                }})
            })
            .collect();
        self.auth(
            self.client
                .post(format!("{}:commit", self.base))
                .json(&json!({"writes": writes})),
        )
        .await?
        .send()
        .await?
        .error_for_status()?;
        Ok(())
    }
    pub async fn ping(&self) -> Result<()> {
        self.get::<Value>("system", "active").await?;
        Ok(())
    }
}

pub async fn access_token(client: &Client) -> Result<String> {
    if let Ok(token) = std::env::var("GOOGLE_OAUTH_ACCESS_TOKEN") {
        return Ok(token);
    }
    let value: Value = client.get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
        .header("Metadata-Flavor", "Google").send().await?.error_for_status()?.json().await?;
    value["access_token"]
        .as_str()
        .map(String::from)
        .context("metadata token missing")
}
fn encode(value: &Value) -> Value {
    match value {
        Value::Null => json!({"nullValue": null}),
        Value::Bool(v) => json!({"booleanValue": v}),
        Value::Number(v) if v.is_i64() || v.is_u64() => json!({"integerValue": v.to_string()}),
        Value::Number(v) => json!({"doubleValue": v}),
        Value::String(v) => json!({"stringValue": v}),
        Value::Array(v) => {
            json!({"arrayValue": {"values": v.iter().map(|item| if item.is_array() { json!({"mapValue":{"fields":{"__qc_nested_array":encode(item)}}}) } else { encode(item) }).collect::<Vec<_>>()}})
        }
        Value::Object(v) => {
            json!({"mapValue": {"fields": v.iter().map(|(k,v)| (k.clone(),encode(v))).collect::<Map<_,_>>()}})
        }
    }
}
fn decode(value: &Value) -> Result<Value> {
    if value.get("nullValue").is_some() {
        return Ok(Value::Null);
    }
    for key in ["booleanValue", "stringValue", "doubleValue"] {
        if let Some(v) = value.get(key) {
            return Ok(v.clone());
        }
    }
    if let Some(v) = value.get("integerValue") {
        let s = v
            .as_str()
            .ok_or_else(|| anyhow!("integer must be a string"))?;
        return Ok(serde_json::from_str(s)?);
    }
    if let Some(v) = value.get("arrayValue") {
        return Ok(Value::Array(
            v["values"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(decode)
                .collect::<Result<_>>()?,
        ));
    }
    if let Some(v) = value.get("mapValue") {
        let fields = v["fields"].as_object().cloned().unwrap_or_default();
        if fields.len() == 1 && fields.contains_key("__qc_nested_array") {
            return decode(&fields["__qc_nested_array"]);
        }
        return Ok(Value::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), decode(v)?)))
                .collect::<Result<_>>()?,
        ));
    }
    bail!("unsupported Firestore value")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serde_roundtrip() {
        let data = json!({"n":42,"score":0.72,"review":null,"nested":{"ok":true},"list":["crazing",1],"matrix":[[1,2],[3,4]]});
        assert_eq!(decode(&encode(&data)).unwrap(), data);
    }
}
