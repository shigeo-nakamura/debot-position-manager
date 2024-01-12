use debot_utils::{DateTimeUtils, HasId, ToDateTimeString};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ReasonForClose {
    Liquidated,
    Expired,
    TakeProfit,
    CutLoss,
    Other(String),
}

impl fmt::Display for ReasonForClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasonForClose::Liquidated => write!(f, "Liquidated"),
            ReasonForClose::Expired => write!(f, "Expired"),
            ReasonForClose::TakeProfit => write!(f, "TakeProfit"),
            ReasonForClose::CutLoss => write!(f, "CutLoss"),
            ReasonForClose::Other(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum State {
    #[default]
    Opening,
    Open,
    Closing(String),
    Closed(String),
    Canceled,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Opening => write!(f, "Opening"),
            State::Open => write!(f, "Open"),
            State::Closing(reason) => write!(f, "Closing({})", reason),
            State::Closed(reason) => write!(f, "Closed({})", reason),
            State::Canceled => write!(f, "Canceled"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradePosition {
    id: Option<u32>,
    order_id: String,
    ordered_price: f64,
    state: State,
    token_name: String,
    fund_name: String,
    ordered_time: i64,
    order_effective_duration: i64,
    open_time: i64,
    open_time_str: String,
    close_time_str: String,
    average_open_price: f64,
    is_long_position: bool,
    predicted_price: f64,
    take_profit_price: f64,
    cut_loss_price: f64,
    close_price: Option<f64>,
    close_amount: Option<f64>,
    amount: f64,
    amount_in_anchor_token: f64,
    pnl: Option<f64>,
    fee: f64,
    atr: Option<f64>,
}

impl HasId for TradePosition {
    fn id(&self) -> Option<u32> {
        self.id
    }
}

impl TradePosition {
    pub fn new(
        id: u32,
        order_id: &str,
        ordered_price: f64,
        order_effective_duration: i64,
        token_name: &str,
        fund_name: &str,
        is_long_position: bool,
        predicted_price: f64,
        atr: Option<f64>,
    ) -> Self {
        Self {
            id: Some(id),
            order_id: order_id.to_owned(),
            ordered_price,
            order_effective_duration,
            state: State::Opening,
            token_name: token_name.to_owned(),
            fund_name: fund_name.to_owned(),
            ordered_time: chrono::Utc::now().timestamp(),
            open_time: 0,
            open_time_str: String::new(),
            close_time_str: String::new(),
            average_open_price: 0.0,
            is_long_position,
            predicted_price,
            take_profit_price: 0.0,
            cut_loss_price: 0.0,
            close_price: None,
            close_amount: None,
            amount: 0.0,
            amount_in_anchor_token: 0.0,
            pnl: None,
            fee: 0.0,
            atr,
        }
    }

    pub fn open(
        &mut self,
        average_open_price: f64,
        amount: f64,
        amount_in_anchor_token: f64,
        fee: f64,
        take_profit_price: f64,
        cut_loss_price: f64,
    ) -> Result<(), ()> {
        if self.state != State::Opening {
            log::error!("open: Invalid state: {}", self.state);
            return Err(());
        }

        let actual_amount = Self::actual_amount(self.is_long_position, amount);
        let side = if self.is_long_position { "Buy" } else { "Sell" };

        log::info!(
            "++ Opened a new {} position for token: {}, amount = {}",
            side,
            self.token_name,
            actual_amount
        );

        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();
        self.average_open_price = average_open_price;
        self.amount = actual_amount;
        self.amount_in_anchor_token = amount_in_anchor_token;
        self.fee = fee;
        self.take_profit_price = take_profit_price;
        self.cut_loss_price = cut_loss_price;
        self.state = State::Open;

        return Ok(());
    }

    pub fn close(&mut self, order_id: &str, reason: &str) -> Result<(), ()> {
        if self.state != State::Open {
            log::error!("close: Invalid state: {}", self.state);
            return Err(());
        }

        self.order_id = order_id.to_owned();
        self.ordered_time = chrono::Utc::now().timestamp();
        self.state = State::Closing(reason.to_owned());

        return Ok(());
    }

    pub fn delete(
        &mut self,
        close_price: Option<f64>,
        fee: f64,
        do_liquidate: bool,
        liquidated_reason: Option<String>,
    ) {
        if do_liquidate {
            let reason = match liquidated_reason {
                Some(r) => format!("Liquidated, {}", r),
                None => String::from("Liquidated"),
            };
            self.update(close_price, -self.amount, fee, &reason);
        } else {
            let reason = match self.state.clone() {
                State::Closing(reason) => reason,
                _ => {
                    log::error!("delete: Invalid state: {}", self.state);
                    return;
                }
            };
            self.update(close_price, -self.amount, fee, &reason);
        }
    }

    pub fn cancel(&mut self) {
        match self.state {
            State::Opening => {
                self.state = State::Canceled;
                log::info!("-- Cancled the position: {:?}", self);
            }
            State::Closing(_) => {
                self.state = State::Open;
                log::info!("-- Cancled the position: {:?}", self);
            }
            _ => {
                log::error!("cancel: Invalid state: {}", self.state);
            }
        }
    }

    pub fn should_cancel_order(&self) -> bool {
        match self.state {
            State::Opening | State::Closing(_) => {
                let current_time = chrono::Utc::now().timestamp();
                let ordering_duration = current_time - self.ordered_time;
                ordering_duration > self.order_effective_duration
            }
            _ => false,
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

    pub fn is_expired(&self, max_holding_duration: i64) -> Option<ReasonForClose> {
        let current_time = chrono::Utc::now().timestamp();
        let holding_duration = current_time - self.open_time;
        if holding_duration > max_holding_duration {
            return Some(ReasonForClose::Expired);
        }
        None
    }

    pub fn pnl(&self, current_price: f64) -> f64 {
        (current_price - self.average_open_price) * self.amount
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    pub fn ordered_price(&self) -> f64 {
        self.ordered_price
    }

    pub fn predicted_price(&self) -> f64 {
        self.predicted_price
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
            -self.amount
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

    fn actual_amount(is_long_position: bool, amount: f64) -> f64 {
        if is_long_position {
            amount
        } else {
            -amount
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
}
