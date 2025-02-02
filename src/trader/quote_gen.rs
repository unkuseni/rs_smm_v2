use skeleton::{
    exchange::exchange::Exchange,
    utils::{
        bot::LiveBot,
        localorderbook::OrderBook,
        logger::Logger,
        models::{sort_grid, BatchOrder, BybitBook, BybitClient, BybitPrivate, LiveOrder},
        number::{geometric_weights, geomspace, nbsqrt, round_step, Round},
    },
};
use std::collections::{HashSet, VecDeque};

type Result<T> = std::result::Result<T, f64>;

// Named constants for magic numbers
const SAFETY_FACTOR: f64 = 0.95;
const VOLATILITY_MULTIPLIER: f64 = 100.0;
const MAX_SPREAD_MULTIPLIER: f64 = 3.7;
const INVENTORY_ADJUSTMENT: f64 = -0.63;

const MIN_CANCEL_LIMIT: usize = 1;
const ORDER_CHUNK_SIZE: usize = 10;

#[derive(Debug)]
pub struct QuoteGenerator {
    logger: Logger,
    client: BybitClient,
    max_position_usd: f64,
    pub position_qty: f64,
    minimum_spread: f64,
    pub adjusted_spread: f64,
    pub inventory_delta: f64,
    pub live_buys: VecDeque<LiveOrder>,
    pub live_sells: VecDeque<LiveOrder>,
    total_order: usize,
    final_order_distance: f64,
    rate_limit: usize,
    cancel_limit: usize,
    initial_limit: usize,
    bounds: f64,
    last_update_price: f64,
    time_limit: u64,
    tick_window: usize,
}

impl QuoteGenerator {
    pub async fn new(
        client: BybitClient,
        asset: f64,
        leverage: f64,
        orders_per_side: usize,
        tick_window: usize,
        rate_limit: usize,
    ) -> Self {
        let bot = LiveBot::new("./config.toml").await.unwrap();
        Self {
            logger: Logger::new(bot),
            client,
            max_position_usd: Self::max_position_usd(asset, leverage),
            position_qty: 0.0,
            minimum_spread: 0.0,
            adjusted_spread: 0.0,
            inventory_delta: 0.0,
            live_buys: VecDeque::with_capacity(ORDER_CHUNK_SIZE),
            live_sells: VecDeque::with_capacity(ORDER_CHUNK_SIZE),
            total_order: orders_per_side,
            final_order_distance: 10.0,
            rate_limit,
            initial_limit: rate_limit,
            cancel_limit: rate_limit,
            bounds: 0.0,
            time_limit: 0,
            last_update_price: 0.0,
            tick_window,
        }
    }

    fn max_position_usd(asset: f64, leverage: f64) -> f64 {
        (asset * leverage) * SAFETY_FACTOR
    }

    pub fn set_min_spread(&mut self, spread: f64) {
        self.minimum_spread = spread;
    }

    fn set_inventory_delta(&mut self, price: f64) {
        self.inventory_delta = if self.position_qty.abs() > f64::EPSILON {
            (self.position_qty * price) / self.max_position_usd
        } else {
            0.0
        };
    }

    fn calculate_vol_adjusted_value(
        &mut self,
        base_value: f64,
        book: &BybitBook,
        volatility: f64,
    ) -> f64 {
        let volatility_multiplier =
            1.0 + (volatility * VOLATILITY_MULTIPLIER * (self.tick_window as f64).sqrt());
        let min_value = base_value * volatility_multiplier;
        let max_value = min_value * MAX_SPREAD_MULTIPLIER * volatility_multiplier;
        book.get_spread().clip(min_value, max_value)
    }

    fn vol_adjusted_spread(&mut self, book: &BybitBook, volatility: f64) -> f64 {
        let mid_price = book.get_mid_price();
        let base_min_spread = bps_to_decimal(if self.minimum_spread.abs() < f64::EPSILON {
            volatility * VOLATILITY_MULTIPLIER * (self.tick_window as f64).sqrt()
        } else {
            self.minimum_spread
        }) * mid_price;

        self.adjusted_spread = self.calculate_vol_adjusted_value(base_min_spread, book, volatility);
        self.adjusted_spread
    }

    fn vol_adjusted_bounds(&mut self, book: &BybitBook, volatility: f64) -> f64 {
        let base_min_spread = bps_to_decimal(if self.minimum_spread.abs() < f64::EPSILON {
            volatility * VOLATILITY_MULTIPLIER * (self.tick_window as f64).sqrt()
        } else {
            self.minimum_spread
        }) * self.last_update_price;

        self.bounds = self.calculate_vol_adjusted_value(base_min_spread, book, volatility);
        self.bounds
    }

    fn _order_refresh_time(&self, volatility: f64) -> f64 {
        let window_seconds = self.tick_window as f64;
        if window_seconds < f64::EPSILON || volatility < f64::EPSILON {
            return f64::INFINITY;
        }

        let per_second_vol = volatility / window_seconds.sqrt();
        if per_second_vol < f64::EPSILON {
            return f64::INFINITY;
        }

        let ratio = self.adjusted_spread / (2.0 * per_second_vol);
        ratio.powf(2.0)
    }

    fn generate_quotes(
        &mut self,
        symbol: &str,
        book: &BybitBook,
        skew: f64,
        volatility: f64,
    ) -> Result<Vec<BatchOrder>> {
        let spread = self.vol_adjusted_spread(book, volatility);

        let inventory_factor = nbsqrt(self.inventory_delta)?;
        let skew_factor = skew * (1.0 - inventory_factor.abs());
        let combined_skew =
            (skew_factor + INVENTORY_ADJUSTMENT * inventory_factor).clamp(-1.0, 1.0);

        let is_positive_skew = combined_skew >= 0.0;
        let orders = self.generate_skew_orders(symbol, spread, skew.abs(), book, is_positive_skew);

        Ok(orders)
    }

    fn generate_skew_orders(
        &self,
        symbol: &str,
        spread: f64,
        skew: f64,
        book: &BybitBook,
        is_positive_skew: bool,
    ) -> Vec<BatchOrder> {
        let mid_price = book.get_mid_price();
        let notional = book.min_notional;
        // let clipped_r = skew.clamp(0.10, 0.63);
        let post_only_max = book.post_only_max;

        let (best_bid, best_ask) = if is_positive_skew {
            let bid = mid_price - (spread * (1.0 - skew.sqrt()));
            (bid, bid + spread)
        } else {
            let ask = mid_price + (spread * (1.0 - skew.sqrt()));
            (ask - spread, ask)
        };

        let end = spread * self.final_order_distance;
        let bid_prices = geomspace(best_bid - end, best_bid, self.total_order);
        let ask_prices = geomspace(best_ask, best_ask + end, self.total_order);

        let (bid_r, ask_r) = if is_positive_skew {
            // (clipped_r, 0.37)
            (0.37, 0.37)
        } else {
            // (0.37, clipped_r)
            (0.37, 0.37)
        };

        let max_buy_qty = if self.position_qty != 0.0 {
            (self.max_position_usd / 2.0) - (self.position_qty * mid_price)
        } else {
            self.max_position_usd / 2.0
        };
        let bid_sizes = if self.inventory_delta < 0.5 {
            geometric_weights(bid_r, self.total_order, false)
                .into_iter()
                .map(|w| w * max_buy_qty)
                .collect()
        } else {
            vec![]
        };

        let max_sell_qty = if self.position_qty != 0.0 {
            (self.max_position_usd / 2.0) + (self.position_qty * mid_price)
        } else {
            self.max_position_usd / 2.0
        };
        let ask_sizes = if self.inventory_delta > -0.5 {
            geometric_weights(ask_r, self.total_order, true)
                .into_iter()
                .map(|w| w * max_sell_qty)
                .collect()
        } else {
            vec![]
        };

        let mut orders = Vec::with_capacity(self.total_order * 2);
        for i in 0..self.total_order {
            if let (Some(&bid_price), Some(&bid_size)) = (bid_prices.get(i), bid_sizes.get(i)) {
                let size = (bid_size / bid_price).min(post_only_max);
                orders.push(BatchOrder::new(
                    symbol.to_string(),
                    round_price(book, bid_price),
                    round_size(size, book),
                    true,
                ));
            }

            if let (Some(&ask_price), Some(&ask_size)) = (ask_prices.get(i), ask_sizes.get(i)) {
                let size = (ask_size / ask_price).min(post_only_max);
                orders.push(BatchOrder::new(
                    symbol.to_string(),
                    round_price(book, ask_price),
                    round_size(size, book),
                    false,
                ));
            }
        }
        orders.retain(|order| (order.1 * order.2) >= notional);
        orders
    }

    async fn send_batch_orders(&mut self, orders: Vec<BatchOrder>) -> bool {
        let mut result = false;
        for chunk in orders.chunks(ORDER_CHUNK_SIZE) {
            if self.rate_limit == 0 {
                break;
            }

            if let Ok((live_buys, live_sells)) = self.client.batch_orders(chunk.to_vec()).await {
                self.live_buys.extend(live_buys);
                self.live_sells.extend(live_sells);
                self.live_buys = sort_grid(&mut self.live_buys, -1);
                self.live_sells = sort_grid(&mut self.live_sells, 1);
                self.rate_limit -= 1;
                result = true;
            } else {
                self.logger.error("Failed to send batch orders");
                self.rate_limit -= 1;
            }
        }
        result
    }

    fn check_for_fills(&mut self, info: &BybitPrivate) {
        let mut buy_indices = Vec::new();
        let mut sell_indices = Vec::new();

        for exec in &info.executions {
            let Ok(qty) = exec.exec_qty.replace(',', "").parse::<f64>() else {
                continue;
            };
            if qty <= 0.0 {
                continue;
            }

            match exec.side.as_str() {
                "Buy" => {
                    if let Some(idx) = self
                        .live_buys
                        .iter()
                        .position(|o| o.order_id == exec.order_id)
                    {
                        self.position_qty += self.live_buys[idx].qty;
                        let msg = format!(
                            "Buy fill: {:.2} @ {}",
                            self.live_buys[idx].qty, self.live_buys[idx].price
                        );
                        self.logger.info(&msg);
                        buy_indices.push(idx);
                    }
                }
                "Sell" => {
                    if let Some(idx) = self
                        .live_sells
                        .iter()
                        .position(|o| o.order_id == exec.order_id)
                    {
                        self.position_qty -= self.live_sells[idx].qty;
                        let msg = format!(
                            "Sell fill: {:.2} @ {}",
                            self.live_sells[idx].qty, self.live_sells[idx].price
                        );
                        self.logger.info(&msg);
                        sell_indices.push(idx);
                    }
                }
                _ => (),
            }
        }

        buy_indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in buy_indices {
            self.live_buys.remove(idx);
        }

        sell_indices.sort_unstable_by(|a, b| b.cmp(a));
        for idx in sell_indices {
            self.live_sells.remove(idx);
        }
    }

    async fn out_of_bounds(
        &mut self,
        book: &BybitBook,
        symbol: &str,
        private: BybitPrivate,
    ) -> bool {
        if self.live_buys.is_empty() && self.live_sells.is_empty() {
            self.last_update_price = book.mid_price;
            return true;
        }

        let bounds = self.bounds;
        let current_bid_bound = self.last_update_price - bounds;
        let current_ask_bound = self.last_update_price + bounds;

        let bounds_violated = !(current_bid_bound..=current_ask_bound).contains(&book.mid_price);
        let stale_data = (book.last_update - self.time_limit) > (self.tick_window * 1000) as u64;
        self.check_for_fills(&private);
        self.set_inventory_delta(book.get_mid_price());

        if (bounds_violated || stale_data) && self.cancel_limit > MIN_CANCEL_LIMIT {
            if let Ok(cancelled) = self.client.cancel_all(symbol).await {
                let cancelled_ids: HashSet<_> = cancelled.iter().map(|o| &o.order_id).collect();
                self.live_buys
                    .retain(|o| !cancelled_ids.contains(&o.order_id));
                self.live_sells
                    .retain(|o| !cancelled_ids.contains(&o.order_id));
                self.last_update_price = book.mid_price;
                self.cancel_limit -= 1;
                return true;
            } else {
                self.logger.error("Failed to cancel all orders");
                self.cancel_limit -= 1;
            }
        }
        false
    }

    pub async fn update_grid(
        &mut self,
        private: BybitPrivate,
        skew: f64,
        book: BybitBook,
        symbol: String,
        volatility: f64,
    ) {
        self.vol_adjusted_bounds(&book, volatility);

        if self.time_limit > 1 && (book.last_update - self.time_limit) > 1000 {
            self.rate_limit = self.initial_limit;
            self.cancel_limit = self.initial_limit;
        }

        if self.out_of_bounds(&book, &symbol, private).await {
            self.set_inventory_delta(book.get_mid_price());
            if let Ok(orders) = self.generate_quotes(&symbol, &book, skew, volatility) {
                if self.rate_limit > 1 {
                    let order_len = orders.len();

                    if self.send_batch_orders(orders).await {
                        self.logger.info(&format!(
                            "Generated {} orders for {} at {} Position: {:#?} Skew: {:#?}",
                            order_len,
                            symbol,
                            round_price(&book, book.get_mid_price()),
                            self.position_qty,
                            skew,
                        ));
                    }
                }
                self.time_limit = book.last_update;
            }
        }
    }
}

fn bps_to_decimal(bps: f64) -> f64 {
    bps * 0.0001
}

fn round_price(book: &BybitBook, price: f64) -> f64 {
    price.round_to(book.tick_size.count_decimal_places() as u8)
}

fn round_size(qty: f64, book: &BybitBook) -> f64 {
    round_step(qty, book.lot_size)
}
