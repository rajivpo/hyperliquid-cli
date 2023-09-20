use colored::*;

pub fn format_decimal(s: &str, decimals: usize, color: Color, is_price: bool) -> ColoredString {
    let f: f64 = s.parse().unwrap();
    let formatted = if is_price {
        if f.abs() >= 1.0 {
            let as_string = format!("{:.5}", f);
            let without_trailing_zeros = as_string.trim_end_matches('0');
            if without_trailing_zeros.ends_with('.') {
                without_trailing_zeros[..without_trailing_zeros.len()-1].to_string()
            } else {
                without_trailing_zeros.to_string()
            }
        } else {
            format!("{:.6}", f)
        }
    } else {
        format!("{:.*}", decimals, f)
    };
    formatted.color(color)
}
