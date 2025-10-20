use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use utoipa::{IntoParams, ToSchema};
use wither::bson::{self, doc};

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SortingQuery {
    #[serde(deserialize_with = "deserialize_single_or_seq", default)]
    pub sort_by: Option<Vec<String>>,
    #[serde(deserialize_with = "deserialize_single_or_seq", default)]
    pub order_type: Option<Vec<String>>,
}

impl fmt::Display for SortingQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sort_by = self
            .sort_by
            .as_ref()
            .map_or("".to_string(), |v| v.join(","));
        let order_type = self
            .order_type
            .as_ref()
            .map_or("".to_string(), |v| v.join(","));
        write!(f, "sort_by:{sort_by}:order_type:{order_type}")
    }
}

impl SortingQuery {
    pub fn sort_doc(&self) -> Option<bson::Document> {
        self.sort_by.as_ref()?;

        let mut sort_doc = doc! {};
        let sort_by = self.sort_by.as_ref().unwrap();
        let binding = vec!["asc".to_string(); sort_by.len()];
        let order_type = self.order_type.as_ref().unwrap_or(&binding);

        for (field, order) in sort_by.iter().zip(order_type.iter()) {
            let order_value = match order.as_str() {
                "desc" => -1,
                _ => 1, // Default to ascending for any other value or empty string
            };
            sort_doc.insert(field.to_string(), order_value);
        }
        Some(sort_doc)
    }
}

pub fn deserialize_single_or_seq<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    match opt {
        Some(serde_json::Value::String(s)) => {
            Ok(Some(s.split(',').map(|s| s.trim().to_string()).collect()))
        }
        Some(serde_json::Value::Array(arr)) => {
            let mut vec = Vec::new();
            for item in arr {
                if let serde_json::Value::String(s) = item {
                    vec.push(s);
                } else {
                    return Err(serde::de::Error::custom("Expected a string"));
                }
            }
            Ok(Some(vec))
        }
        _ => Ok(None),
    }
}
