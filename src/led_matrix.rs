use core::sync::atomic::Ordering;
use embassy_time::Timer;
use esp_hal::gpio::Output;
use portable_atomic::AtomicU64;

/// Stores the state of our 8x8 LED matrix
pub static MATRIX: AtomicU64 = AtomicU64::new(0);

/// Helper function to select the bit we want to access
fn bit(x: u8, y: u8) -> u64 {
    1_u64 << y * 8 + x
}

/// Function returns the state (ON/OFF) of the LED at coordinates (x, y) in the virtual matrix
pub fn get(x: u8, y: u8) -> bool {
    MATRIX.load(Ordering::Relaxed) & bit(x, y) != 0
}

/// Function sets the state of LED to `on` at coordinates (x, y) in the virtual matrix
pub fn set(x: u8, y: u8, on: bool) {
    let b = bit(x, y);
    MATRIX
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |bits| {
            Some(if on { bits | b } else { bits & !b })
        })
        .ok();
}

/// Function returns state of virtual matrix
pub fn snapshot() -> u64 {
    MATRIX.load(Ordering::Relaxed)
}

/// Function updates our physical LED matrix based on the state of `MATRIX`
#[embassy_executor::task]
pub async fn update_matrix(mut rows: [Output<'static>; 8], mut cols: [Output<'static>; 8]) {
    loop {
        for (y, row) in rows.iter_mut().enumerate() {
            for col in cols.iter_mut() {
                col.set_high();
            }
            row.set_high();
            for (x, col) in cols.iter_mut().enumerate() {
                if get(x as u8, y as u8) {
                    col.set_low();
                }
            }
            Timer::after_millis(1).await;
            row.set_low();
        }
    }
}
