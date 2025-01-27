#[cfg(test)]
mod tests {
    use skeleton::utils::number::{
        decay, geometric_weights, geomspace, linspace, nbsqrt, round_step, Round,
    };

    #[test]
    fn test_decay() {
        let value = 10.0;
        let rate = Some(0.5);
        let result = decay(value, rate);
        assert_eq!(result, 0.006737946999085467);
    }
    #[test]
    fn test_geometric_weights() {
        let (ratio, size, reverse) = (0.3, 5, false);
        let buy_weights = geometric_weights(ratio, size, reverse);
        let sell_weights = geometric_weights(ratio, size, true);
        println!("Buy Weights: {:?}", buy_weights);
        println!("Sell Weights: {:?}", sell_weights);
    }
    #[test]
    fn test_geomspace() {
        let (start, end, size) = (0.5, 0.76, 5);
        let result = geomspace(start, end, size);
        println!("{:?}", result);
        println!("{:?}", result);
    }

    #[test]
    fn test_geomspace_geom_weights() {
        let (start, end, size) = (0.5, 0.76, 4);
        let (ratio, wei_size, reverse) = (0.37, 4, true);
        let result = geomspace(start, end, size);
        let buy_weights = geometric_weights(ratio, wei_size, false);
        let sell_weights = geometric_weights(ratio, wei_size, reverse);
        println!("Buy Weights: {:#?} Prices: {:#?} Sell Weights: {:#?}", buy_weights, result, sell_weights);
    }
    #[test]
    fn test_linspace() {
        let (start, end, size) = (1.0, 10.0, 5);
        let result = linspace(start, end, size);
        assert_eq!(result, vec![1.0, 3.25, 5.5, 7.75, 10.0]);
    }
    #[test]
    fn test_nbsqrt() {
        let value = 25.0;
        let result = nbsqrt(value).unwrap();
        assert_eq!(result, 5.0);
    }
    #[test]
    fn test_round_step() {
        let (value, step) = (6.05050, 0.5000);
        let result = round_step(value, step);
        assert_eq!(result, 6.0);
    }

    #[test]
    fn test_round() {
        let value = 35.463245660;
        assert_eq!(value.round_to(3), 35.463);
        assert_eq!(value.clip(0.0, 100.0), 35.46324566);
        assert_eq!(value.count_decimal_places(), 8);
    }
}
