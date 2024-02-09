use debot_utils::{DateTimeUtils, HasId, ToDateTimeString};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::PositionType;

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
    Canceled(String),
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Opening => write!(f, "Opening"),
            State::Open => write!(f, "Open"),
            State::Closing(reason) => write!(f, "Closing({})", reason),
            State::Closed(reason) => write!(f, "Closed({})", reason),
            State::Canceled(reason) => write!(f, "Canceled({}", reason),
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
    position_type: PositionType,
    predicted_price: f64,
    take_profit_price: Option<f64>,
    cut_loss_price: Option<f64>,
    close_price: Option<f64>,
    close_amount: Option<f64>,
    amount: f64,
    asset_in_usd: f64,
    pnl: f64,
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
        position_type: PositionType,
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
            position_type,
            predicted_price,
            take_profit_price: None,
            cut_loss_price: None,
            close_price: None,
            close_amount: None,
            amount: 0.0,
            asset_in_usd: 0.0,
            pnl: 0.0,
            fee: 0.0,
            atr,
        }
    }

    pub fn on_opened(
        &mut self,
        average_open_price: f64,
        amount: f64,
        asset_in_usd: f64,
        fee: f64,
        take_profit_price: f64,
        cut_loss_price: f64,
    ) -> Result<(), ()> {
        if self.state != State::Opening {
            log::error!("open: Invalid state: {}", self.state);
            return Err(());
        }

        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();
        self.average_open_price = average_open_price;
        self.amount = amount;
        self.asset_in_usd = asset_in_usd;
        self.fee = fee;
        self.take_profit_price = Some(take_profit_price);
        self.cut_loss_price = Some(cut_loss_price);
        self.state = State::Open;

        log::info!(
            "++ Opened a new position: {}",
            self.format_position(average_open_price)
        );

        return Ok(());
    }

    pub fn request_close(&mut self, order_id: &str, reason: &str) -> Result<(), ()> {
        if self.state != State::Open {
            log::error!("close: Invalid state: {}", self.state);
            return Err(());
        }

        self.order_id = order_id.to_owned();
        self.ordered_time = chrono::Utc::now().timestamp();
        self.state = State::Closing(reason.to_owned());

        return Ok(());
    }

    pub fn on_closed(
        &mut self,
        close_price: Option<f64>,
        fee: f64,
        do_liquidate: bool,
        liquidated_reason: Option<String>,
    ) {
        self.fee += fee;

        let reason = if do_liquidate {
            match liquidated_reason {
                Some(r) => format!("Liquidated, {}", r),
                None => String::from("Liquidated"),
            }
        } else {
            match self.state.clone() {
                State::Closing(reason) => reason,
                _ => {
                    log::error!("delete: Invalid state: {}", self.state);
                    return;
                }
            }
        };

        self.delete(close_price, &reason);
    }

    pub fn cancel(&mut self) -> Result<bool, ()> {
        match self.state {
            State::Opening => {
                self.state = State::Canceled(String::from("Not filled"));
                log::debug!("-- Cancled the opening order: {}", self.order_id);
                Ok(true)
            }
            State::Closing(_) => {
                self.state = State::Open;
                log::info!("-- Cancled the closing order: {}", self.order_id);
                Ok(false)
            }
            _ => {
                log::error!("cancel: Invalid state: {}", self.state);
                Err(())
            }
        }
    }

    fn increase(
        &mut self,
        current_price: f64,
        average_open_price: f64,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        asset_in_usd: f64,
        fee: f64,
    ) {
        self.average_open_price = (self.average_open_price * self.amount
            + average_open_price * amount)
            / (self.amount + amount);

        self.take_profit_price = match self.take_profit_price {
            Some(price) => {
                Some((price * self.amount + take_profit_price * amount) / (self.amount + amount))
            }
            None => Some(take_profit_price),
        };

        self.cut_loss_price = match self.cut_loss_price {
            Some(price) => {
                Some((price * self.amount + cut_loss_price * amount) / (self.amount + amount))
            }
            None => Some(take_profit_price),
        };

        self.amount += amount;
        self.asset_in_usd += asset_in_usd;
        self.fee += fee;

        log::info!(
            "+ Increase the position: {}",
            self.format_position(current_price)
        );
    }

    fn delete(&mut self, current_price: Option<f64>, reason: &str) {
        if self.state == State::Opening && self.amount == 0.0 {
            self.state = State::Canceled(reason.to_owned());
            return;
        }

        self.state = State::Closed(reason.to_owned());
        self.close_price = current_price;
        self.pnl -= self.fee;
        self.close_amount = Some(self.amount);
        self.close_time_str = DateTimeUtils::get_current_datetime_string();

        log::info!("-- Cloes the position: {}, pnl: {:.3?}", reason, self.pnl);
    }

    pub fn on_updated(
        &mut self,
        current_price: f64,
        average_open_price: f64,
        position_type: PositionType,
        take_profit_price: f64,
        cut_loss_price: f64,
        amount: f64,
        asset_in_usd: f64,
        fee: f64,
    ) {
        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();

        if self.position_type == position_type {
            self.increase(
                current_price,
                average_open_price,
                take_profit_price,
                cut_loss_price,
                amount,
                asset_in_usd,
                fee,
            );
        } else {
            self.fee += fee;
            self.amount -= amount;
            self.asset_in_usd -= asset_in_usd;

            // Full close
            if self.amount == 0.0 {
                self.average_open_price = 0.0;
                self.delete(Some(current_price), "CounterTrade");
            }
            // Patial close
            else {
                if self.amount < 0.0 {
                    self.amount *= -1.0;
                    self.asset_in_usd *= -1.0;
                    self.average_open_price = self.asset_in_usd / self.amount;
                    self.position_type = self.position_type.opposite();
                    self.take_profit_price = Some(take_profit_price);
                    self.cut_loss_price = Some(cut_loss_price);
                }

                log::info!(
                    "** Reduce the position: {}",
                    self.format_position(current_price)
                );
            }
        }
        // just for debugging
        self.ordered_price = self.average_open_price;
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

    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        self.amount * current_price + self.asset_in_usd
    }

    pub fn pnl(&self) -> f64 {
        self.pnl
    }

    pub fn id(&self) -> Option<u32> {
        self.id
    }

    pub fn average_open_price(&self) -> f64 {
        self.average_open_price
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

    pub fn state(&self) -> State {
        self.state.clone()
    }

    pub fn token_name(&self) -> &str {
        &self.token_name
    }

    pub fn fund_name(&self) -> &str {
        &self.fund_name
    }

    pub fn amount(&self) -> f64 {
        self.amount
    }

    pub fn position_type(&self) -> PositionType {
        self.position_type.clone()
    }

    pub fn asset_in_usd(&self) -> f64 {
        self.asset_in_usd
    }

    fn should_take_profit(&self, close_price: f64) -> bool {
        if self.state != State::Open {
            return false;
        }

        match self.take_profit_price {
            Some(take_profit_price) => {
                if self.position_type == PositionType::Long {
                    close_price >= take_profit_price
                } else {
                    close_price <= take_profit_price
                }
            }
            None => false,
        }
    }

    fn should_cut_loss(&self, close_price: f64) -> bool {
        if self.state != State::Open {
            return false;
        }

        match self.cut_loss_price {
            Some(cut_loss_price) => {
                if self.position_type == PositionType::Long {
                    close_price <= cut_loss_price
                } else {
                    close_price >= cut_loss_price
                }
            }
            None => false,
        }
    }

    fn format_position(&self, price: f64) -> String {
        let id = match self.id {
            Some(id) => id,
            None => 0,
        };

        format!(
            "ID:{} {:<6} pnl: {:3.3}, [{}] current: {:>6.3}, open: {:>6.3}, take: {:>6.3}, cut: {:>6.3}, amount: {:6.6}/{:6.6}",
            id,
            self.token_name,
            self.unrealized_pnl(price),
            self.position_type,
            price,
            self.average_open_price,
            self.take_profit_price.unwrap_or_default(),
            self.cut_loss_price.unwrap_or_default(),
            self.amount,
            self.asset_in_usd
        )
    }

    pub fn print_info(&self, current_price: f64) {
        if self.amount != 0.0 {
            log::debug!("{}", self.format_position(current_price));
        }
    }
}
