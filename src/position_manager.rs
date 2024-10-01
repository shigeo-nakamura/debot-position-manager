use crate::PositionType;
use debot_db::CandlePattern;
use debot_utils::get_local_time;
use rust_decimal::{prelude::Signed, Decimal};
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
    id: u32,
    fund_name: String,
    order_id: String,
    ordered_price: Decimal,
    unfilled_amount: Decimal,
    invested_amount: Decimal,
    state: State,
    token_name: String,
    tick_count: u32,
    open_order_tick_count_max: u32,
    close_order_tick_count_max: u32,
    open_tick_count_max: u32,
    open_time_str: String,
    open_timestamp: i64,
    close_time_str: String,
    average_open_price: Decimal,
    position_type: PositionType,
    predicted_price: Decimal,
    take_profit_price: Option<Decimal>,
    cut_loss_price: Option<Decimal>,
    close_price: Decimal,
    close_asset_in_usd: Decimal,
    amount: Decimal,
    asset_in_usd: Decimal,
    pnl: Decimal,
    fee: Decimal,
    // for debug
    atr: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    adx: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    rsi: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    stochastic: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    price: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    candle_pattern: (CandlePattern, CandlePattern, CandlePattern, CandlePattern),
    take_profit_ratio: Decimal,
    atr_spread: Decimal,
    risk_reward: Decimal,
    atr_term: Decimal,
    last_volume: Option<Decimal>,
    last_num_trades: Option<u64>,
    last_funding_rate: Option<Decimal>,
    last_open_interest: Option<Decimal>,
    last_oracle_price: Option<Decimal>,
}

enum UpdateResult {
    Closed,
    Decreaed,
    Inverted,
}

pub enum CancelResult {
    OpeningCanceled,
    ClosingCanceled,
    PartiallyFilled,
}

pub enum OrderType {
    OpenOrder,
    CloseOrder,
}

impl TradePosition {
    pub fn new(
        id: u32,
        fund_name: &str,
        order_id: &str,
        ordered_price: Decimal,
        ordered_amount: Decimal,
        open_order_tick_count_max: u32,
        close_order_tick_count_max: u32,
        open_tick_count_max: u32,
        token_name: &str,
        position_type: PositionType,
        predicted_price: Decimal,
        atr: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        adx: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        rsi: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        stochastic: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        price: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        candle_pattern: (CandlePattern, CandlePattern, CandlePattern, CandlePattern),
        take_profit_ratio: Decimal,
        atr_spread: Decimal,
        risk_reward: Decimal,
        atr_term: Decimal,
        last_volume: Option<Decimal>,
        last_num_trades: Option<u64>,
        last_funding_rate: Option<Decimal>,
        last_open_interest: Option<Decimal>,
        last_oracle_price: Option<Decimal>,
    ) -> Self {
        let decimal_0 = Decimal::new(0, 0);
        Self {
            id,
            fund_name: fund_name.to_owned(),
            order_id: order_id.to_owned(),
            ordered_price,
            unfilled_amount: ordered_amount,
            invested_amount: ordered_price * ordered_amount,
            tick_count: 0,
            open_order_tick_count_max,
            close_order_tick_count_max,
            open_tick_count_max,
            state: State::Opening,
            token_name: token_name.to_owned(),
            open_time_str: String::new(),
            open_timestamp: 0,
            close_time_str: String::new(),
            average_open_price: decimal_0,
            position_type,
            predicted_price,
            take_profit_price: None,
            cut_loss_price: None,
            close_price: decimal_0,
            close_asset_in_usd: decimal_0,
            amount: decimal_0,
            asset_in_usd: decimal_0,
            pnl: decimal_0,
            fee: decimal_0,
            atr,
            adx,
            rsi,
            price,
            candle_pattern,
            take_profit_ratio,
            stochastic,
            atr_spread,
            risk_reward,
            atr_term,
            last_volume,
            last_num_trades,
            last_funding_rate,
            last_open_interest,
            last_oracle_price,
        }
    }

    pub fn on_filled(
        &mut self,
        position_type: PositionType,
        filled_price: Decimal,
        amount: Decimal,
        asset_in_usd: Decimal,
        fee: Decimal,
        take_profit_price: Option<Decimal>,
        cut_loss_price: Option<Decimal>,
        current_price: Decimal,
    ) -> Result<(), ()> {
        match self.state {
            State::Opening | State::Canceled(_) => {
                self.unfilled_amount -= amount;
                if self.unfilled_amount.is_zero() {
                    self.update_state(State::Open)
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
        close_price: Decimal,
        fee: Decimal,
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
                if self.unfilled_amount.is_zero() {
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
        self.update_state(State::Closing(reason.to_owned()));

        return Ok(());
    }

    pub fn cancel(&mut self) -> Result<CancelResult, ()> {
        match self.state {
            State::Opening => {
                if self.amount.is_zero() {
                    self.update_state(State::Canceled(String::from("Not filled at all")));
                    log::debug!("-- Canceled the opening order: {}", self.order_id);
                    Ok(CancelResult::OpeningCanceled)
                } else {
                    self.update_state(State::Open);
                    log::debug!(
                        "-- This opening order is partially filled: {}",
                        self.order_id
                    );
                    Ok(CancelResult::PartiallyFilled)
                }
            }
            State::Closing(_) => {
                self.update_state(State::Open);
                log::info!("-- Canceling the closing order: {}", self.order_id);
                Ok(CancelResult::ClosingCanceled)
            }
            _ => {
                log::error!("cancel: Invalid state: {:?}", self);
                Err(())
            }
        }
    }

    pub fn ignore(&mut self) {
        self.update_state(State::Canceled("Partially filled".to_owned()));
    }

    fn increase(
        &mut self,
        position_type: PositionType,
        filled_price: Decimal,
        take_profit_price: Option<Decimal>,
        cut_loss_price: Option<Decimal>,
        amount: Decimal,
        asset_in_usd: Decimal,
        current_price: Decimal,
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
        filled_price: Decimal,
        take_profit_price: Option<Decimal>,
        cut_loss_price: Option<Decimal>,
        amount: Decimal,
        asset_in_usd: Decimal,
        current_price: Decimal,
    ) {
        match self.update_amount_and_pnl(position_type, amount, asset_in_usd, filled_price) {
            UpdateResult::Closed => {
                self.delete(filled_price, "CounterTrade");
                return;
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

    fn delete(&mut self, close_price: Decimal, reason: &str) {
        if self.state == State::Opening && self.amount.is_zero() {
            self.update_state(State::Canceled(reason.to_owned()));
            return;
        }

        if let State::Closing(closing_reason) = self.state.clone() {
            self.update_state(State::Closed(closing_reason));
        } else {
            self.update_state(State::Closed(reason.to_owned()));
        }

        self.close_asset_in_usd = self.asset_in_usd;
        self.close_price = close_price;
        self.pnl += Self::unrealized_pnl(close_price, self.amount, self.asset_in_usd);
        self.pnl -= self.fee;
        self.amount = Decimal::new(0, 0);
        self.asset_in_usd = Decimal::new(0, 0);

        log::info!(
            "-- Close the position[{}]: {}, pnl: {:.3?}",
            self.position_type,
            reason,
            self.pnl
        );
    }

    fn update_state(&mut self, new_state: State) {
        assert!(new_state != self.state, "The same state: {:?}", new_state);

        match new_state {
            State::Closing(_) | State::Canceled(_) => self.tick_count = 0,
            State::Open => match self.state {
                State::Opening | State::Canceled(_) => {
                    self.tick_count = 0;
                    self.set_open_time();
                }
                State::Closing(_) => self.tick_count = self.open_tick_count_max,
                _ => {}
            },
            State::Closed(_) => {
                self.set_close_time();
            }
            _ => {}
        }

        self.state = new_state
    }

    fn update_amount_and_pnl(
        &mut self,
        position_type: PositionType,
        amount: Decimal,
        asset_in_usd: Decimal,
        close_price: Decimal,
    ) -> UpdateResult {
        let prev_asset_in_usd = self.asset_in_usd;
        let prev_amount = self.amount;

        self.update_amount(position_type, amount, asset_in_usd);

        let update_result = if self.amount.is_zero() {
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
        prev_amount: Decimal,
        close_price: Decimal,
        prev_asset_in_usd: Decimal,
    ) -> Decimal {
        match update_result {
            UpdateResult::Decreaed => {
                (close_price - self.average_open_price) * (prev_amount - self.amount)
            }
            _ => Self::unrealized_pnl(close_price, prev_amount, prev_asset_in_usd),
        }
    }

    fn update_amount(
        &mut self,
        position_type: PositionType,
        amount: Decimal,
        asset_in_usd: Decimal,
    ) {
        if position_type == PositionType::Long {
            self.amount += amount;
            self.asset_in_usd -= asset_in_usd;
        } else {
            self.amount -= amount;
            self.asset_in_usd += asset_in_usd;
        }
    }

    fn realize_pnl(&mut self, pnl: Decimal) {
        self.pnl += pnl;
        self.asset_in_usd -= pnl;
    }

    fn unrealized_pnl(price: Decimal, amount: Decimal, asset_in_usd: Decimal) -> Decimal {
        amount * price + asset_in_usd
    }

    pub fn should_cancel_order(&self) -> bool {
        match self.state {
            State::Opening => self.tick_count > self.open_order_tick_count_max,
            State::Closing(_) => self.tick_count > self.close_order_tick_count_max,
            _ => false,
        }
    }

    pub fn update_counter(&mut self) {
        self.tick_count += 1;
    }

    pub fn should_close(&self, close_price: Decimal) -> Option<ReasonForClose> {
        if self.should_take_profit(close_price) {
            return Some(ReasonForClose::TakeProfit);
        }

        if self.should_cut_loss(close_price) {
            Some(ReasonForClose::CutLoss)
        } else {
            None
        }
    }

    pub fn is_cancel_expired(&self) -> bool {
        if matches!(self.state, State::Canceled(_)) {
            self.tick_count > self.close_order_tick_count_max
        } else {
            false
        }
    }

    pub fn should_open_expired(&self) -> bool {
        if matches!(self.state, State::Open) {
            self.tick_count > self.open_tick_count_max
        } else {
            false
        }
    }

    pub fn pnl(&self) -> (Decimal, Decimal) {
        if self.invested_amount.is_zero() {
            (self.pnl, Decimal::ZERO)
        } else {
            (self.pnl, self.pnl / self.invested_amount)
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn fund_name(&self) -> &str {
        &self.fund_name
    }

    pub fn average_open_price(&self) -> Decimal {
        self.average_open_price
    }

    pub fn order_id(&self) -> &str {
        &self.order_id
    }

    pub fn ordered_price(&self) -> Decimal {
        self.ordered_price
    }

    pub fn predicted_price(&self) -> Decimal {
        self.predicted_price
    }

    pub fn state(&self) -> State {
        self.state.clone()
    }

    pub fn token_name(&self) -> &str {
        &self.token_name
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn unfilled_amount(&self) -> Decimal {
        self.unfilled_amount
    }

    pub fn position_type(&self) -> PositionType {
        self.position_type.clone()
    }

    pub fn asset_in_usd(&self) -> Decimal {
        self.asset_in_usd
    }

    pub fn close_asset_in_usd(&self) -> Decimal {
        self.close_asset_in_usd
    }

    pub fn open_timestamp(&self) -> i64 {
        self.open_timestamp
    }

    pub fn open_time_str(&self) -> &str {
        &self.open_time_str
    }

    pub fn close_time_str(&self) -> &str {
        &self.close_time_str
    }

    pub fn close_price(&self) -> Decimal {
        self.close_price
    }

    pub fn last_volume(&self) -> Option<Decimal> {
        self.last_volume
    }

    pub fn last_num_trades(&self) -> Option<u64> {
        self.last_num_trades
    }

    pub fn last_funding_rate(&self) -> Option<Decimal> {
        self.last_funding_rate
    }

    pub fn last_open_interest(&self) -> Option<Decimal> {
        self.last_open_interest
    }

    pub fn last_oracle_price(&self) -> Option<Decimal> {
        self.last_oracle_price
    }

    pub fn rsi(&self) -> (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
        self.rsi
    }

    pub fn atr(&self) -> (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
        self.atr
    }

    pub fn adx(&self) -> (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
        self.adx
    }

    pub fn stochastic(&self) -> (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
        self.stochastic
    }

    pub fn price(&self) -> (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
        self.price
    }

    pub fn candle_pattern(&self) -> (CandlePattern, CandlePattern, CandlePattern, CandlePattern) {
        self.candle_pattern
    }

    pub fn take_profit_ratio(&self) -> Decimal {
        self.take_profit_ratio
    }

    pub fn atr_spread(&self) -> Decimal {
        self.atr_spread
    }

    pub fn risk_reward(&self) -> Decimal {
        self.risk_reward
    }

    pub fn atr_term(&self) -> Decimal {
        self.atr_term
    }

    pub fn fee(&self) -> Decimal {
        self.fee
    }

    pub fn open_tick_count_max(&self) -> u32 {
        self.open_tick_count_max
    }

    fn should_take_profit(&self, close_price: Decimal) -> bool {
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

    fn should_cut_loss(&self, close_price: Decimal) -> bool {
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

    fn set_open_time(&mut self) {
        let (timestamp, time_str) = get_local_time();
        self.open_timestamp = timestamp;
        self.open_time_str = time_str;
    }

    fn set_close_time(&mut self) {
        let (_, time_str) = get_local_time();
        self.close_time_str = time_str;
    }

    fn format_position(&self, current_price: Decimal) -> String {
        let open_price = self.average_open_price;
        let take_profit_price = self.take_profit_price.unwrap_or_default();
        let cut_loss_price = self.cut_loss_price.unwrap_or_default();

        let unrealized_pnl = Self::unrealized_pnl(current_price, self.amount, self.asset_in_usd);
        let decimal_100 = Decimal::new(100, 0);

        format!(
            "ID:{} {:<6}({}) un-pnl: {:3.3}({:.2}%), re-pnl: {:3.3}, [{}] price: {:>6.3}/{:>6.3}({:.3}%), cut: {:>6.3}({:.3}%), take: {:>6.3}({:.3}%), amount: {:6.6}({:6.6})/{:6.6}",
            self.id,
            self.token_name,
            self.state,
            unrealized_pnl,
            unrealized_pnl / self.asset_in_usd.abs() * decimal_100,
            self.pnl,
            self.position_type,
            current_price,
            open_price,
            (open_price - current_price) / current_price * decimal_100,
            cut_loss_price,
            (cut_loss_price - current_price) / current_price * decimal_100,
            take_profit_price,
            (take_profit_price - current_price) / current_price * decimal_100,
            self.amount,
            self.unfilled_amount,
            self.asset_in_usd
        )
    }

    pub fn print_info(&self, current_price: Decimal) {
        if !self.amount.is_zero() {
            log::debug!("{}", self.format_position(current_price));
        }
    }
}
