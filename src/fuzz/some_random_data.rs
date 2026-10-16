use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn fill_with_random_data(seed_input: &[u8], output_buffer: &mut [u8]) {
    let mut seed = [0u8; 32];
    let bytes_to_copy = seed_input.len().min(32);
    seed[..bytes_to_copy].copy_from_slice(&seed_input[..bytes_to_copy]);

    // 2. Initialize the seeded RNG engine
    let mut rng = ChaCha8Rng::from_seed(seed);

    // 3. Fill your arbitrary-length byte slice with pseudo-random data
    rng.fill_bytes(output_buffer);
}

pub(super) fn some_random_data() -> Vec<u8> {
    let seed = b"This a seed for the RNG, it is something non-critical for security";
    let mut random_bytes = vec![0u8; usize::pow(2, 16)]; // Can be any length
    fill_with_random_data(seed, &mut random_bytes);
    random_bytes.to_vec()
}
