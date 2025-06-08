#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join;
use embassy_stm32::adc::{Adc, AdcChannel, AnyAdcChannel, Resolution};
use embassy_stm32::gpio::{AnyPin, Level, Output, Pin, Speed};
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::{Config, adc};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    {
        // Enable clock for ADC
        use embassy_stm32::rcc::*;
        config.rcc.mux.adcsel = mux::Adcsel::SYS;
    }
    let p = embassy_stm32::init(config);
    info!("Hello World!");
    let joystick_task = read_joystick_better(p.ADC1, p.PA0, p.PA1);
    let blink_led_task = blink_led_better(p.PB3.degrade());

    // NOTE: Prefer using join over spawner and tasks, tasks do not accept generic arguments
    join::join(joystick_task, blink_led_task).await;
    // spawner.spawn(blink_led(p.PB3.degrade())).unwrap();
    // spawner
    //     .spawn(read_joystick(
    //         p.ADC1,
    //         p.PA0.degrade_adc(),
    //         p.PA1.degrade_adc(),
    //     ))
    //     .unwrap();
}

#[embassy_executor::task]
async fn read_joystick(adc: ADC1, mut ch1: AnyAdcChannel<ADC1>, mut ch2: AnyAdcChannel<ADC1>) {
    let mut adc = Adc::new(adc);
    adc.set_resolution(Resolution::BITS12);

    loop {
        let x = adc.blocking_read(&mut ch1);
        let y = adc.blocking_read(&mut ch2);
        info!("(x, y): ({}, {})", x, y);
        Timer::after_millis(500).await;
    }
}
// #[embassy_executor::task]
async fn read_joystick_better<T: adc::Instance>(
    adc: T,
    mut ch1: impl AdcChannel<T>,
    mut ch2: impl AdcChannel<T>,
) {
    let mut adc = Adc::new(adc);
    adc.set_resolution(Resolution::BITS12);

    loop {
        let x = adc.blocking_read(&mut ch1);
        let y = adc.blocking_read(&mut ch2);
        info!("(x, y): ({}, {})", x, y);
        Timer::after_millis(500).await;
    }
}
#[embassy_executor::task]
async fn blink_led(pin: AnyPin) {
    let mut led = Output::new(pin, Level::High, Speed::Low);

    loop {
        info!("Blink");
        led.set_high();
        Timer::after_millis(500).await;
        led.set_low();
        Timer::after_millis(500).await;
    }
}
// #[embassy_executor::task]
async fn blink_led_better(pin: AnyPin) {
    let mut led = Output::new(pin, Level::High, Speed::Low);
    const DELAY: u64 = 300;
    loop {
        led.set_high();
        Timer::after_millis(DELAY).await;
        led.set_low();
        Timer::after_millis(DELAY).await;
    }
}
