use super::impact::mid_price_avg;


/// Calculates the basis of the average trade price relative to the mid price.
///
/// The basis is the difference between the average trade price and the mid price.
///
/// # Arguments
///
/// * `old_price` - The old price.
/// * `curr_price` - The current price.
/// * `avg_trade_price` - The average trade price.
///
/// # Returns
///
/// The basis of the average trade price relative to the mid price.
pub fn mid_price_basis(old_price: f64, curr_price: f64, avg_trade_price: f64) -> f64 {
    // Calculate the basis of the average trade price relative to the mid price.
    // The basis is the difference between the average trade price and the mid price.
    avg_trade_price - mid_price_avg(old_price, curr_price)
}
