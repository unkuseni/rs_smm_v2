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
