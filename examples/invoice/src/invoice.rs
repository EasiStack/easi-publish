//! Invoice data types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::validation::{BoundedString, BoundedVec, NonNegF64};

#[derive(Debug, Deserialize, Serialize)]
pub struct ContactInfo {
    pub name: String,
    pub address: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LineItem {
    pub description: BoundedString<0, 500>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub quantity: NonNegF64,
    pub unit_price: String,
    pub amount: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceData {
    pub invoice_number: BoundedString<0, 200>,
    pub date_issued: String,
    pub date_due: String,
    pub currency: String,
    pub currency_symbol: String,
    pub from: ContactInfo,
    pub to: ContactInfo,
    pub items: BoundedVec<LineItem, 1, 100>,
    pub subtotal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<String>,
    pub total: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}
