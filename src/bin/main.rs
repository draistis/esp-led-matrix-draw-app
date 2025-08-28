#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Level, OutputConfig};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, gpio::Output};
use esp_led_matrix_draw_app::led_matrix::update_matrix;
use esp_led_matrix_draw_app::web_server::web_server_task;
use esp_led_matrix_draw_app::wifi;
use {esp_backtrace as _, esp_println as _};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let rng = Rng::new(peripherals.RNG);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer00 = TimerGroup::new(peripherals.TIMG0).timer0;
    esp_hal_embassy::init(timer00);

    info!("Embassy initialized!");

    let rows: [Output; 8] = [
        Output::new(peripherals.GPIO15, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO17, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO18, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO19, Level::Low, OutputConfig::default()),
    ];
    let cols: [Output; 8] = [
        Output::new(peripherals.GPIO13, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO12, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO14, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO27, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO26, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO25, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO33, Level::High, OutputConfig::default()),
        Output::new(peripherals.GPIO32, Level::High, OutputConfig::default()),
    ];

    let timer10 = TimerGroup::new(peripherals.TIMG1).timer0;
    let wifi_stack = wifi::init_wifi(rng, timer10, peripherals.WIFI, &spawner).await;

    spawner.spawn(web_server_task(wifi_stack)).unwrap();
    spawner.spawn(update_matrix(rows, cols)).unwrap();

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(60)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}
