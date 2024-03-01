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
    unfilled_amount: f64,
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
    close_price: f64,
    close_asset_in_usd: f64,
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

enum UpdateResult {
    Closed,
    Decreaed,
    Inverted,
}

impl TradePosition {
    pub fn new(
        id: u32,
        order_id: &str,
        ordered_price: f64,
        ordered_amount: f64,
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
            unfilled_amount: ordered_amount,
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
            close_price: 0.0,
            close_asset_in_usd: 0.0,
            amount: 0.0,
            asset_in_usd: 0.0,
            pnl: 0.0,
            fee: 0.0,
            atr,
        }
    }

    pub fn on_filled(
        &mut self,
        position_type: PositionType,
        filled_price: f64,
        amount: f64,
        asset_in_usd: f64,
        fee: f64,
        take_profit_price: Option<f64>,
        cut_loss_price: Option<f64>,
        current_price: f64,
    ) -> Result<(), ()> {
        match self.state {
            State::Opening => {
                self.unfilled_amount -= amount;
                if self.unfilled_amount == 0.0 {
                    self.state = State::Open;
                }
            }
            State::Open | State::Closing(_) => {}
            _ => {
                log::error!("on_filled: Invalid state: {}", self.state);
                return Err(());
            }
        }

        log::trace!(
            "state = {}, unfilled_amount = {}, amount = {}",
            self.state,
            self.unfilled_amount,
            amount
        );

        self.open_time = chrono::Utc::now().timestamp();
        self.open_time_str = self.open_time.to_datetime_string();

        self.fee += fee;

        if self.position_type == position_type {
            self.increase(
                position_type,
                filled_price,
                take_profit_price,
                cut_loss_price,
                amount,
                asset_in_usd,
                current_price,
            );
        } else {
            self.decrease(
                position_type,
                filled_price,
                take_profit_price,
                cut_loss_price,
                amount,
                asset_in_usd,
                current_price,
            );
        }

        // To sort orders and the open position by order price(for debugging)
        self.ordered_price = self.average_open_price;

        return Ok(());
    }

    pub fn on_liquidated(
        &mut self,
        close_price: f64,
        fee: f64,
        do_liquidate: bool,
        liquidated_reason: Option<String>,
    ) -> Result<(), ()> {
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
                    return Err(());
                }
            }
        };

        self.delete(close_price, &reason);

        return Ok(());
    }

    pub fn request_close(&mut self, order_id: &str, reason: &str) -> Result<(), ()> {
        match self.state() {
            State::Opening => {
                if self.unfilled_amount == 0.0 {
                    log::error!("request_close: Invalid state(1): {:?}", self);
                    return Err(());
                }
            }
            State::Open => {
                log::debug!("requeset_close: reason = {}", reason.to_owned());
            }
            _ => {
                log::error!("request_close: Invalid state(2): {:?}", self);
                return Err(());
            }
        }

        self.order_id = order_id.to_owned();
        self.ordered_time = chrono::Utc::now().timestamp();
        self.state = State::Closing(reason.to_owned());

        return Ok(());
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
                self.ordered_time = chrono::Utc::now().timestamp();
                log::info!("-- Cancled the closing order: {}", self.order_id);
                Ok(false)
            }
            _ => {
                log::error!("cancel: Invalid state: {:?}", self);
                Err(())
            }
        }
    }

    fn increase(
        &mut self,
        position_type: PositionType,
        filled_price: f64,
        take_profit_price: Option<f64>,
        cut_loss_price: Option<f64>,
        amount: f64,
        asset_in_usd: f64,
        current_price: f64,
    ) {
        let current_amount = self.amount.abs();

        self.average_open_price = (self.average_open_price * current_amount
            + filled_price * amount)
            / (current_amount + amount);

        self.take_profit_price = match take_profit_price {
            Some(new_price) => match self.take_profit_price {
                Some(current_price) => Some(
                    (current_price * current_amount + new_price * amount)
                        / (current_amount + amount),
                ),
                None => Some(new_price),
            },
            None => None,
        };

        self.cut_loss_price = match cut_loss_price {
            Some(new_price) => match self.cut_loss_price {
                Some(current_price) => Some(
                    (current_price * current_amount + new_price * amount)
                        / (current_amount + amount),
                ),
                None => Some(new_price),
            },
            None => None,
        };

        self.update_amount(position_type, amount, asset_in_usd);

        log::info!(
            "+ Increase the position: {}",
            self.format_position(current_price)
        );
    }

    fn decrease(
        &mut self,
        position_type: PositionType,
        filled_price: f64,
        take_profit_price: Option<f64>,
        cut_loss_price: Option<f64>,
        amount: f64,
        asset_in_usd: f64,
        current_price: f64,
    ) {
        match self.update_amount_and_pnl(position_type, amount, asset_in_usd, filled_price) {
            UpdateResult::Closed => {
                self.delete(filled_price, "CounterTrade");
            }
            UpdateResult::Inverted => {
                self.average_open_price = filled_price;
                self.take_profit_price = take_profit_price;
                self.cut_loss_price = cut_loss_price;
                self.position_type = self.position_type.opposite();
                log::info!(
                    "- The position is inverted: {}",
                    self.format_position(filled_price)
                );
            }
            UpdateResult::Decreaed => {
                log::info!(
                    "** The position is decreased: {}",
                    self.format_position(current_price)
                );
            }
        }
    }

    fn delete(&mut self, close_price: f64, reason: &str) {
        if self.state == State::Opening && self.amount == 0.0 {
            self.state = State::Canceled(reason.to_owned());
            return;
        }

        if let State::Closing(closing_reason) = self.state.clone() {
            self.state = State::Closed(closing_reason);
        } else {
            self.state = State::Closed(reason.to_owned());
        }

        self.close_asset_in_usd = self.asset_in_usd;
        self.close_price = close_price;
        self.pnl += Self::unrealized_pnl(close_price, self.amount, self.asset_in_usd);
        self.pnl -= self.fee;
        self.amount = 0.0;
        self.asset_in_usd = 0.0;
        self.close_time_str = DateTimeUtils::get_current_datetime_string();

        log::info!("-- Cloes the position: {}, pnl: {:.3?}", reason, self.pnl);
    }

    fn update_amount_and_pnl(
        &mut self,
        position_type: PositionType,
        amount: f64,
        asset_in_usd: f64,
        close_price: f64,
    ) -> UpdateResult {
        let prev_asset_in_usd = self.asset_in_usd;
        let prev_amount = self.amount;

        self.update_amount(position_type, amount, asset_in_usd);

        let update_result = if self.amount == 0.0 {
            UpdateResult::Closed
        } else if prev_amount.signum() != self.amount.signum() {
            UpdateResult::Inverted
        } else {
            UpdateResult::Decreaed
        };

        let pnl = self.calculate_pnl_for_update(
            &update_result,
            prev_amount,
            close_price,
            prev_asset_in_usd,
        );
        self.realize_pnl(pnl);

        update_result
    }

    fn calculate_pnl_for_update(
        &self,
        update_result: &UpdateResult,
        prev_amount: f64,
        close_price: f64,
        prev_asset_in_usd: f64,
    ) -> f64 {
        match update_result {
            UpdateResult::Decreaed => {
                (close_price - self.average_open_price) * (prev_amount - self.amount)
            }
            _ => Self::unrealized_pnl(close_price, prev_amount, prev_asset_in_usd),
        }
    }

    fn update_amount(&mut self, position_type: PositionType, amount: f64, asset_in_usd: f64) {
        if position_type == PositionType::Long {
            self.amount += amount;
            self.asset_in_usd -= asset_in_usd;
        } else {
            self.amount -= amount;
            self.asset_in_usd += asset_in_usd;
        }
    }

    fn realize_pnl(&mut self, pnl: f64) {
        self.pnl += pnl;
        self.asset_in_usd -= pnl;
    }

    fn unrealized_pnl(price: f64, amount: f64, asset_in_usd: f64) -> f64 {
        amount * price + asset_in_usd
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

    pub fn unfilled_amount(&self) -> f64 {
        self.unfilled_amount
    }

    pub fn position_type(&self) -> PositionType {
        self.position_type.clone()
    }

    pub fn asset_in_usd(&self) -> f64 {
        self.asset_in_usd
    }

    pub fn close_asset_in_usd(&self) -> f64 {
        self.close_asset_in_usd
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

    fn format_position(&self, current_price: f64) -> String {
        let id = match self.id {
            Some(id) => id,
            None => 0,
        };

        let open_price = self.average_open_price;
        let take_profit_price = self.take_profit_price.unwrap_or_default();
        let cut_loss_price = self.cut_loss_price.unwrap_or_default();

        let unrealized_pnl = Self::unrealized_pnl(current_price, self.amount, self.asset_in_usd);

        format!(
            "ID:{} {:<6}({}) un-pnl: {:3.3}({:.2}%), re-pnl: {:3.3}, [{}] price: {:>6.3}/{:>6.3}({:.3}%), cut: {:>6.3}({:.3}%), take: {:>6.3}({:.3}%), amount: {:6.6}({:6.6})/{:6.6}",
            id,
            self.token_name,
            self.state,
            unrealized_pnl,
            unrealized_pnl / self.asset_in_usd.abs() * 100.0,
            self.pnl,
            self.position_type,
            current_price,
            open_price,
            (open_price - current_price) / current_price * 100.0,
            cut_loss_price,
            (cut_loss_price - current_price) / current_price * 100.0,
            take_profit_price,
            (take_profit_price - current_price) / current_price * 100.0,
            self.amount,
            self.unfilled_amount,
            self.asset_in_usd
        )
    }

    pub fn print_info(&self, current_price: f64) {
        if self.amount != 0.0 {
            log::debug!("{}", self.format_position(current_price));
        }
    }
}
