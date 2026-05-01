use crate::models::span::AttributeValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resource {
    pub service_name: String,
    pub attributes: HashMap<String, AttributeValue>,
}
