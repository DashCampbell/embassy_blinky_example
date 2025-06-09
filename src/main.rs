#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::{Config, Peripheral};
use {defmt_rtt as _, panic_probe as _};

struct LEDColor {
    r: u8,
    g: u8,
    b: u8,
}
impl LEDColor {
    fn new(r: u8, g: u8, b: u8) -> Self {
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
    // settings
    const PWM_FREQ: Hertz = Hertz::khz(800); // 1 / 1.25us
    const BITS_PER_LED: usize = 24; // 8 bits per color
    const LED_COUNT: usize = 1;
    // const RESET_PERIODS: usize = 40; // 40 low cycles = 1 reset signal
    const RESET_PERIODS: usize = 1; // 40 low cycles = 1 reset signal
    const DMA_BUFFER_LEN: usize = (BITS_PER_LED * LED_COUNT) + RESET_PERIODS;

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

    let led_array: [LEDColor; LED_COUNT] = [LEDColor::new(0b11110000, 0b10101010, 0b11001100)];
    let mut dma_buffer: [u16; DMA_BUFFER_LEN] = [0; DMA_BUFFER_LEN];
    info!("dma buffer length: {}", DMA_BUFFER_LEN);
    set_dma_buffer(&mut dma_buffer, &led_array);
    info!("dma buffer: {}", dma_buffer);
    loop {
        set_dma_buffer(&mut dma_buffer, &led_array);
        pwm.waveform_ch1(&mut dma1_ch2, &dma_buffer).await;
    }
}
fn set_dma_buffer(dma_buffer: &mut [u16], led_array: &[LEDColor]) {
    for led in led_array {
        set_byte(led.g, dma_buffer, 0);
        set_byte(led.r, dma_buffer, 8);
        set_byte(led.b, dma_buffer, 16);
    }
}
fn set_byte(color: u8, buffer: &mut [u16], start: usize) {
    // 0.4 /1.25 = 0.32
    // 0.8 / 1.25 = 0.64
    const MAX_TICK: u16 = 50;
    const LOW_VAL: u16 = (0.32f64 * MAX_TICK as f64) as u16;
    const HIGH_VAL: u16 = (0.64f64 * MAX_TICK as f64) as u16;
    for i in 0..8 {
        buffer[i + start] = if (color & (1 << (7 - i))) > 0 {
            HIGH_VAL
        } else {
            LOW_VAL
        };
    }
}
