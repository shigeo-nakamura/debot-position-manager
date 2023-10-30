use crate::ReasonForClose;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum TradeAction {
    #[default]
    BuyOpen,
    BuyClose,
    SellOpen,
    SellClose,
}

#[derive(Debug, Clone, Default)]
pub struct TradeChance {
    pub trader_name: String,
    pub dex_index: Vec<usize>,
    pub token_index: Vec<usize>,
    pub amounts: Vec<f64>,
    pub action: TradeAction,
    pub reason_for_close: Option<ReasonForClose>,
    pub currect_price: Option<f64>,
    pub predicted_price: Option<f64>,
    pub atr: Option<f64>,
    pub momentum: Option<f64>,
}