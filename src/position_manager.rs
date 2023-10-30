use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TradePosition {
    pub id: Option<u32>,
    pub take_profit_strategy: TakeProfitStrategy,
    pub state: State,
    pub token_name: String,
    pub fund_name: String,
    pub open_time: i64,
    pub open_time_str: String,
    pub close_time_str: String,
    pub average_open_price: f64,
    pub is_long_position: bool,
    pub take_profit_price: f64,
    #[serde(skip)]
    pub cut_loss_price: Arc<std::sync::Mutex<f64>>,
    pub initial_cut_loss_price: f64,
    pub trailing_distance: f64,
    pub sold_price: Option<f64>,
    pub sold_amount: Option<f64>,
    pub amount: f64,
    pub amount_in_anchor_token: f64,
    pub realized_pnl: Option<f64>,
    pub momentum: Option<f64>,
    pub atr: Option<f64>,
    pub predicted_price: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub enum State {
    #[default]
    Open,
    CutLoss,
    TookProfit,
    Closed,
    Liquidated,
    Expired,
    Bullish,
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
        log::debug!(
            "Created new open position for token: {}, average_open_price: {:6.3}, take_profit_price: {:6.3}, cut_loss_price: {:6.3}, atr:{:?}",
            token_name, average_open_price, take_profit_price, cut_loss_price, atr
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
            sold_price: None,
            sold_amount: None,
            amount,
            amount_in_anchor_token,
            realized_pnl: None,
            momentum,
            atr,
            predicted_price,
        }
    }

    fn should_take_profit_fixed_threshold(&self, sell_price: f64) -> bool {
        if self.is_long_position {
            sell_price >= self.take_profit_price
        } else {
            sell_price <= self.take_profit_price
        }
    }
    // Adjusts cut loss price and returns false. The cut loss price is adjusted
    // based on the sell price and the trailing distance.
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
        sell_price: f64,
        max_holding_interval: i64,
    ) -> Option<ReasonForClose> {
        if self.should_take_profit(sell_price) {
            return Some(ReasonForClose::TakeProfit);
        }

        if self.should_early_close(sell_price, max_holding_interval) {
            return Some(ReasonForClose::Other);
        }

        self.should_cut_loss(sell_price)
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
                close_price > self.average_open_price
            } else {
                close_price < self.average_open_price
            }
        } else {
            return false;
        }
    }

    fn should_take_profit(&self, sell_price: f64) -> bool {
        match self.take_profit_strategy {
            TakeProfitStrategy::FixedThreshold => {
                self.should_take_profit_fixed_threshold(sell_price)
            }
            TakeProfitStrategy::TrailingStop => self.should_take_profit_trailing_stop(sell_price),
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

    fn update(&mut self, average_price: f64, amount: f64) {
        if self.state == State::Open {
            self.amount += amount;
            self.average_open_price = (self.average_open_price * self.amount
                + average_price * amount)
                / (self.amount + amount);
            log::info!("Updated open position :{:?}", self);
        } else {
            self.amount -= amount;
            self.sold_price = Some(average_price);
            self.sold_amount = Some(amount);
            let pnl = self.pnl(average_price, self.average_open_price, self.amount);
            self.realized_pnl = Some(pnl);
            self.close_time_str = DateTimeUtils::get_current_datetime_string();

            log::info!("Cloes the position: {:?}", self);
        }
    }

    pub fn del(&mut self, sold_price: f64, amount: f64, state: State) {
        self.state = state;
        self.update(sold_price, amount);
    }

    pub fn add(
        &mut self,
        average_price: f64,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        amount_in_anchor_token: f64,
    ) {
        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();

        self.amount_in_anchor_token += amount_in_anchor_token;

        self.take_profit_price = (self.take_profit_price * self.amount
            + take_profit_price * amount)
            / (self.amount + amount);

        let mut self_cut_loss_price = self.cut_loss_price.lock().unwrap();
        *self_cut_loss_price =
            (*self_cut_loss_price * self.amount + cut_loss_price * amount) / (self.amount + amount);
        drop(self_cut_loss_price);

        self.update(average_price, amount);
    }

    pub fn print_info(&self, current_price: f64) {
        let id = match self.id {
            Some(id) => id,
            None => 0,
        };

        log::info!(
            "ID: {:<3} Token: {:<6} PNL: {:>6.3}, current: {:>6.3}, buy: {:>6.3}, take_profit: {:>6.3}, cut_loss: {:>6.3}, amount: {:>6.6}",
            id,
            self.token_name,
            self.pnl(current_price, self.average_open_price, self.amount),
            current_price,
            self.average_open_price,
            self.take_profit_price,
            *self.cut_loss_price.lock().unwrap(),
            self.amount
        );
    }

    fn pnl(&self, current_price: f64, open_price: f64, amount: f64) -> f64 {
        (if self.is_long_position {
            current_price - open_price
        } else {
            open_price - current_price
        }) * amount
    }
}
