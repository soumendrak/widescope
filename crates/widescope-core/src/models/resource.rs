use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::models::span::AttributeValue;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resource {
    pub service_name: String,
    pub attributes: HashMap<String, AttributeValue>,
}
