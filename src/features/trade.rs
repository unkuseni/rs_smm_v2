use skeleton::exchange::exchange::TradeType;

/// Calculate the trade imbalance for a given TradeType.
///
/// # Returns
/// A float representing the trade imbalance. A value of 1.0 means all trades were buys, and -1.0 means all trades were sells.
pub fn trade_imbalance(trades: &TradeType) -> f64 {
    // Calculate total volume and buy volume
    let (total_volume, buy_volume) = calculate_volumes(trades);
    // Handle empty trade history (optional)
    if total_volume == 0.0 {
        // You can either return an empty tuple or a specific value to indicate no trades
        return 0.0;
    }
    // Calculate buy-sell ratio (avoid division by zero)
    let ratio = buy_volume / total_volume;
    2.0 * ratio - 1.0
}

/// Given a TradeType, this function calculates the total volume and buy volume.
/// It supports both Bybit and Binance formats.
///
/// # Arguments
///
/// * `trades`: The TradeType to calculate the volumes from
///
/// # Returns
///
/// A tuple of two f64s, the first one being the total volume and the second one being the buy volume
fn calculate_volumes(trades: &TradeType) -> (f64, f64) {
    match trades {
        TradeType::Bybit(trades) => {
            let (total_volume, buy_volume) =
                trades.iter().fold((0.0, 0.0), |(total, buy), trade| {
                    let new_total = total + trade.volume;
                    let new_buy = if trade.side == "Buy" {
                        buy + trade.volume
                    } else {
                        buy
                    };
                    (new_total, new_buy)
                });
            (total_volume, buy_volume)
        }
        TradeType::Binance(trades) => {
            let (total_volume, buy_volume) =
                trades.iter().fold((0.0, 0.0), |(total, buy), trade| {
                    let new_total = total + trade.qty.parse::<f64>().unwrap();
                    let new_buy = if trade.is_buyer_maker {
                        buy
                    } else {
                        buy + trade.qty.parse::<f64>().unwrap()
                    };
                    (new_total, new_buy)
                });
            (total_volume, buy_volume)
        }
    }
}

#[inline(always)]
pub fn avg_trade_price(
    mid_price: f64,
    old_trades: Option<&TradeType>,
    curr_trades: &TradeType,
    prev_avg: f64,
    tick_window: usize,
) -> f64 {
    let Some(old_trades) = old_trades else {
        return mid_price;
    };

    // Precompute inverse tick
    let inv_tick = 1.0 / tick_window as f64;

    match (old_trades, curr_trades) {
        (TradeType::Bybit(old_trades), TradeType::Bybit(curr_trades)) => {
            // Calculate volumes and turnovers in one pass each
            let (old_volume, old_turnover) =
                old_trades.iter().fold((0.0, 0.0), |(vol, turn), trade| {
                    (vol + trade.volume, turn + trade.volume * trade.price)
                });

            let (curr_volume, curr_turnover) =
                curr_trades.iter().fold((0.0, 0.0), |(vol, turn), trade| {
                    (vol + trade.volume, turn + trade.volume * trade.price)
                });

            if old_volume != curr_volume {
                ((curr_turnover - old_turnover) / (curr_volume - old_volume)) * inv_tick
            } else {
                prev_avg
            }
        }
        (TradeType::Binance(old_trades), TradeType::Binance(curr_trades)) => {
            // Pre-parse all values to avoid repeated parsing
            let (old_volume, old_turnover) = old_trades
                .iter()
                .filter_map(|trade| {
                    let qty = trade.qty.parse::<f64>().ok()?;
                    let price = trade.price.parse::<f64>().ok()?;
                    Some((qty, qty * price))
                })
                .fold((0.0, 0.0), |(vol, turn), (qty, value)| {
                    (vol + qty, turn + value)
                });

            let (curr_volume, curr_turnover) = curr_trades
                .iter()
                .filter_map(|trade| {
                    let qty = trade.qty.parse::<f64>().ok()?;
                    let price = trade.price.parse::<f64>().ok()?;
                    Some((qty, qty * price))
                })
                .fold((0.0, 0.0), |(vol, turn), (qty, value)| {
                    (vol + qty, turn + value)
                });

            if old_volume != curr_volume {
                ((curr_turnover - old_turnover) / (curr_volume - old_volume)) * inv_tick
            } else {
                prev_avg
            }
        }
        _ => prev_avg,
    }
}
