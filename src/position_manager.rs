use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

use debot_utils::{DateTimeUtils, HasId, ToDateTimeString};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum TakeProfitStrategy {
    #[default]
    FixedThreshold,
    TrailingStop,
}

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
    take_profit_strategy: TakeProfitStrategy,
    state: State,
    token_name: String,
    fund_name: String,
    open_time: i64,
    open_time_str: String,
    close_time_str: String,
    average_open_price: f64,
    is_long_position: bool,
    take_profit_price: f64,
    #[serde(skip)]
    cut_loss_price: Arc<std::sync::Mutex<f64>>,
    initial_cut_loss_price: f64,
    trailing_distance: f64,
    close_price: Option<f64>,
    close_amount: Option<f64>,
    amount: f64,
    amount_in_anchor_token: f64,
    realized_pnl: Option<f64>,
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
        take_profit_strategy: TakeProfitStrategy,
        average_open_price: f64,
        is_long_position: bool,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        amount_in_anchor_token: f64,
        atr: Option<f64>,
        momentum: Option<f64>,
        predicted_price: Option<f64>,
    ) -> Self {
        let side = if is_long_position { "Buy" } else { "Sell" };
        let actual_amount = if is_long_position { amount } else { -amount };

        log::debug!(
            "Created a new {} position for token: {}, average_open_price: {:6.3}, take_profit_price: {:6.3}, cut_loss_price: {:6.3}, atr:{:?}",
            side, token_name, average_open_price, take_profit_price, cut_loss_price, atr
        );

        let open_time = chrono::Utc::now().timestamp();

        let modified_cut_loss_price = cut_loss_price;

        let trailing_distance = if is_long_position {
            take_profit_price - average_open_price
        } else {
            average_open_price - take_profit_price
        };

        Self {
            id: None,
            take_profit_strategy,
            state: State::Open,
            token_name: token_name.to_owned(),
            fund_name: fund_name.to_owned(),
            open_time,
            open_time_str: open_time.to_datetime_string(),
            close_time_str: String::new(),
            average_open_price,
            is_long_position,
            take_profit_price,
            cut_loss_price: Arc::new(std::sync::Mutex::new(modified_cut_loss_price)),
            initial_cut_loss_price: modified_cut_loss_price,
            trailing_distance,
            close_price: None,
            close_amount: None,
            amount: actual_amount,
            amount_in_anchor_token,
            realized_pnl: None,
            momentum,
            atr,
            predicted_price,
        }
    }

    fn should_take_profit_fixed_threshold(&self, close_price: f64) -> bool {
        if self.is_long_position {
            close_price >= self.take_profit_price
        } else {
            close_price <= self.take_profit_price
        }
    }
    // Adjusts cut loss price and returns false. The cut loss price is adjusted
    // based on the close price and the trailing distance.
    fn should_take_profit_trailing_stop(&self, close_price: f64) -> bool {
        if self.is_long_position {
            let current_distance = close_price - self.average_open_price;
            if current_distance > self.trailing_distance {
                let cut_loss_price = close_price - self.trailing_distance;
                let mut cut_loss_price_self = self.cut_loss_price.lock().unwrap();
                if cut_loss_price > *cut_loss_price_self {
                    *cut_loss_price_self = cut_loss_price;
                }
            }
        } else {
            let current_distance = self.average_open_price - close_price;
            if current_distance > self.trailing_distance {
                let cut_loss_price = close_price + self.trailing_distance;
                let mut cut_loss_price_self = self.cut_loss_price.lock().unwrap();
                if cut_loss_price < *cut_loss_price_self {
                    *cut_loss_price_self = cut_loss_price;
                }
            }
        }

        false
    }

    pub fn should_close(
        &self,
        close_price: f64,
        max_holding_interval: Option<i64>,
    ) -> Option<ReasonForClose> {
        if self.should_take_profit(close_price) {
            return Some(ReasonForClose::TakeProfit);
        }

        if let Some(interval) = max_holding_interval {
            if self.should_early_close(close_price, interval) {
                return Some(ReasonForClose::Other);
            }
        }

        self.should_cut_loss(close_price)
    }

    pub fn is_expired(&self, max_holding_interval: i64) -> Option<ReasonForClose> {
        let current_time = chrono::Utc::now().timestamp();
        let holding_interval = current_time - self.open_time;
        if holding_interval > max_holding_interval {
            return Some(ReasonForClose::Expired);
        }
        None
    }

    fn should_early_close(&self, close_price: f64, max_holding_interval: i64) -> bool {
        let current_time = chrono::Utc::now().timestamp();
        let holding_interval = current_time - self.open_time;
        if holding_interval > max_holding_interval * 3 / 4 {
            if self.is_long_position {
                return close_price > self.average_open_price;
            } else {
                return close_price < self.average_open_price;
            }
        }
        false
    }

    fn should_take_profit(&self, close_price: f64) -> bool {
        match self.take_profit_strategy {
            TakeProfitStrategy::FixedThreshold => {
                self.should_take_profit_fixed_threshold(close_price)
            }
            TakeProfitStrategy::TrailingStop => self.should_take_profit_trailing_stop(close_price),
        }
    }

    fn should_cut_loss(&self, close_price: f64) -> Option<ReasonForClose> {
        let cut_loss_price = *self.cut_loss_price.lock().unwrap();

        if self.is_long_position {
            if close_price < cut_loss_price {
                if close_price > self.average_open_price {
                    return Some(ReasonForClose::TakeProfit);
                } else {
                    return Some(ReasonForClose::CutLoss);
                }
            }
        } else {
            if close_price > cut_loss_price {
                if close_price < self.average_open_price {
                    return Some(ReasonForClose::TakeProfit);
                } else {
                    return Some(ReasonForClose::CutLoss);
                }
            }
        }

        None
    }

    fn update(&mut self, price: f64, amount: f64, reason: &str) {
        let prev_amount = self.amount;
        self.amount += amount;

        if self.amount == 0.0 {
            self.state = State::Closed(reason.to_owned());
        }

        if self.state == State::Open {
            self.average_open_price = (self.average_open_price * prev_amount.abs()
                + price * amount.abs())
                / self.amount.abs();
            log::info!("Updated open position :{:?}", self);
        } else {
            self.close_price = Some(price);
            self.close_amount = Some(prev_amount);
            let pnl = self.pnl(price, prev_amount);
            self.realized_pnl = Some(pnl);
            self.close_time_str = DateTimeUtils::get_current_datetime_string();

            log::info!("Cloes the position: {:?}", self);
        }
    }

    pub fn del(&mut self, close_price: f64, reason: &str) {
        self.update(close_price, -self.amount, reason)
    }

    pub fn add(
        &mut self,
        price: f64,
        is_long_position: bool,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        amount_in_anchor_token: f64,
    ) {
        let actual_amount = if is_long_position { amount } else { -amount };

        if actual_amount + self.amount == 0.0 {
            return self.del(price, "reverse trade");
        }

        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();

        self.amount_in_anchor_token += amount_in_anchor_token;

        self.take_profit_price = (self.take_profit_price * self.amount
            + take_profit_price * amount)
            / (self.amount.abs() + amount);

        let mut self_cut_loss_price = self.cut_loss_price.lock().unwrap();
        *self_cut_loss_price = (*self_cut_loss_price * self.amount.abs() + cut_loss_price * amount)
            / (self.amount + amount);
        drop(self_cut_loss_price);

        self.update(price, actual_amount, "add");
    }

    pub fn print_info(&self, current_price: f64) {
        let id = match self.id {
            Some(id) => id,
            None => 0,
        };

        log::info!(
            "ID: {:<3} Token: {:<6} PNL: {:>6.3}, current: {:>6.3}, open: {:>6.3}, take_profit: {:>6.3}, cut_loss: {:>6.3}, amount: {:>6.6}",
            id,
            self.token_name,
            self.pnl(current_price, self.amount),
            current_price,
            self.average_open_price,
            self.take_profit_price,
            *self.cut_loss_price.lock().unwrap(),
            self.amount
        );
    }

    fn pnl(&self, current_price: f64, amount: f64) -> f64 {
        (current_price - self.average_open_price) * amount
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

    pub fn initial_cut_loss_price(&self) -> f64 {
        self.initial_cut_loss_price
    }

    pub fn amount_in_anchor_token(&self) -> f64 {
        self.amount_in_anchor_token
    }

    pub fn reset_cut_loss_price(&mut self) {
        self.cut_loss_price = Arc::new(std::sync::Mutex::new(self.initial_cut_loss_price));
    }
}
