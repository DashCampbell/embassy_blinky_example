#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{Config, Peripheral};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[derive(Copy, Clone)]
struct LEDColor {
    r: u8,
    g: u8,
    b: u8,
}
impl LEDColor {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        LEDColor { r, g, b }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.hsi = true;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI, // 16MHz
            prediv: PllPreDiv::DIV2,
            mul: PllMul::MUL10,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV2),
        });
    }
    let p = embassy_stm32::init(config);
    info!("Hello World!");

    // Configure the pwm pin
    let pwm_pin = PwmPin::new_ch1(p.PA8, OutputType::PushPull);
    // Obtain a PWM handler and configure the timer
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(pwm_pin),
        None,
        None,
        None,
        PWM_FREQ,
        CountingMode::EdgeAlignedUp,
    );

    // Configure Duty Cycle
    let mut ch1 = pwm.ch1();
    // Duty Cycle = Sys_Freq / PWM_Freq
    let max_duty = ch1.max_duty_cycle();
    info!("max duty cycle: {}", max_duty);
    // Enable channel 1
    ch1.enable();
    let mut dma1_ch2 = p.DMA1_CH2.into_ref();
    // settings
    const PWM_FREQ: Hertz = Hertz::khz(800); // 1 / 1.25us
    const BITS_PER_LED: usize = 32; // 8 bits per color
    // const RESET_PERIODS: usize = 40; // 40 low cycles = 1 reset signal
    // const RESET_PERIODS: usize = 55; // 40 low cycles = 1 reset signal
    const RESET_PERIODS: usize = 64; // 40 low cycles = 1 reset signal

    // let led_array: [LEDColor; LED_COUNT] = [LEDColor::new(0b11110000, 0b10101010, 0b11001100)];
    const RED: LEDColor = LEDColor::new(255, 0, 0);
    const GREEN: LEDColor = LEDColor::new(0, 255, 0);
    const BLUE: LEDColor = LEDColor::new(0, 0, 255);
    const MAGENTA: LEDColor = LEDColor::new(255, 0, 255);
    const CYAN: LEDColor = LEDColor::new(0, 255, 255);
    const YELLOW: LEDColor = LEDColor::new(255, 255, 0);
    const ORANGE: LEDColor = LEDColor::new(255, 20, 0);
    const WHITE: LEDColor = LEDColor::new(255, 255, 255);

    const LED_COUNT: usize = 14 * 4 + 2;
    let led_array: [LEDColor; LED_COUNT] = [
        MAGENTA, MAGENTA, BLUE, BLUE, CYAN, CYAN, GREEN, GREEN, YELLOW, YELLOW, ORANGE, ORANGE,
        RED, RED, MAGENTA, MAGENTA, BLUE, BLUE, CYAN, CYAN, GREEN, GREEN, YELLOW, YELLOW, ORANGE,
        ORANGE, RED, RED, MAGENTA, MAGENTA, BLUE, BLUE, CYAN, CYAN, GREEN, GREEN, YELLOW, YELLOW,
        ORANGE, ORANGE, RED, RED, MAGENTA, MAGENTA, BLUE, BLUE, CYAN, CYAN, GREEN, GREEN, YELLOW,
        YELLOW, ORANGE, ORANGE, RED, RED, WHITE, WHITE,
    ];
    const DMA_BUFFER_LEN: usize = (BITS_PER_LED * LED_COUNT) + RESET_PERIODS;

    let mut dma_buffer: [u16; DMA_BUFFER_LEN] = [0; DMA_BUFFER_LEN];
    info!("led count: {}", LED_COUNT);
    info!("bits per led: {}", BITS_PER_LED);
    info!("dma buffer length: {}", DMA_BUFFER_LEN);
    let mut count = 0usize;
    set_dma_buffer(&mut dma_buffer, &led_array);
    // set_dma_buffer_with_index(&mut dma_buffer, &led_array, count);
    info!("dma buffer: {}", dma_buffer);
    loop {
        // set_dma_buffer(&mut dma_buffer, &led_array);
        // debug!("count: {}", count);
        set_dma_buffer_with_index(&mut dma_buffer, &led_array, count);
        pwm.waveform_ch1(&mut dma1_ch2, &dma_buffer).await;
        count += 1;
        Timer::after_millis(1000).await;
    }
}
fn set_dma_buffer(dma_buffer: &mut [u16], led_array: &[LEDColor]) {
    for (mut led_index, led) in led_array.iter().enumerate() {
        led_index *= 32;
        set_byte(led.g, dma_buffer, led_index);
        set_byte(led.r, dma_buffer, led_index + 8);
        set_byte(led.b, dma_buffer, led_index + 16);
        set_byte(0, dma_buffer, led_index + 24);
    }
}
fn set_dma_buffer_with_index(dma_buffer: &mut [u16], led_array: &[LEDColor], start: usize) {
    for i in 0..led_array.len() {
        let led_index = (i + start) % led_array.len();
        debug!("led index: {}", led_index);
        let led = &led_array[led_index];
        let byte_index = led_index * 32;
        set_byte(led.g, dma_buffer, byte_index);
        set_byte(led.r, dma_buffer, byte_index + 8);
        set_byte(led.b, dma_buffer, byte_index + 16);
        set_byte(0, dma_buffer, byte_index + 24);
    }
}
fn set_byte(color: u8, buffer: &mut [u16], start: usize) {
    // 0.4 /1.25 = 0.32
    // 0.8 / 1.25 = 0.64
    const MAX_TICK: u16 = 50;
    const LOW_VAL: u16 = 12;
    const HIGH_VAL: u16 = 24;
    // const LOW_VAL: u16 = (0.32f64 * MAX_TICK as f64) as u16;
    // const HIGH_VAL: u16 = (0.64f64 * MAX_TICK as f64) as u16;
    for i in 0..8 {
        buffer[i + start] = if (color & (1 << (7 - i))) > 0 {
            HIGH_VAL
        } else {
            LOW_VAL
        };
    }
}
