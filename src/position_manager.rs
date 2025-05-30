use crate::PositionType;
use debot_db::CandlePattern;
use debot_utils::get_local_time;
use rust_decimal::{prelude::Signed, Decimal};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, fmt};

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
pub enum PositionState {
    #[default]
    None,
    Open,
    Closing(String),
    Closed(String),
}

impl fmt::Display for PositionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PositionState::None => write!(f, "None"),
            PositionState::Open => write!(f, "Open"),
            PositionState::Closing(reason) => write!(f, "Closing({})", reason),
            PositionState::Closed(reason) => write!(f, "Closed({})", reason),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Position {
    id: u32,
    fund_name: String,
    state: PositionState,
    token_name: String,
    tick_count: u32,
    actual_entry_tick: u32,
    actual_hold_tick: u32,
    max_holding_tick_count: u32,
    exit_timeout_tick_count: u32,
    open_time_str: String,
    open_timestamp: i64,
    close_time_str: String,
    average_open_price: Decimal,
    position_type: PositionType,
    target_price: Decimal,
    take_profit_price: Option<Decimal>,
    cut_loss_price: Option<Decimal>,
    close_price: Decimal,
    close_asset_in_usd: Decimal,
    amount: Decimal,
    asset_in_usd: Decimal,
    pnl: Decimal,
    fee: Decimal,
    trailing_peak_price: RefCell<Option<Decimal>>,
    // for debug
    atr: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    adx: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    rsi: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    stochastic: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    price: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
    candle_pattern: (
        CandlePattern,
        CandlePattern,
        CandlePattern,
        CandlePattern,
        CandlePattern,
        CandlePattern,
    ),
    take_profit_ratio: Decimal,
    atr_spread: Decimal,
    risk_reward: Decimal,
    atr_term: Decimal,
    tick_spread: i64,
    bias_ticks: i64,
    last_volume: Option<Decimal>,
    last_num_trades: Option<u64>,
    last_funding_rate: Option<Decimal>,
    last_open_interest: Option<Decimal>,
    last_oracle_price: Option<Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub enum OrderState {
    #[default]
    Open,
    Filled,
}

impl fmt::Display for OrderState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderState::Open => write!(f, "Open"),
            OrderState::Filled => write!(f, "Filled"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Order {
    id: String,
    unfilled_amount: Decimal,
    state: OrderState,
    tick_count: u32,
    entry_timeout_tick_count: u32,
}

enum UpdateResult {
    Closed,
    Decreased,
    Inverted,
}

pub enum OrderType {
    OpenOrder,
    CloseOrder,
}

impl Position {
    pub fn new(
        id: u32,
        fund_name: &str,
        exit_timeout_tick_count: u32,
        max_holding_tick_count: u32,
        token_name: &str,
        position_type: PositionType,
        target_price: Decimal,
        atr: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        adx: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        rsi: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        stochastic: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        price: (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal),
        candle_pattern: (
            CandlePattern,
            CandlePattern,
            CandlePattern,
            CandlePattern,
            CandlePattern,
            CandlePattern,
        ),
        take_profit_ratio: Decimal,
        atr_spread: Decimal,
        risk_reward: Decimal,
        atr_term: Decimal,
        tick_spread: i64,
        bias_ticks: i64,
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
            tick_count: 0,
            actual_entry_tick: 0,
            actual_hold_tick: 0,
            max_holding_tick_count,
            exit_timeout_tick_count,
            state: PositionState::None,
            token_name: token_name.to_owned(),
            open_time_str: String::new(),
            open_timestamp: 0,
            close_time_str: String::new(),
            average_open_price: decimal_0,
            position_type,
            target_price,
            take_profit_price: None,
            cut_loss_price: None,
            close_price: decimal_0,
            close_asset_in_usd: decimal_0,
            amount: decimal_0,
            asset_in_usd: decimal_0,
            pnl: decimal_0,
            fee: decimal_0,
            trailing_peak_price: None.into(),
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
            tick_spread,
            bias_ticks,
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
        if !matches!(
            self.state,
            PositionState::None | PositionState::Open | PositionState::Closing(_)
        ) {
            log::error!("on_filled: Invalid position state: {:?}", self);
            return Err(());
        }

        log::trace!("state = {}, amount = {}", self.state, amount);

        self.fee += fee;

        if self.state == PositionState::None {
            self.position_type = position_type.clone();
        }

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
                PositionState::Closing(reason) => reason,
                _ => {
                    log::error!("delete: Invalid PositionState: {}", self.state);
                    return Err(());
                }
            }
        };

        self.delete(close_price, &reason);

        return Ok(());
    }

    pub fn request_close(&mut self, reason: &str) -> Result<(), ()> {
        if !matches!(self.state, PositionState::Open) {
            log::error!("request_close: Invalid position state: {:?}", self);
            return Err(());
        }

        self.update_state(PositionState::Closing(reason.to_owned()));

        return Ok(());
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
        self.update_state(PositionState::Open);

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
        self.close_asset_in_usd += asset_in_usd;

        match self.update_amount_and_pnl(position_type, amount, asset_in_usd, filled_price) {
            UpdateResult::Closed => {
                let reason = if self.pnl > Decimal::ZERO {
                    "TakeProfit"
                } else {
                    "CutLoss"
                };
                self.delete(filled_price, reason);
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
            UpdateResult::Decreased => {
                log::info!(
                    "** The position is decreased: {}",
                    self.format_position(current_price)
                );
            }
        }
    }

    fn delete(&mut self, close_price: Decimal, reason: &str) {
        if let PositionState::Closing(closing_reason) = self.state.clone() {
            self.update_state(PositionState::Closed(closing_reason));
        } else {
            self.update_state(PositionState::Closed(reason.to_owned()));
        }

        self.close_price = close_price;
        self.pnl += Self::unrealized_pnl(close_price, self.amount, self.asset_in_usd);
        self.pnl -= self.fee;
        self.amount = Decimal::new(0, 0);
        self.asset_in_usd = Decimal::new(0, 0);

        log::info!(
            "-- Close the position[{}][{}]: {}, amount: {:.3}, pnl: {:.3?}",
            self.id,
            self.position_type,
            self.state,
            self.amount,
            self.pnl
        );
    }

    fn update_state(&mut self, new_state: PositionState) {
        match new_state {
            PositionState::Closing(_) => {
                self.actual_hold_tick = self.tick_count;
                self.tick_count = 0;
            }
            PositionState::Open => match self.state {
                PositionState::None => {
                    self.actual_entry_tick = self.tick_count;
                    self.tick_count = 0;
                    self.set_open_time();
                }
                PositionState::Closing(_) => {
                    return;
                }
                _ => {}
            },
            PositionState::Closed(_) => {
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
            UpdateResult::Decreased
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
            UpdateResult::Decreased => {
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

    pub fn update_counter(&mut self) {
        if matches!(self.state, PositionState::Open | PositionState::Closing(_)) {
            self.tick_count += 1;
        }
    }

    pub fn should_close(&self, close_price: Decimal, use_trailing: bool) -> Option<ReasonForClose> {
        if self.should_take_profit(close_price, use_trailing) {
            return Some(ReasonForClose::TakeProfit);
        }

        if self.should_cut_loss(close_price) {
            Some(ReasonForClose::CutLoss)
        } else {
            None
        }
    }

    pub fn pnl(&self) -> (Decimal, Decimal) {
        if self.close_asset_in_usd.is_zero() {
            (self.pnl, Decimal::ZERO)
        } else {
            (self.pnl, self.pnl / self.close_asset_in_usd.abs())
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

    pub fn target_price(&self) -> Decimal {
        self.target_price
    }

    pub fn state(&self) -> PositionState {
        self.state.clone()
    }

    pub fn token_name(&self) -> &str {
        &self.token_name
    }

    pub fn amount(&self) -> Decimal {
        self.amount
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

    pub fn candle_pattern(
        &self,
    ) -> (
        CandlePattern,
        CandlePattern,
        CandlePattern,
        CandlePattern,
        CandlePattern,
        CandlePattern,
    ) {
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

    pub fn actual_entry_tick(&self) -> u32 {
        self.actual_entry_tick
    }

    pub fn actual_hold_tick(&self) -> u32 {
        self.actual_hold_tick
    }

    pub fn tick_spread(&self) -> i64 {
        self.tick_spread
    }

    pub fn bias_ticks(&self) -> i64 {
        self.bias_ticks
    }

    pub fn should_open_expired(&self, close_price: Decimal) -> bool {
        if matches!(self.state, PositionState::Open) {
            self.tick_count > self.max_holding_tick_count
                && !self.has_reached_take_profit(close_price)
        } else {
            false
        }
    }

    pub fn take_profit_price(&self) -> Option<Decimal> {
        self.take_profit_price
    }

    pub fn cut_loss_price(&self) -> Option<Decimal> {
        self.cut_loss_price
    }

    fn is_trailing_stop_triggered(&self, close_price: Decimal) -> bool {
        let open_price = self.average_open_price;

        let Some(tp_price) = self.take_profit_price else {
            return false;
        };

        let expected_profit = match self.position_type {
            PositionType::Long => tp_price - open_price,
            PositionType::Short => open_price - tp_price,
        };

        let trailing_stop_ratio = expected_profit / open_price * Decimal::new(5, 1);

        match self.position_type {
            PositionType::Long => {
                if let Some(peak) = *self.trailing_peak_price.borrow() {
                    let stop_price = peak * (Decimal::ONE - trailing_stop_ratio);
                    return close_price <= stop_price && close_price > open_price;
                }
            }
            PositionType::Short => {
                if let Some(trough) = *self.trailing_peak_price.borrow() {
                    let stop_price = trough * (Decimal::ONE + trailing_stop_ratio);
                    return close_price >= stop_price && close_price < open_price;
                }
            }
        }

        false
    }

    pub fn should_take_profit(&self, close_price: Decimal, use_trailing: bool) -> bool {
        if !matches!(self.state, PositionState::Open) {
            return false;
        }

        let open_price = self.average_open_price;
        let mut reached_tp = false;

        if let Some(tp_price) = self.take_profit_price {
            match self.position_type {
                PositionType::Long => {
                    if close_price >= tp_price {
                        reached_tp = true;
                        let mut peak = self.trailing_peak_price.borrow_mut();
                        let current_peak = peak.get_or_insert(close_price.max(open_price));
                        if close_price > *current_peak {
                            *current_peak = close_price;
                        }
                    }
                }
                PositionType::Short => {
                    if close_price <= tp_price {
                        reached_tp = true;
                        let mut trough = self.trailing_peak_price.borrow_mut();
                        let current_trough = trough.get_or_insert(close_price.min(open_price));
                        if close_price < *current_trough {
                            *current_trough = close_price;
                        }
                    }
                }
            }
        } else {
            return false;
        }

        if !use_trailing {
            return reached_tp;
        }

        let triggered = self.is_trailing_stop_triggered(close_price);

        match self.position_type {
            PositionType::Long => {
                if let Some(peak) = *self.trailing_peak_price.borrow() {
                    let expected = self.take_profit_price.unwrap() - open_price;
                    let ratio = expected / open_price * Decimal::new(5, 1);
                    let stop = peak * (Decimal::ONE - ratio);
                    log::warn!(
                        "Trailing Stop [Long][{}]: {} - price: {:.2}, open: {:.2}, peak: {:.2}, stop: {:.2}, ratio: {:.4}",
                        self.id, triggered, close_price, open_price, peak, stop, ratio
                    );
                }
            }
            PositionType::Short => {
                if let Some(trough) = *self.trailing_peak_price.borrow() {
                    let expected = open_price - self.take_profit_price.unwrap();
                    let ratio = expected / open_price * Decimal::new(5, 1);
                    let stop = trough * (Decimal::ONE + ratio);
                    log::warn!(
                        "Trailing Stop [Short][{}]: {} - price: {:.2}, open: {:.2}, trough: {:.2}, stop: {:.2}, ratio: {:.4}",
                        self.id, triggered, close_price, open_price, trough, stop, ratio
                    );
                }
            }
        }

        triggered
    }

    fn has_reached_take_profit(&self, close_price: Decimal) -> bool {
        match self.position_type {
            PositionType::Long => {
                if let Some(tp) = self.take_profit_price {
                    if close_price >= tp {
                        return true;
                    }
                }
            }
            PositionType::Short => {
                if let Some(tp) = self.take_profit_price {
                    if close_price <= tp {
                        return true;
                    }
                }
            }
        }

        // Also consider trailing stop trigger
        self.is_trailing_stop_triggered(close_price)
    }

    fn should_cut_loss(&self, close_price: Decimal) -> bool {
        if !matches!(self.state, PositionState::Open) {
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

    pub fn should_cancel_closing(&self) -> bool {
        match self.state {
            PositionState::Closing(_) => self.tick_count > self.exit_timeout_tick_count,
            _ => false,
        }
    }

    pub fn cancel_closing(&mut self) {
        if !matches!(self.state, PositionState::Closing(_)) {
            log::warn!("cancel_closing: invalid state: {:?}", self);
        }
        self.state = PositionState::Open;
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
            "ID:{} {:<6}({}) tick: {}/{}, un-pnl: {:3.3}({:.2}%), re-pnl: {:3.3}, [{}] price: {:>6.3}/{:>6.3}({:.3}%), cut: {:>6.3}({:.3}%), take: {:>6.3}({:.3}%), amount: {:6.6}/{:6.6}",
            self.id,
            self.token_name,
            self.state,
            self.tick_count,
            if matches!(self.state, PositionState::Closing(_)) {
                self.exit_timeout_tick_count
            } else {
                self.max_holding_tick_count
            },
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
            self.asset_in_usd
        )
    }

    pub fn get_info(&self, current_price: Decimal) -> Option<String> {
        if self.amount.is_zero() {
            None
        } else {
            Some(format!("{}", self.format_position(current_price)))
        }
    }
}

impl Order {
    pub fn new(id: String, amount: Decimal, entry_timeout_tick_count: u32) -> Order {
        Self {
            id,
            unfilled_amount: amount,
            state: OrderState::Open,
            tick_count: 0,
            entry_timeout_tick_count,
        }
    }

    pub fn on_filled(&mut self, amount: Decimal) -> Result<(), ()> {
        if matches!(self.state, OrderState::Filled) {
            log::warn!(
                "The order is filled unexpectedly: id = {}, state = {}, amount = {}",
                self.id,
                self.state,
                amount
            );
            return Err(());
        }

        self.unfilled_amount -= amount;
        if self.unfilled_amount.is_zero() {
            self.state = OrderState::Filled;
        }

        log::info!(
            "Order filled: id = {}, state = {}, unfilled_amount = {}",
            self.id,
            self.state,
            self.unfilled_amount
        );

        return Ok(());
    }

    pub fn should_cancel_order(&self) -> bool {
        if matches!(self.state, OrderState::Open) {
            self.tick_count > self.entry_timeout_tick_count
        } else {
            false
        }
    }

    pub fn update_counter(&mut self) {
        if matches!(self.state, OrderState::Open) {
            self.tick_count += 1;
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn state(&self) -> OrderState {
        self.state.clone()
    }
}
