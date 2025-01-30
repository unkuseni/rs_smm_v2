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
    let (total_volume, buy_volume) = trades.iter().fold((0.0, 0.0), |(total, buy), trade| {
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

#[inline(always)]
pub fn avg_trade_price(
    mid_price: f64,
    old_trades: Option<&TradeType>,
    curr_trades: &TradeType,
    prev_avg: f64,
) -> f64 {
    // If no old_trades, compute VWAP of curr_trades directly
    let Some(old_trades) = old_trades else {
        return compute_vwap(curr_trades).unwrap_or(mid_price);
    };

    let (old_volume, old_turnover) = old_trades.iter().fold((0.0, 0.0), |(vol, turn), trade| {
        (vol + trade.volume, turn + trade.volume * trade.price)
    });

    let (curr_volume, curr_turnover) = curr_trades.iter().fold((0.0, 0.0), |(vol, turn), trade| {
        (vol + trade.volume, turn + trade.volume * trade.price)
    });

    if old_volume != curr_volume {
        (curr_turnover - old_turnover) / (curr_volume - old_volume)
    } else {
        prev_avg
    }
}

/// Helper to compute VWAP when old_trades is None
fn compute_vwap(trades: &TradeType) -> Option<f64> {
    let (volume, turnover) = trades.iter().fold((0.0, 0.0), |(vol, turn), trade| {
        (vol + trade.volume, turn + trade.volume * trade.price)
    });
    if volume == 0.0 {
        None
    } else {
        Some(turnover / volume)
    }
}
