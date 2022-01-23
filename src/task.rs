use crate::tracker::TrackedData;
use rand::Rng;

// random_value_generator generates random values.
pub fn random_value_generator() -> Result<TrackedData, String> {
    let mut rng = rand::thread_rng();
    let mut vec = vec![];

    for _ in 1..10 {
        vec.push(rng.gen_range(0..10).to_string());
    }
    Ok(vec)
}
