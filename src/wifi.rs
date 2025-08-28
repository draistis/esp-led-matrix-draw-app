use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Ipv4Cidr, Runner, StackResources};
use embassy_time::Timer;
use esp_hal::rng::Rng;
use esp_hal::timer::timg;
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiState};

const AP_SSID: &str = "ESP32-AP";
const AP_IP: Ipv4Cidr = Ipv4Cidr::new(Ipv4Address::new(192, 168, 4, 1), 24);
const AP_GATEWAY: Ipv4Address = Ipv4Address::new(192, 168, 4, 1);
const AP_PASSWORD: &str = "bigchungus";
static STACK_RESOURCES: static_cell::ConstStaticCell<StackResources<3>> =
    static_cell::ConstStaticCell::new(StackResources::<3>::new());

pub async fn init_wifi(
    mut rng: Rng,
    timer: timg::Timer<'static>,
    peripherals_wifi: esp_hal::peripherals::WIFI<'static>,
    spawner: &Spawner,
) -> embassy_net::Stack<'static> {
    let wifi_init = esp_wifi::init(timer, rng).expect("Failed to initialize WIFI/BLE controller");
    let wifi_init: &'static mut esp_wifi::EspWifiController<'_> =
        static_cell::make_static!(wifi_init);

    let (wifi_controller, interfaces) = esp_wifi::wifi::new(wifi_init, peripherals_wifi)
        .expect("Failed to initialize WIFI controller");

    let device = interfaces.ap;
    let ap_seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    let ap_config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: AP_IP,
        gateway: Some(AP_GATEWAY),
        dns_servers: Default::default(),
    });

    let (stack, runner) = embassy_net::new(device, ap_config, STACK_RESOURCES.take(), ap_seed);

    spawner.spawn(net_task(runner)).unwrap();
    spawner.spawn(connection_task(wifi_controller)).unwrap();

    stack
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    loop {
        if esp_wifi::wifi::wifi_state() == WifiState::ApStarted {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after_millis(5000).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = esp_wifi::wifi::Configuration::AccessPoint(
                esp_wifi::wifi::AccessPointConfiguration {
                    ssid: AP_SSID.into(),
                    password: AP_PASSWORD.into(),
                    auth_method: esp_wifi::wifi::AuthMethod::WPA2Personal,
                    ..Default::default()
                },
            );
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap(); // Attempt to free null pointer
            info!("Wifi started!");
        }
    }
}
