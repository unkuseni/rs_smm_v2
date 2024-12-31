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
        let weights = geometric_weights(ratio, size, reverse);
        assert_eq!(
            weights,
            vec![
                0.7017051434987018,
                0.21051154304961053,
                0.06315346291488316,
                0.01894603887446495,
                0.005683811662339485
            ]
        );
    }
    #[test]
    fn test_geomspace() {
        let (start, end, size) = (1.0, 10.0, 5);
        let result = geomspace(start, end, size);
        assert_eq!(result, vec![1.0, 1.778279410038923, 3.1622776601683795, 5.623413251903492, 10.0]);
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
        assert_eq!(value.clip(0.0, 100.0),  35.46324566);
        assert_eq!(value.count_decimal_places(), 8);
    }
}
