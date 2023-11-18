use debot_utils::{DateTimeUtils, HasId, ToDateTimeString};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ReasonForClose {
    Liquidated,
    Expired,
    TakeProfit,
    CutLoss,
    Other,
}

impl fmt::Display for ReasonForClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ReasonForClose::Liquidated => write!(f, "Liquidated"),
            ReasonForClose::Expired => write!(f, "Expired"),
            ReasonForClose::TakeProfit => write!(f, "TakeProfit"),
            ReasonForClose::CutLoss => write!(f, "CutLoss"),
            ReasonForClose::Other => write!(f, "Other"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradePosition {
    id: Option<u32>,
    state: State,
    token_name: String,
    fund_name: String,
    open_time: i64,
    open_time_str: String,
    close_time_str: String,
    average_open_price: f64,
    is_long_position: bool,
    take_profit_price: f64,
    cut_loss_price: f64,
    close_price: Option<f64>,
    close_amount: Option<f64>,
    amount: f64,
    amount_in_anchor_token: f64,
    pnl: Option<f64>,
    fee: f64,
    momentum: Option<f64>,
    atr: Option<f64>,
    predicted_price: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Open,
    Closed(String),
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Open => write!(f, "Open"),
            State::Closed(reason) => write!(f, "Closed({})", reason),
        }
    }
}

impl HasId for TradePosition {
    fn id(&self) -> Option<u32> {
        self.id
    }
}

impl TradePosition {
    pub fn new(
        token_name: &str,
        fund_name: &str,
        average_open_price: f64,
        is_long_position: bool,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        amount_in_anchor_token: f64,
        fee: f64,
        atr: Option<f64>,
        momentum: Option<f64>,
        predicted_price: Option<f64>,
    ) -> Self {
        let side = if is_long_position { "Buy" } else { "Sell" };
        let actual_amount = Self::actual_amount(is_long_position, amount);

        log::info!(
            "++ Open a new {} position for token: {}, amount = {}",
            side,
            token_name,
            actual_amount
        );

        let open_time = chrono::Utc::now().timestamp();

        Self {
            id: None,
            state: State::Open,
            token_name: token_name.to_owned(),
            fund_name: fund_name.to_owned(),
            open_time,
            open_time_str: open_time.to_datetime_string(),
            close_time_str: String::new(),
            average_open_price,
            is_long_position,
            take_profit_price,
            cut_loss_price,
            close_price: None,
            close_amount: None,
            amount: actual_amount,
            amount_in_anchor_token,
            pnl: None,
            fee,
            momentum,
            atr,
            predicted_price,
        }
    }

    fn should_take_profit(&self, close_price: f64) -> bool {
        if self.is_long_position {
            close_price >= self.take_profit_price
        } else {
            close_price <= self.take_profit_price
        }
    }

    fn should_cut_loss(&self, close_price: f64) -> bool {
        if self.is_long_position {
            close_price <= self.cut_loss_price
        } else {
            close_price >= self.cut_loss_price
        }
    }

    pub fn should_close(&self, close_price: f64) -> Option<ReasonForClose> {
        if self.should_take_profit(close_price) {
            return Some(ReasonForClose::TakeProfit);
        }

        if self.should_cut_loss(close_price) {
            Some(ReasonForClose::CutLoss)
        } else {
            None
        }
    }

    pub fn is_expired(&self, max_holding_interval: i64) -> Option<ReasonForClose> {
        let current_time = chrono::Utc::now().timestamp();
        let holding_interval = current_time - self.open_time;
        if holding_interval > max_holding_interval {
            return Some(ReasonForClose::Expired);
        }
        None
    }

    fn update(&mut self, price: Option<f64>, amount: f64, fee: f64, reason: &str) {
        let pnl = match price {
            Some(price) => Some(self.pnl(price) - fee),
            None => None,
        };

        let prev_amount = self.amount;
        self.amount += amount;

        if self.amount == 0.0 {
            self.state = State::Closed(reason.to_owned());
        }

        self.fee += fee;

        if self.state == State::Open {
            self.average_open_price = (self.average_open_price * prev_amount.abs()
                + price.unwrap() * amount.abs())
                / self.amount.abs();
            log::info!("Updated open position :{:?}", self);
        } else {
            self.close_price = price;
            self.close_amount = Some(prev_amount);
            self.pnl = pnl;
            self.close_time_str = DateTimeUtils::get_current_datetime_string();

            log::info!("-- Cloes the position: {:?}", self);
        }
    }

    pub fn del(&mut self, close_price: Option<f64>, fee: f64, reason: &str) {
        self.update(close_price, -self.amount, fee, reason)
    }

    pub fn add(
        &mut self,
        price: f64,
        is_long_position: bool,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        amount_in_anchor_token: f64,
        fee: f64,
    ) {
        let actual_amount = Self::actual_amount(is_long_position, amount);

        if actual_amount + self.amount == 0.0 {
            return self.del(Some(price), fee, "reverse trade");
        }

        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();

        self.amount_in_anchor_token += amount_in_anchor_token;

        self.take_profit_price = (self.take_profit_price * self.amount.abs()
            + take_profit_price * amount)
            / (self.amount.abs() + amount);

        self.cut_loss_price = (self.cut_loss_price * self.amount.abs() + cut_loss_price * amount)
            / (self.amount.abs() + amount);

        self.update(Some(price), actual_amount, fee, "add");
    }

    pub fn print_info(&self, current_price: f64) {
        let id = match self.id {
            Some(id) => id,
            None => 0,
        };

        log::debug!(
            "ID: {:<3} Token: {:<6} PNL: {:>6.3}, current: {:>6.3}, open: {:>6.3}, take_profit: {:>6.3}, cut_loss: {:>6.3}, amount: {:>6.6}",
            id,
            self.token_name,
            self.pnl(current_price,),
            current_price,
            self.average_open_price,
            self.take_profit_price,
            self.cut_loss_price,
            self.amount
        );
    }

    fn actual_amount(is_long_position: bool, amount: f64) -> f64 {
        if is_long_position {
            amount
        } else {
            -amount
        }
    }

    pub fn pnl(&self, current_price: f64) -> f64 {
        (current_price - self.average_open_price) * self.amount
    }

    pub fn set_id(&mut self, id: Option<u32>) {
        self.id = id;
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn token_name(&self) -> &str {
        &self.token_name
    }

    pub fn fund_name(&self) -> &str {
        &self.fund_name
    }

    pub fn amount(&self) -> f64 {
        if self.is_long_position {
            self.amount
        } else {
            self.amount * -1.0
        }
    }

    pub fn is_long_position(&self) -> bool {
        self.is_long_position
    }

    pub fn cut_loss_price(&self) -> f64 {
        self.cut_loss_price
    }

    pub fn amount_in_anchor_token(&self) -> f64 {
        self.amount_in_anchor_token
    }
}
