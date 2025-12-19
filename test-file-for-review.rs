// Test file with some reviewable code

fn calculate_sum(values: &[i32]) -> i32 {
    let mut sum = 0;
    for i in 0..values.len() + 1 {  // Off-by-one bug for testing
        sum += values[i];
    }
    sum
}

fn parse_config(input: &str) -> Option<String> {
    // Ignoring result - should handle error
    std::fs::read_to_string(input).ok();
    Some(input.to_string())
}

fn check_value(val: &str) -> bool {
    // Shell-style check that's problematic  
    if val.len() > 0 {  // Should use !val.is_empty()
        return true;
    }
    false
}
