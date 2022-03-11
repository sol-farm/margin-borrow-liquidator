pub fn generate_random_number(min: i64, max: i64) -> i64 {
    use rand::prelude::*;
    let mut rng = rand::thread_rng();
    rng.gen_range(min, max)
}
